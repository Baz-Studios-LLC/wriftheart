//! people_tab.rs — the PEOPLE codex tab (js updatePeopleDex/drawPeopleDex): everyone
//! you've actually spoken to, grouped under fold-able place banners (warmest first),
//! wanderers at the bottom. The right pane is the standard dex pane: portrait, gender
//! mark, tier, the ten-heart row, when you last spoke, and their tastes/birthday once
//! you've earned them.

use super::{dex, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::talk::{PeopleLedger, PersonRec, HEART_GRID};
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::people;
use crate::ui::{border_strips, label};
use crate::CANVAS_W;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

const WANDERERS: &str = "THE WANDERING FOLK";
const VIS: usize = 8;
const RH: f32 = 21.0;

/// Roster cursor + folded places (session-local, like the js `peopleCollapsed`).
#[derive(Resource, Default)]
pub struct PeopleDex {
    pub cur: usize,
    pub collapsed: HashSet<String>,
}

#[derive(Component, Clone)]
pub struct PeopleUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = if pad { "DPAD BROWSE" } else { "ARROWS BROWSE" };
    super::hint_scaffold(bindings, pad, browse)
}

/// One roster line: a place banner or a person under it (js peopleRows).
enum Row {
    Place { name: String, n: usize, folded: bool },
    Person(PersonRec),
}

fn rows(ledger: &PeopleLedger, collapsed: &HashSet<String>) -> Vec<Row> {
    let mut groups: HashMap<String, Vec<&PersonRec>> = HashMap::default();
    for r in ledger.0.values().filter(|r| !r.name.is_empty()) {
        groups.entry(r.town.clone().unwrap_or_else(|| WANDERERS.to_string())).or_default().push(r);
    }
    let mut places: Vec<String> = groups.keys().cloned().collect();
    places.sort_by(|a, b| (a == WANDERERS).cmp(&(b == WANDERERS)).then_with(|| a.cmp(b)));
    let mut out = Vec::new();
    for place in places {
        let mut list = groups.remove(&place).unwrap_or_default();
        list.sort_by(|a, b| b.pts.cmp(&a.pts).then_with(|| a.name.cmp(&b.name)));
        let folded = collapsed.contains(&place);
        out.push(Row::Place { name: place, n: list.len(), folded });
        if !folded {
            out.extend(list.into_iter().map(|r| Row::Person(r.clone())));
        }
    }
    out
}

/// Their face — the villager's down-facing frame, cached per identity seed.
#[derive(Default)]
pub struct Portraits(HashMap<u32, Handle<Image>>);

fn portrait(cache: &mut Portraits, seed: u32, images: &mut Assets<Image>) -> Handle<Image> {
    cache
        .0
        .entry(seed)
        .or_insert_with(|| {
            crate::actors::hero::build_frames(&crate::actors::hero::random_look(seed), images).frames[0][0].clone()
        })
        .clone()
}

/// One pixel heart, `frac` part-filled left-to-right (js drawHeartPx as a bake).
fn heart_image(frac: f32, bright: bool, images: &mut Assets<Image>) -> Handle<Image> {
    let fill_cols = if frac <= 0.0 { 0 } else { ((5.0 * frac.min(1.0)).round() as usize).max(1) };
    let grid: Vec<String> = HEART_GRID
        .iter()
        .map(|row| {
            row.chars()
                .enumerate()
                .map(|(x, ch)| if ch == 'H' && x < fill_cols { 'F' } else { ch })
                .collect()
        })
        .collect();
    let rows: Vec<&str> = grid.iter().map(|s| s.as_str()).collect();
    let fill = if bright { 0xfc5878 } else { 0xc04060 };
    images.add(bake(&rows, &[('H', 0x3a2a34), ('F', fill)]))
}

/// The tiny Venus/Mars marks (the pixel font has no such glyphs; js draws them in vector).
const F_MARK: &[&str] = &[".###.", "#...#", "#...#", ".###.", "..#..", ".###.", "..#.."];
const M_MARK: &[&str] = &["..###", "....#", "..#.#", ".##..", "#..#.", "#..#.", ".##.."];

/// js TIER_OF.
fn tier_of(h: i32) -> (&'static str, u32) {
    match h {
        _ if h >= 7 => ("CONFIDANT", 0xffd34d),
        _ if h >= 3 => ("FRIEND", 0x7ee08a),
        _ if h >= 1 => ("ACQUAINTANCE", 0xb8c4d8),
        _ => ("STRANGER", 0x7a7a82),
    }
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    ptr: Res<crate::input::Pointer>,
    cx_state: Res<CodexState>,
    ledger: Res<PeopleLedger>,
    clock: Res<crate::app::room_render::FrameClock>,
    mut px: ResMut<PeopleDex>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<PeopleUi>>,
    mut cache: Local<Portraits>,
    mut seen_gen: Local<u32>,
) {
    let mut dirty = *seen_gen != cx_state.generation;
    *seen_gen = cx_state.generation;
    let list = rows(&ledger, &px.collapsed);
    if !list.is_empty() {
        px.cur = px.cur.min(list.len() - 1);
        if state.pressed(Action::Up) {
            px.cur = (px.cur + list.len() - 1) % list.len();
            dirty = true;
        }
        if state.pressed(Action::Down) {
            px.cur = (px.cur + 1) % list.len();
            dirty = true;
        }
        if ptr.wheel_steps != 0 {
            // Wheel walks the roster, clamped (Baz: any scrollable list).
            px.cur = (px.cur as i32 - ptr.wheel_steps).clamp(0, list.len() as i32 - 1) as usize;
            dirty = true;
        }
        if state.pressed(Action::Slot1)
            && let Some(Row::Place { name, .. }) = list.get(px.cur)
        {
            // Fold/unfold the place.
            if !px.collapsed.remove(name) {
                px.collapsed.insert(name.clone());
            }
            dirty = true;
        }
    }
    if dirty {
        redraw(&mut commands, &mut images, &ledger, &px, clock.0, &mut cache, &old);
    }
}

fn redraw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    ledger: &PeopleLedger,
    px: &PeopleDex,
    clock: i64,
    cache: &mut Portraits,
    old: &Query<Entity, With<PeopleUi>>,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, PeopleUi);
    let list = rows(ledger, &px.collapsed);
    let met: usize = list
        .iter()
        .map(|r| if let Row::Place { n, .. } = r { *n } else { 0 })
        .sum();
    label(commands, images, &format!("PEOPLE  {met} MET"), 8.0, 15.0, 0xbfb9a0, CONTENT_Z + 0.1, tag());
    if list.is_empty() {
        dex::center_label(commands, images, "NO ONE MET YET", CANVAS_W as f32 / 2.0, 96.0, 0x8a8a92, CONTENT_Z + 0.1, tag());
        dex::center_label(commands, images, "GO SAY HELLO IN TOWN", CANVAS_W as f32 / 2.0, 108.0, 0x5a5a62, CONTENT_Z + 0.1, tag());
        return;
    }
    let cur = px.cur.min(list.len() - 1);
    let fill = |c: &mut Commands, x: f32, y: f32, w: f32, h: f32, col: Color, z: f32| {
        c.spawn((Sprite::from_color(col, Vec2::new(w, h)), at(x, y, w, h, z), PIXEL_LAYER, (CodexUi, TabContent, PeopleUi)));
    };

    // --- Left: the roster, grouped under fold-able place banners (js layout). ---
    let (y0, lw) = (dex::DEX_GY + 2.0, dex::DEX_RX - 14.0);
    let scroll = cur.saturating_sub(3).min(list.len().saturating_sub(VIS));
    for (i, row) in list.iter().skip(scroll).take(VIS).enumerate() {
        let y = y0 + i as f32 * RH;
        let on = scroll + i == cur;
        match row {
            Row::Place { name, n, folded } => {
                let bg = if on { Color::srgba(0.988, 0.878, 0.659, 0.13) } else { Color::srgba(0.91, 0.784, 0.376, 0.06) };
                fill(commands, 6.0, y, lw, RH - 2.0, bg, CONTENT_Z);
                fill(commands, 6.0, y, 2.0, RH - 2.0, Color::srgb_u8(0xe8, 0xc8, 0x60), CONTENT_Z + 0.02);
                if on {
                    for (sx, sy, sw, sh) in border_strips(6.0, y, lw, RH - 2.0, 1.0) {
                        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0xe0, 0xa8), CONTENT_Z + 0.03);
                    }
                }
                label(commands, images, name, 14.0, y + 6.0, if on { 0xfce0a8 } else { 0xe8c860 }, CONTENT_Z + 0.04, tag());
                let t = format!("{n} {}", if *folded { "+" } else { "-" });
                let tw = font::measure(&t) as f32;
                label(commands, images, &t, 6.0 + lw - 8.0 - tw, y + 6.0, if on { 0xfce0a8 } else { 0x8a8a72 }, CONTENT_Z + 0.04, tag());
            }
            Row::Person(r) => {
                let h = people::hearts(r.pts);
                let bg = if on {
                    Color::srgba(0.988, 0.878, 0.659, 0.13)
                } else if i % 2 == 1 {
                    Color::srgba(1.0, 1.0, 1.0, 0.03)
                } else {
                    Color::srgba(0.0, 0.0, 0.0, 0.25)
                };
                fill(commands, 12.0, y, lw - 6.0, RH - 2.0, bg, CONTENT_Z);
                if on {
                    for (sx, sy, sw, sh) in border_strips(12.0, y, lw - 6.0, RH - 2.0, 1.0) {
                        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0xe0, 0xa8), CONTENT_Z + 0.03);
                    }
                }
                fill(commands, 15.0, y + 1.0, 18.0, 17.0, Color::srgb_u8(0x10, 0x10, 0x18), CONTENT_Z + 0.02);
                let face = portrait(cache, r.seed, images);
                commands.spawn((Sprite::from_image(face), at(16.0, y + 2.0, 16.0, 16.0, CONTENT_Z + 0.04), PIXEL_LAYER, tag()));
                label(commands, images, &r.name, 37.0, y + 3.0, if on { 0xfcfcfc } else { tier_of(h).1 }, CONTENT_Z + 0.04, tag());
                if h == 0 {
                    label(commands, images, "A NEW FACE", 37.0, y + 12.0, 0x5a5a62, CONTENT_Z + 0.04, tag());
                } else {
                    for hh in 0..h.min(10) {
                        let img = heart_image(1.0, on, images);
                        commands.spawn((Sprite::from_image(img), at(37.0 + hh as f32 * 7.0, y + 12.0, 5.0, 5.0, CONTENT_Z + 0.04), PIXEL_LAYER, tag()));
                    }
                }
            }
        }
    }
    if scroll > 0 {
        label(commands, images, "<", 6.0 + lw - 8.0, y0 - 8.0, 0xe8c860, CONTENT_Z + 0.04, tag());
    }
    if scroll + VIS < list.len() {
        label(commands, images, ">", 6.0 + lw - 8.0, y0 + VIS as f32 * RH - 2.0, 0xe8c860, CONTENT_Z + 0.04, tag());
    }

    // --- Right: the standard dex pane. ---
    let pw = CANVAS_W as f32 - 6.0 - dex::DEX_RX;
    let cx = dex::DEX_RX + (pw / 2.0).round();
    let py = dex::DEX_GY;
    match &list[cur] {
        Row::Place { name, n, folded } => {
            dex::draw_pane(commands, images, true, None, name, None, "", true, tag());
            // The little crowd on the plate (js: up to 9 portraits, 3x3).
            let sel: Vec<&PersonRec> = {
                let mut l: Vec<&PersonRec> = ledger
                    .0
                    .values()
                    .filter(|r| !r.name.is_empty() && r.town.as_deref().unwrap_or(WANDERERS) == name)
                    .collect();
                l.sort_by(|a, b| b.pts.cmp(&a.pts).then_with(|| a.name.cmp(&b.name)));
                l
            };
            for (i, r) in sel.iter().take(9).enumerate() {
                let face = portrait(cache, r.seed, images);
                let (fx, fy) = (cx - 25.0 + (i % 3) as f32 * 17.0, py + 45.0 - 25.0 + (i / 3) as f32 * 17.0);
                commands.spawn((Sprite::from_image(face), at(fx, fy, 16.0, 16.0, CONTENT_Z + 0.12), PIXEL_LAYER, tag()));
            }
            let souls = format!("{n} {}", if *n == 1 { "SOUL KNOWN" } else { "SOULS KNOWN" });
            dex::center_label(commands, images, &souls, cx, py + 90.0, 0x8a8a92, CONTENT_Z + 0.1, tag());
            let total: i32 = sel.iter().map(|r| people::hearts(r.pts)).sum();
            let t1 = format!("TOGETHER: {total} {}", if total == 1 { "HEART" } else { "HEARTS" });
            dex::center_label(commands, images, &t1, cx, py + 108.0, 0xfc9ab8, CONTENT_Z + 0.1, tag());
            if let Some(best) = sel.first() {
                let t2 = format!("CLOSEST: {}", best.name);
                dex::center_label(commands, images, &t2, cx, py + 122.0, tier_of(people::hearts(best.pts)).1, CONTENT_Z + 0.1, tag());
            }
            let t3 = if *folded { "UNFOLD" } else { "FOLD" };
            dex::center_label(commands, images, t3, cx, py + 140.0, 0x5a5a62, CONTENT_Z + 0.1, tag());
        }
        Row::Person(r) => {
            let h = people::hearts(r.pts);
            let (tier, tier_col) = tier_of(h);
            let sub = r.town.as_ref().map_or("A WANDERER".to_string(), |t| format!("OF {t}"));
            let face = portrait(cache, r.seed, images);
            dex::draw_pane(commands, images, true, Some((face, 48.0)), &r.name, Some((&sub, 0x8a8a92)), "", true, tag());
            // Their mark, level with the name (js drawGenderSym beside the centred name).
            let mark = if people::gender_for(r.seed) == "F" { (F_MARK, 0xfc9ab8) } else { (M_MARK, 0x8ab0e0) };
            let mx = cx + (font::measure(&r.name) as f32 / 2.0) + 8.0;
            commands.spawn((
                Sprite::from_image(images.add(bake(mark.0, &[('#', mark.1)]))),
                at(mx, py + 70.0, 5.0, 7.0, CONTENT_Z + 0.12),
                PIXEL_LAYER,
                tag(),
            ));
            dex::center_label(commands, images, tier, cx, py + 102.0, tier_col, CONTENT_Z + 0.1, tag());
            // The ten-heart row, the earned one part-filled (js drawHeartRow).
            let part = (r.pts % people::HEART_PTS) as f32 / people::HEART_PTS as f32;
            for i in 0..10 {
                let frac = if i < h { 1.0 } else if i == h { part } else { 0.0 };
                let img = heart_image(frac, true, images);
                commands.spawn((Sprite::from_image(img), at(cx - 44.0 + i as f32 * 9.0, py + 114.0, 5.0, 5.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag()));
            }
            let hl = if h >= 10 {
                "A BOND FOR LIFE".to_string()
            } else {
                format!("HEART {}: {}/{}", h + 1, r.pts % people::HEART_PTS, people::HEART_PTS)
            };
            dex::center_label(commands, images, &hl, cx, py + 126.0, 0x9a9aa8, CONTENT_Z + 0.1, tag());
            let today = crate::app::gather::farm_day(clock); // the dawn day (matches talk.rs)
            let days = today - if r.last_chat >= 0 { r.last_chat } else { today };
            let lc = if r.last_chat < 0 {
                "NEVER PROPERLY MET".to_string()
            } else if days <= 0 {
                "SPOKE TODAY".to_string()
            } else if days == 1 {
                "SPOKE YESTERDAY".to_string()
            } else {
                format!("LAST SPOKE {days} DAYS AGO")
            };
            dex::center_label(commands, images, &lc, cx, py + 140.0, if days <= 0 && r.last_chat >= 0 { 0xa8e0a8 } else { 0x8a8a92 }, CONTENT_Z + 0.1, tag());
            // Tastes stay a mystery until revealed; birthdays once you've been told.
            // (js rows 154/166 — pulled up 2/4px: our canvas is 8px shorter, CANVAS_H note.)
            let ts = people::taste_for(r.seed);
            let tst = match (r.know_love, r.know_hate) {
                (true, true) => format!("LOVES {} - HATES {}", people::taste_word(ts.love), people::taste_word(ts.hate)),
                (true, false) => format!("LOVES {}", people::taste_word(ts.love)),
                (false, true) => format!("HATES {}", people::taste_word(ts.hate)),
                (false, false) => "TASTES: A MYSTERY".to_string(),
            };
            let tcol = if tst == "TASTES: A MYSTERY" { 0x5a5a62 } else { 0xd8b8c8 };
            dex::center_label(commands, images, &tst, cx, py + 152.0, tcol, CONTENT_Z + 0.1, tag());
            let bst = if r.know_bday {
                let b = people::birthday_for(r.seed);
                format!("BORN {} {}", super::calendar_tab::SEASONS[b.season as usize], b.day)
            } else {
                "BIRTHDAY: NOT YET SHARED".to_string()
            };
            dex::center_label(commands, images, &bst, cx, py + 162.0, if r.know_bday { 0xd8b8c8 } else { 0x5a5a62 }, CONTENT_Z + 0.1, tag());
        }
    }
}

/// Fresh cursor next visit (folds persist for the session, js peopleCollapsed).
pub fn reset(mut px: ResMut<PeopleDex>) {
    px.cur = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roster_groups_and_sorts() {
        let mut ledger = PeopleLedger::default();
        let rec = |name: &str, town: Option<&str>, pts: i32| PersonRec {
            name: name.into(),
            town: town.map(String::from),
            pts,
            ..Default::default()
        };
        ledger.0.insert("a".into(), rec("ZED", Some("OAKDALE"), 50));
        ledger.0.insert("b".into(), rec("ANA", Some("OAKDALE"), 300));
        ledger.0.insert("c".into(), rec("MOSS", None, 10));
        let list = rows(&ledger, &HashSet::default());
        // OAKDALE banner, ANA (warmest first), ZED, then the wanderers' banner + MOSS.
        assert_eq!(list.len(), 5);
        assert!(matches!(&list[0], Row::Place { name, n: 2, .. } if name == "OAKDALE"));
        assert!(matches!(&list[1], Row::Person(r) if r.name == "ANA"));
        assert!(matches!(&list[2], Row::Person(r) if r.name == "ZED"));
        assert!(matches!(&list[3], Row::Place { name, n: 1, .. } if name == WANDERERS));
        // Folding hides the group's people.
        let mut folded = HashSet::default();
        folded.insert("OAKDALE".to_string());
        let list = rows(&ledger, &folded);
        assert_eq!(list.len(), 3);
    }
}
