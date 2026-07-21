//! app/status.rs — the status-effect system (js status.js registry + player.js
//! statuses). One resource tracks active effects (id -> frames left, max-refresh on
//! reapply, js addStatus); the tick recomputes the player's LIVE defense each frame
//! (js p.defense = p.stat('defense')), runs the DoTs (poison every 36, burn every 30
//! w/ flash — a DoT never lands the killing blow), and the regen clock (heal every
//! max(40, 300 - regen*26) below full). Buff stats (move/melee/defense/regen) sum
//! into play wherever the base stat is read. The HUD wears a little icon row.
//! Debuffs arrive ON HIT: mobs/projectiles carry [`crate::combat::Afflicts`] and
//! resolve_combat forwards it in HitLanded for players only.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::Player;
use crate::combat::Health;

/// Per-effect stat bonuses (js DEFS stats{} — summed while active).
#[derive(Clone, Copy, Default)]
pub struct StatMods {
    pub mv: f64,
    pub melee: f64,
    pub defense: f64,
    pub regen: f64,
    pub luck: f64,
    pub crit: f64,
}

pub struct StatusDef {
    pub id: &'static str,
    pub name: &'static str,
    pub color: u32,
    pub mods: StatMods,
    pub icon: &'static [&'static str],
}

const M0: StatMods = StatMods { mv: 0.0, melee: 0.0, defense: 0.0, regen: 0.0, luck: 0.0, crit: 0.0 };

/// 10x10 icons (pixel redraws of the js vector glyphs); 'X' takes the def colour,
/// 'W' white, 'K' black — baked with the colour override at rig build.
pub static DEFS: &[StatusDef] = &[
    StatusDef { id: "blessing", name: "BELLNIGHT BLESSING", color: 0xbfe0ff, mods: StatMods { luck: 0.15, regen: 0.5, ..M0 },
        icon: &["....XX....", "...XXXX...", "..XXXXXX..", "..XXXXXX..", ".XXXXXXXX.", ".XXXXXXXX.", "XXXXXXXXXX", "....WW....", "....WW....", ".........."] },
    StatusDef { id: "hunterhour", name: "HUNTERS HOUR", color: 0x9ab0e0, mods: StatMods { mv: 0.1, crit: 0.1, ..M0 },
        icon: &["...XXXX...", "..XXXXXX..", ".XXXX.....", ".XXX......", ".XXX......", ".XXX......", ".XXXX.....", "..XXXXXX..", "...XXXX...", ".........."] },
    StatusDef { id: "waysong", name: "WAYSONG", color: 0x8ee0a0, mods: StatMods { mv: 0.12, regen: 0.4, ..M0 },
        icon: &["......X...", "......XX..", "......XXX.", "......X...", "......X...", "......X...", "..XXX.X...", ".XXXXXX...", "..XXXX....", ".........."] },
    StatusDef { id: "poison", name: "POISONED", color: 0x5acb3a, mods: M0,
        icon: &["....X.....", "....XX....", "...XXX....", "...XXXX...", "..XXXXX...", ".XXXXXXX..", ".XXWXXXX..", ".XXXXXXX..", "..XXXXX...", "...XXX...."] },
    StatusDef { id: "burn", name: "BURNING", color: 0xfc7430, mods: M0,
        icon: &["....X.....", "...XX.....", "...XXX....", "..XXXXX...", "..XXWXX...", ".XXWWXXX..", ".XXWWWXX..", ".XXXWXXX..", "..XXXXX...", "...XXX...."] },
    StatusDef { id: "slow", name: "SLOWED", color: 0x8fb6e0, mods: M0,
        icon: &[".XXXXXXXX.", "..XXXXXX..", "...XXXX...", "....XX....", "....WW....", "....XX....", "...XXXX...", "..XXXXXX..", ".XXXXXXXX.", ".........."] },
    StatusDef { id: "shock", name: "SHOCKED", color: 0xfce64a, mods: M0,
        icon: &[".....XX...", "....XX....", "...XX.....", "..XXXXX...", "....XX....", "...XX.....", "..XX......", ".XX.......", "XX........", ".........."] },
    StatusDef { id: "warpcd", name: "WARP RECHARGING", color: 0xb890ff, mods: M0,
        icon: &["...XXXX...", "..X....X..", ".X..XX..X.", ".X.X..X.X.", ".X.X.WX.X.", ".X..XX..X.", "..X....X..", "...XXXX...", "..........", ".........."] },
    StatusDef { id: "ward", name: "WARDSONG", color: 0x7fb0e0, mods: StatMods { defense: 2.0, ..M0 },
        icon: &[".XXXXXXXX.", ".XXXXXXXX.", ".XXXWXXXX.", ".XXXWWXXX.", ".XXWXXXXX.", "..XXXXXX..", "..XXXXXX..", "...XXXX...", "....XX....", ".........."] },
    StatusDef { id: "wellfed", name: "WELL FED", color: 0xf0a848, mods: StatMods { regen: 1.0, ..M0 },
        icon: &["..X...X...", "..X...X...", "..X...X...", "..XXXXX...", "...XXX....", "..XXXXX...", ".XXXWXXX..", ".XXXXXXX..", "..XXXXX...", ".........."] },
    StatusDef { id: "mighty", name: "MIGHTY", color: 0xfc5040, mods: StatMods { melee: 0.2, ..M0 },
        icon: &["....XX....", "...XXXX...", "..XXXXXX..", ".XXXXXXXX.", "...XXXX...", "...XXXX...", "...XXXX...", "...XXXX...", "...XXXX...", ".........."] },
    StatusDef { id: "guarded", name: "GUARDED", color: 0x7fb0e0, mods: StatMods { defense: 1.0, ..M0 },
        icon: &[".XXXXXXXX.", ".XXXXXXXX.", ".XXXWWXXX.", ".XWWWWWWX.", ".XXXWWXXX.", "..XXWWXX..", "..XXXXXX..", "...XXXX...", "....XX....", ".........."] },
    StatusDef { id: "swift", name: "SWIFT", color: 0xfce04a, mods: StatMods { mv: 0.15, ..M0 },
        icon: &["....XXX...", "...XXX....", "..XXX.....", ".XXXXXX...", "....XXX...", "...XXX....", "..XXX.....", ".XXX......", ".XX.......", ".........."] },
    StatusDef { id: "lucky", name: "LUCKY", color: 0x3cdc5a, mods: StatMods { luck: 0.12, ..M0 },
        icon: &["...XX.....", "..XXXX....", ".XX..XX...", "..XXXX....", ".XX..XX...", "..XXXX....", "...XX.....", "...XX.....", "...XX.....", ".........."] },
    StatusDef { id: "keen", name: "KEEN", color: 0xe8f0ff, mods: StatMods { crit: 0.12, ..M0 },
        icon: &["..XXXXXX..", ".X......X.", "X...XX...X", "X..XWWX..X", "X..XWWX..X", "X...XX...X", ".X......X.", "..XXXXXX..", "..........", ".........."] },
];

pub fn def(id: &str) -> Option<&'static StatusDef> {
    DEFS.iter().find(|d| d.id == id)
}

/// The player's active effects (js p.statuses: id -> {t, dur}).
#[derive(Resource, Default)]
pub struct Statuses {
    pub active: HashMap<&'static str, (i32, i32)>, // id -> (frames left, full duration)
    dot_clock: HashMap<&'static str, i32>,
    regen_t: i32,
}

impl Statuses {
    /// js addStatus: reapplying refreshes to at least `frames`.
    pub fn add(&mut self, id: &'static str, frames: i32) {
        let e = self.active.entry(id).or_insert((0, frames));
        e.0 = e.0.max(frames);
        e.1 = frames;
    }
    pub fn has(&self, id: &str) -> bool {
        self.active.contains_key(id)
    }
    /// Sum a stat field over every active effect (js statusStat).
    pub fn remove(&mut self, id: &str) {
        self.active.remove(id);
        self.dot_clock.remove(id);
    }

    pub fn sum(&self, f: impl Fn(&StatMods) -> f64) -> f64 {
        self.active.keys().filter_map(|id| def(id)).map(|d| f(&d.mods)).sum()
    }
}

/// The per-tick heart (js player.js): expiry, live defense, DoTs, regen.
fn status_tick(
    mut statuses: ResMut<Statuses>,
    tstats: Res<super::slideout::TreeStats>,
    mut players: Query<&mut Health, With<Player>>,
) {
    let s = &mut *statuses;
    s.active.retain(|_, (t, _)| {
        *t -= 1;
        *t > 0
    });
    let Ok(mut h) = players.single_mut() else { return };
    // Live defense (js: p.defense = p.stat('defense') every tick — a GUARDED buff or
    // its expiry lands at once; wardsong rides this too).
    h.defense = tstats.defense.round() as i32 + s.sum(|m| m.defense).round() as i32;
    // DoTs (js DOTS): poison bites every 36, burn every 30 (with a flash) — and a DoT
    // never lands the killing blow on its own.
    for (id, every, flash) in [("poison", 36, 0u32), ("burn", 30, 6)] {
        if s.active.contains_key(id) {
            let c = s.dot_clock.entry(id).or_insert(0);
            *c += 1;
            if *c >= every {
                *c = 0;
                if h.hp > 1 {
                    h.hp -= 1;
                    if flash > 0 {
                        h.flash = h.flash.max(flash);
                    }
                }
            }
        } else {
            s.dot_clock.insert(id, 0);
        }
    }
    // Regen (js): tree ranks + buffs together; heal 1 every max(40, 300 - regen*26).
    let regen = tstats.regen + s.sum(|m| m.regen);
    if regen > 0.0 && h.hp < h.max {
        s.regen_t += 1;
        if s.regen_t >= (300.0 - regen * 26.0).max(40.0) as i32 {
            s.regen_t = 0;
            h.hp += 1;
        }
    } else {
        s.regen_t = 0;
    }
}

/// On-hit afflictions: resolve_combat forwards the attacker's Afflicts for player
/// targets — webs mire, scorpions envenom, rime chills.
fn afflict_on_hit(
    mut statuses: ResMut<Statuses>,
    players: Query<Entity, With<Player>>,
    mut hits: MessageReader<crate::combat::HitLanded>,
) {
    let Ok(pe) = players.single() else { return };
    for hit in hits.read() {
        if hit.target == pe
            && let Some((id, frames)) = hit.afflicts
            && !id.is_empty()
        {
            statuses.add(id, frames);
        }
    }
}

/// The HUD's little icon row (js status widget): active effects under the sidebar
/// clock, blinking out their last three seconds.
#[derive(Component)]
struct StatusIcon(&'static str);

fn hud_icons(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    statuses: Res<Statuses>,
    layout: Res<super::hud::SidebarLayout>,
    mut cache: Local<HashMap<&'static str, Handle<Image>>>,
    mut icons: Query<(Entity, &StatusIcon, &mut Visibility)>,
) {
    // One icon entity per active effect, laid out in def order; despawn the expired.
    let mut want: Vec<&'static StatusDef> = DEFS.iter().filter(|d| statuses.has(d.id)).collect();
    want.truncate(8);
    for (e, icon, _) in &icons {
        if !want.iter().any(|d| d.id == icon.0) {
            commands.entity(e).despawn();
        }
    }
    for (i, d) in want.iter().enumerate() {
        // The widget STACK hands us our y (below the quest list, sliding up when
        // it's empty) — the hand-picked 130.0 overlapped a full quest log (Baz).
        let x = 10.0 + (i % 4) as f32 * 13.0;
        let y = layout.buffs_y + (i / 4) as f32 * 13.0;
        let existing = icons.iter_mut().find(|(_, ic, _)| ic.0 == d.id);
        let blink = statuses
            .active
            .get(d.id)
            .map(|(t, _)| *t < 180 && (*t / 8) % 2 == 0)
            .unwrap_or(false);
        match existing {
            Some((e, _, mut vis)) => {
                *vis = if blink { Visibility::Hidden } else { Visibility::Inherited };
                commands.entity(e).insert(crate::gfx::at(x, y, 10.0, 10.0, 32.0));
            }
            None => {
                let img = cache
                    .entry(d.id)
                    .or_insert_with(|| images.add(crate::gfx::bake(d.icon, &[('X', d.color)])))
                    .clone();
                commands.spawn((
                    Sprite::from_image(img),
                    crate::gfx::at(x, y, 10.0, 10.0, 32.0),
                    crate::gfx::PIXEL_LAYER,
                    StatusIcon(d.id),
                ));
            }
        }
    }
    let _ = RoomActor; // icons are UI, not room cast — they persist across slides
}

/// Eat a cooked meal (play.rs consumable branch writes; the buffs live here so the
/// dish table sits beside the DEFS it points into). js: each dish's use() addStatus.
#[derive(Message)]
pub struct EatDish(pub &'static str);

/// Antidote: clears poison + slow (js use()). The veto (not sick -> not consumed)
/// is decided in play.rs, which reads the same Statuses.
#[derive(Message)]
pub struct CureStatus;

/// Every dish's buff list (js: each dish's use() addStatus calls) — data, so the
/// wiring test can walk it against the item defs.
pub static DISH_BUFFS: &[(&str, &[(&str, i32)])] = &[
    ("roast", &[("wellfed", 5400)]),
    ("stew", &[("guarded", 7200)]), // + cures poison (eat_dish)
    ("skewer", &[("mighty", 7200)]),
    ("saute", &[("swift", 5400)]),
    ("pie", &[("keen", 7200)]),
    ("tart", &[("lucky", 7200)]),
    ("grilledfish", &[("wellfed", 5400)]),
    ("chowder", &[("wellfed", 7200), ("guarded", 7200)]),
    ("anglersfry", &[("lucky", 7200), ("swift", 7200)]),
];

fn cure_status(mut cures: MessageReader<CureStatus>, mut statuses: ResMut<Statuses>, mut sfx: MessageWriter<super::sfx::Sfx>) {
    for _ in cures.read() {
        statuses.remove("poison");
        statuses.remove("slow");
        sfx.write(super::sfx::Sfx("pickup")); // "potion" isn't in the synth bank — was silent
    }
}

fn eat_dish(mut eats: MessageReader<EatDish>, mut statuses: ResMut<Statuses>, mut sfx: MessageWriter<super::sfx::Sfx>) {
    for EatDish(id) in eats.read() {
        let Some((_, buffs)) = DISH_BUFFS.iter().find(|(d, _)| d == id) else { continue };
        if *id == "stew" {
            statuses.remove("poison"); // comfort food cures what ails you
        }
        for (buff, frames) in *buffs {
            statuses.add(buff, *frames);
        }
        sfx.write(super::sfx::Sfx("pickup")); // "potion" isn't in the synth bank — was silent
    }
}

pub struct StatusPlugin;
impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Statuses>()
            .add_message::<EatDish>()
            .add_message::<CureStatus>()
            .add_systems(
                bevy::app::FixedUpdate,
                (status_tick, eat_dish, cure_status, afflict_on_hit.after(crate::combat::resolve_combat))
                    .before(super::play::EndTick)
                    .run_if(super::screen::playing),
            )
            .add_systems(Update, hud_icons);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sound() {
        for d in DEFS {
            assert_eq!(d.icon.len(), 10, "{} icon height", d.id);
            for r in d.icon {
                assert_eq!(r.chars().count(), 10, "{} icon width", d.id);
            }
        }
        let mut ids: Vec<_> = DEFS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DEFS.len(), "duplicate status id");
    }
}
