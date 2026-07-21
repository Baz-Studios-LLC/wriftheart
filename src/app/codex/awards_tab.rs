//! awards_tab.rs — THE HALL OF DEEDS (js drawAchDex + the unlock ticker): the grouped
//! award ledger on the LEFT (category banners, medal studs, live progress), the medal
//! plaque on the RIGHT. Up/down walk the list; left/right hop whole categories. Hidden
//! awards stay '? ? ?' until earned. The ticker also lives here: every half second of
//! play it measures the snapshot and unlocks anything newly earned (toast + saved).

use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::achievements::{self, AchStats};
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;
use crate::{CANVAS_H, CANVAS_W};
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// Deeds done — once earned, stays earned (js unlockedAchievements, saved).
#[derive(Resource, Default)]
pub struct Unlocked(pub HashSet<&'static str>);

/// The ledger cursor.
#[derive(Resource, Default)]
pub struct AwardsDex {
    pub cur: usize,
}

/// Everything the snapshot measures (grouped under the 16-param cap).
#[derive(SystemParam)]
pub struct AchCtx<'w> {
    pub stats: Res<'w, crate::app::stats::Stats>,
    pub bestiary: Res<'w, super::mobs_tab::Bestiary>,
    pub discovered: Res<'w, super::items_tab::Discovered>,
    pub visited: Res<'w, crate::app::play::Visited>,
    pub cleared: Res<'w, crate::app::encounters::ClearedEncounters>,
    pub giver_done: Res<'w, crate::app::quests::GiverDone>,
    pub relics: Res<'w, crate::app::dungeon::Relics>,
    pub dungeon_ledger: Res<'w, crate::app::dungeon::DungeonLedger>,
    pub town_names: Res<'w, crate::app::banners::TownNames>,
    pub learned: Res<'w, crate::app::flute::LearnedSongs>,
    pub gather: Res<'w, crate::app::gather::GatherState>,
    pub progress: Res<'w, crate::app::rewards::Progress>,
    pub inv: Res<'w, crate::inventory::PlayerInv>,
    pub people: Res<'w, crate::app::talk::PeopleLedger>,
}

/// js achStats() — the live world measured into one snapshot. Fields whose systems
/// haven't ported (homes, animals, guild wings, songstones, rifts…) read 0 and their
/// awards simply wait.
pub fn snapshot(cx: &AchCtx) -> AchStats {
    let s = &cx.stats;
    let mut met = 0.0;
    let mut best_hearts: f64 = 0.0;
    for r in cx.people.0.values() {
        if !r.name.is_empty() {
            met += 1.0;
            best_hearts = best_hearts.max(crate::people::hearts(r.pts) as f64);
        }
    }
    AchStats {
        kills: s.get("kills"),
        mobs: cx.bestiary.0.len() as f64,
        mobs_total: (crate::actors::mobs::MOB_DEFS.len() + 2) as f64, // + goblin, slinger
        elites: s.get("elites") + s.get("champions"),
        bosses: s.get("bosses"),
        gate: s.get("gate"),
        dmg: s.get("dmg"),
        deaths: s.get("deaths"),
        level: cx.progress.level as f64,
        rooms: cx.visited.0.len() as f64,
        walk: s.get("walk"),
        towns: cx.town_names.0.len() as f64,
        dungeons: cx.dungeon_ledger.0.len() as f64, // entered stands in for discovered
        encounters: cx.cleared.0.len() as f64,
        warps: s.get("warps"),
        relics: cx.relics.0.len() as f64,
        won: s.get("won"),
        kingsplitter: if cx.inv.has_item("kingsplitter") { 1.0 } else { 0.0 },
        rift_best: s.get("riftBest"),
        riftfloors: s.get("riftfloors"),
        home: s.get("home"),
        sleeps: s.get("sleeps"),
        crops: s.get("crops"),
        eggs: s.get("eggs"),
        milk: s.get("milk"),
        pets: s.get("pets"),
        animals: s.get("animals"),
        fish: s.get("fish"),
        bigfish: s.get("bigFish"),
        junk: s.get("junk"),
        crafts: s.get("crafts"),
        blueprints: s.get("blueprints"),
        tables: s.get("tables"),
        trees: s.get("trees"),
        rocks: s.get("stones"),
        items: cx.discovered.0.len() as f64,
        items_total: crate::items::all_defs().count() as f64,
        money: cx.inv.money as f64,
        coins_lifetime: s.get("coins"),
        met,
        hellos: s.get("hellos"),
        best_hearts,
        gifts: s.get("gifts"),
        quests: cx.giver_done.0.values().map(|n| *n as f64).sum(),
        festivals: s.get("festivals"),
        wings: s.get("wings"),
        full_halls: s.get("fullHalls"),
        songs: cx.learned.0.len() as f64,
        songs_total: crate::songs::LIST.len() as f64,
        songstones: s.get("songstones"),
        books: cx.gather.tomes.len() as f64,
        books_total: crate::lore_books::BOOKS.len() as f64,
        digs: s.get("digs"),
        chests: s.get("chests"),
    }
}

/// The half-second ticker: anything newly earned unlocks with a toast (js checks its
/// achievements against the snapshot in the main loop).
pub fn award_ticker(
    ctx: AchCtx,
    mut unlocked: ResMut<Unlocked>,
    mut log: ResMut<crate::app::rewards::LootLog>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut saves: MessageWriter<crate::app::save::SaveRequest>,
    mut tick: Local<u32>,
) {
    *tick += 1;
    if !(*tick).is_multiple_of(32) {
        return;
    }
    let s = snapshot(&ctx);
    let mut any = false;
    for a in achievements::LIST {
        if unlocked.0.contains(a.id) {
            continue;
        }
        let goal = (a.goal)(&s);
        if goal > 0.0 && (a.cur)(&s) >= goal {
            unlocked.0.insert(a.id);
            log.add("award", &format!("DEED DONE: {}", a.name.to_uppercase()), 1, 0xffd34d, false, true);
            sfx.write(crate::app::sfx::Sfx("levelup"));
            any = true;
        }
    }
    if any {
        saves.write(crate::app::save::SaveRequest);
    }
}

// --- The codex tab ---------------------------------------------------------------

// js dex geometry: ledger column left, detail plaque right.
const LX: f32 = 8.0;
const RX: f32 = 214.0; // js DEX_RX = 8 + 9*22 + 8
const LW: f32 = RX - LX - 8.0;
const GY: f32 = 27.0;
const RH: f32 = 11.0;

/// Trophy badge (js STAR_ART) — gold earned, gray locked.
const STAR_ART: &[&str] = &["...PP...", "...PP...", "PPPPPPPP", ".PPPPPP.", "..PPPP..", ".PPPPPP.", ".PP..PP.", "........"];

#[derive(Component, Clone)]
pub struct AwardsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let ud = format!("{}/{}", bindings.prompt(Action::Up, pad), bindings.prompt(Action::Down, pad));
    let lr = format!("{}/{}", bindings.prompt(Action::Left, pad), bindings.prompt(Action::Right, pad));
    hint_scaffold(bindings, pad, &format!("{ud} BROWSE - {lr} CATEGORY"))
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    input: Res<ActionState>,
    ptr: Res<crate::input::Pointer>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    cx_state: Res<CodexState>,
    ctx: AchCtx,
    unlocked: Res<Unlocked>,
    mut dex: ResMut<AwardsDex>,
    old: Query<Entity, With<AwardsUi>>,
    mut last: Local<Option<(u32, usize, usize)>>,
) {
    let list = achievements::LIST;
    let n = list.len();
    // js updateAchDex: rows walk, categories hop.
    if input.pressed(Action::Up) {
        dex.cur = (dex.cur + n - 1) % n;
        sfx.write(crate::app::sfx::Sfx("menuMove"));
    }
    if input.pressed(Action::Down) {
        dex.cur = (dex.cur + 1) % n;
        sfx.write(crate::app::sfx::Sfx("menuMove"));
    }
    if ptr.wheel_steps != 0 {
        // Wheel walks the ledger, clamped (Baz: any scrollable list).
        dex.cur = (dex.cur as i32 - ptr.wheel_steps).clamp(0, n as i32 - 1) as usize;
    }
    if input.pressed(Action::Left) || input.pressed(Action::Right) {
        let dir: i32 = if input.pressed(Action::Right) { 1 } else { -1 };
        let mut cats: Vec<&str> = Vec::new();
        for a in list {
            if cats.last() != Some(&a.cat) {
                cats.push(a.cat);
            }
        }
        let ci = cats.iter().position(|c| *c == list[dex.cur].cat).unwrap_or(0) as i32;
        let nc = cats[(ci + dir).rem_euclid(cats.len() as i32) as usize];
        dex.cur = list.iter().position(|a| a.cat == nc).unwrap_or(0);
        sfx.write(crate::app::sfx::Sfx("menuMove"));
    }

    let key = (cx_state.generation, dex.cur, unlocked.0.len());
    if Some(key) == *last {
        return;
    }
    *last = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, AwardsUi);
    let s = snapshot(&ctx);

    // Header: title, earned tally, hall-wide progress bar.
    label(&mut commands, &mut images, "THE HALL OF DEEDS", LX, 16.0, 0xe8c860, CONTENT_Z + 0.1, tag());
    let tally = format!("{}/{}", unlocked.0.len(), n);
    let tw = font::measure(&tally) as f32;
    label(&mut commands, &mut images, &tally, LX + LW - tw, 16.0, 0x8a8a92, CONTENT_Z + 0.1, tag());
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.08), Vec2::new(LW, 2.0)),
        at(LX, 23.0, LW, 2.0, CONTENT_Z + 0.1),
        PIXEL_LAYER,
        tag(),
    ));
    let frac = unlocked.0.len() as f32 / n as f32;
    if frac > 0.0 {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0xe8, 0xc8, 0x60), Vec2::new((LW * frac).round(), 2.0)),
            at(LX, 23.0, (LW * frac).round(), 2.0, CONTENT_Z + 0.11),
            PIXEL_LAYER,
            tag(),
        ));
    }

    // The grouped ledger: category banners + award lines (js rows).
    enum Row {
        Banner(&'static str),
        Award(usize),
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut sel_row = 0;
    for (i, aw) in list.iter().enumerate() {
        if i == 0 || aw.cat != list[i - 1].cat {
            rows.push(Row::Banner(aw.cat));
        }
        if i == dex.cur {
            sel_row = rows.len();
        }
        rows.push(Row::Award(i));
    }
    let y0 = GY + 1.0;
    let vis = ((CANVAS_H as f32 - y0 - 12.0) / RH).floor() as usize;
    let scroll = sel_row.saturating_sub(vis / 2).min(rows.len().saturating_sub(vis));
    for r in 0..vis {
        let Some(row) = rows.get(scroll + r) else { break };
        let y = y0 + r as f32 * RH;
        match row {
            Row::Banner(cat) => {
                let cc = achievements::cat_color(cat);
                commands.spawn((
                    Sprite::from_color(Color::srgba(0.91, 0.78, 0.38, 0.08), Vec2::new(LW, RH - 2.0)),
                    at(LX, y, LW, RH - 2.0, CONTENT_Z + 0.1),
                    PIXEL_LAYER,
                    tag(),
                ));
                commands.spawn((
                    Sprite::from_color(Color::srgb_u8((cc >> 16) as u8, (cc >> 8) as u8, cc as u8), Vec2::new(2.0, RH - 2.0)),
                    at(LX, y, 2.0, RH - 2.0, CONTENT_Z + 0.11),
                    PIXEL_LAYER,
                    tag(),
                ));
                label(&mut commands, &mut images, cat, LX + 6.0, y + 1.0, cc, CONTENT_Z + 0.12, tag());
            }
            Row::Award(i) => {
                let aw = &list[*i];
                let got = unlocked.0.contains(aw.id);
                let sel = *i == dex.cur;
                let hide = aw.hidden && !got;
                if sel {
                    commands.spawn((
                        Sprite::from_color(Color::srgba(0.99, 0.88, 0.66, 0.13), Vec2::new(LW, RH)),
                        at(LX, y - 1.0, LW, RH, CONTENT_Z + 0.1),
                        PIXEL_LAYER,
                        tag(),
                    ));
                }
                // The medal stud.
                let stud = if got { 0xffd34d } else { 0x3a3e48 };
                commands.spawn((
                    Sprite::from_color(Color::srgb_u8((stud >> 16) as u8, (stud >> 8) as u8, stud as u8), Vec2::new(3.0, 3.0)),
                    at(LX + 5.0, y + 2.0, 3.0, 3.0, CONTENT_Z + 0.11),
                    PIXEL_LAYER,
                    tag(),
                ));
                let name = if hide { "? ? ?" } else { aw.name };
                let nc = if got {
                    if sel { 0xfcfcfc } else { 0xe8e0c0 }
                } else if sel {
                    0xc8c8d0
                } else {
                    0x7a7a84
                };
                label(&mut commands, &mut images, name, LX + 12.0, y + 1.0, nc, CONTENT_Z + 0.12, tag());
                // Progress: '*' earned, '?' hidden, else floor(cur)/goal.
                let pg = if got {
                    "*".to_string()
                } else if hide {
                    "?".to_string()
                } else {
                    format!("{}/{}", ((a_cur(aw, &s)).min((aw.goal)(&s))).floor() as i64, (aw.goal)(&s) as i64)
                };
                let pw = font::measure(&pg) as f32;
                label(&mut commands, &mut images, &pg, LX + LW - 6.0 - pw, y + 1.0, if got { 0xffd34d } else { 0x4a4e58 }, CONTENT_Z + 0.12, tag());
            }
        }
    }
    if scroll > 0 {
        label(&mut commands, &mut images, "<", LX + LW + 1.0, y0, 0xe8c860, CONTENT_Z + 0.12, tag());
    }
    if scroll + vis < rows.len() {
        label(&mut commands, &mut images, ">", LX + LW + 1.0, y0 + (vis as f32 - 1.0) * RH, 0xe8c860, CONTENT_Z + 0.12, tag());
    }

    // The medal plaque (right pane).
    let aw = &list[dex.cur];
    let got = unlocked.0.contains(aw.id);
    let hide = aw.hidden && !got;
    let (px, py) = (RX, GY);
    let (pw, ph) = (CANVAS_W as f32 - 6.0 - px, CANVAS_H as f32 - GY - 14.0);
    let pcx = px + (pw / 2.0).round();
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), Vec2::new(pw, ph)),
        at(px, py, pw, ph, CONTENT_Z + 0.05),
        PIXEL_LAYER,
        tag(),
    ));
    for (sx, sy, sw, sh) in crate::ui::border_strips(px, py, pw, ph, 1.0) {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x2c, 0x2c, 0x36), Vec2::new(sw, sh)),
            at(sx, sy, sw, sh, CONTENT_Z + 0.06),
            PIXEL_LAYER,
            tag(),
        ));
    }
    if got {
        // Earned: a gilded inner frame.
        for (sx, sy, sw, sh) in crate::ui::border_strips(px + 2.0, py + 2.0, pw - 4.0, ph - 4.0, 1.0) {
            commands.spawn((
                Sprite::from_color(Color::srgba(0.91, 0.78, 0.38, 0.4), Vec2::new(sw, sh)),
                at(sx, sy, sw, sh, CONTENT_Z + 0.06),
                PIXEL_LAYER,
                tag(),
            ));
        }
    }
    let cc = achievements::cat_color(aw.cat);
    let cw = font::measure(aw.cat) as f32;
    label(&mut commands, &mut images, aw.cat, (pcx - cw / 2.0).round(), py + 7.0, cc, CONTENT_Z + 0.1, tag());
    // The medal star (the js laurel ring joins with the polish pass).
    let star = images.add(if got { bake(STAR_ART, &[]) } else { bake(STAR_ART, &[('P', 0x4a4a52)]) });
    let mut spr = Sprite::from_image(star);
    spr.custom_size = Some(Vec2::splat(32.0));
    commands.spawn((spr, at(pcx - 16.0, py + 30.0, 32.0, 32.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag()));
    let nm = if hide { "? ? ?".to_string() } else { aw.name.to_uppercase() };
    let nw = font::measure(&nm) as f32;
    label(&mut commands, &mut images, &nm, (pcx - nw / 2.0).round(), py + 74.0, if got { 0xffd34d } else { 0xc8c8d0 }, CONTENT_Z + 0.1, tag());
    let desc = if hide { "A HIDDEN DEED. EARN IT TO SEE WHAT IT HONORS." } else { aw.desc };
    let mut yy = py + 88.0;
    for ln in wrap(desc, (pw - 16.0) as i32) {
        let lw = font::measure(&ln) as f32;
        label(&mut commands, &mut images, &ln, (pcx - lw / 2.0).round(), yy, 0x9aa0aa, CONTENT_Z + 0.1, tag());
        yy += 8.0;
    }
    let pg = if got {
        "EARNED".to_string()
    } else if hide {
        "? / ?".to_string()
    } else {
        format!("{} / {}", (a_cur(aw, &s)).min((aw.goal)(&s)).floor() as i64, (aw.goal)(&s) as i64)
    };
    let pgw = font::measure(&pg) as f32;
    label(&mut commands, &mut images, &pg, (pcx - pgw / 2.0).round(), py + ph - 12.0, if got { 0xffd34d } else { 0x8a8a92 }, CONTENT_Z + 0.1, tag());
}

fn a_cur(aw: &achievements::AwardDef, s: &AchStats) -> f64 {
    (aw.cur)(s)
}

/// Word-wrap to the plaque (js wrapText).
fn wrap(text: &str, max_w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let probe = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if font::measure(&probe) > max_w && !line.is_empty() {
            out.push(line);
            line = word.to_string();
        } else {
            line = probe;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}
