//! flute.rs — the Windwood Flute's play-mode (port of game.js startFlute/updateFlute/
//! castSong + drawFlute): press the flute's slot and the four MOVE keys become the four
//! NOTES, in the LIVE world — foes keep coming while you play. Play any learned song's
//! four notes and the melody catches: it sings itself back, then casts. The overlay
//! never shows notation; the SONGS codex page is the only written record.
//!
//! Also home to MANA (the songs are its first consumer — the sidebar MP bar goes live),
//! song TEACHING (the tavern bard + the seven songbooks; getting a flute back-teaches
//! any books already read), and the Lullaby's sleep/wake hooks.
//!
//! NOT YET (flagged): the Song of Opening's singing stones (their caves port
//! later), the warp CHARGE animation, and the held ocarina sustain (notes play
//! as one-shots).
//! port), the js vignette/glimmer polish, the warp charge animation, Song of Opening's
//! singing stones (no songstones in the world yet — the notes fade unanswered).

use super::battle::RoomActor;
use super::dungeon::InDungeon;
use super::interior::Inside;
use super::play::{CurRoom, GameWorld, Player, SlideActive};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::actors::mobs::Mob;
use crate::combat::{Combatant, Health, HitLanded, Hitbox, Team};
use crate::gfx::{at, font, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};

/// The warp list acts on CLICKS only (hover would scroll the list under a still mouse).
fn input_click(ptr: &crate::input::Pointer) -> bool {
    ptr.click
}
use crate::inventory::PlayerInv;
use crate::room::{PX_H, PX_W};
use crate::songs::{self, SongDef};
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

pub const MANA_BASE: i32 = 12; // js MANA_BASE — max mana with no Focus skills/gear

/// The arcane pool (js player.mana): songs spend it; it seeps back while you walk.
#[derive(Resource)]
pub struct Mana {
    pub cur: i32,
    pub max: i32,
    accum: i32, // js manaRegen — sub-point accumulator
    /// Red-flash frames after a fizzled cast (js manaFlash — the HUD bar reads it).
    pub flash: i32,
    /// Frames until regen resumes after a cast (js castCool 70).
    pub cast_cool: i32,
}
impl Default for Mana {
    fn default() -> Self {
        Mana { cur: MANA_BASE, max: MANA_BASE, accum: 0, flash: 0, cast_cool: 0 }
    }
}
impl Mana {
    /// js spendMana: pay or fizzle (flash + the caller plays the click).
    pub fn spend(&mut self, cost: i32) -> bool {
        if self.cur < cost {
            self.flash = 16;
            return false;
        }
        self.cur -= cost;
        self.cast_cool = 70; // regen ramps back up after this lapses
        true
    }
}

/// Songs you've been taught (js learnedSongs, saved) — until taught, the right notes
/// just fizzle.
#[derive(Resource, Default)]
pub struct LearnedSongs(pub HashSet<&'static str>);

pub struct Mote {
    pub x: f32,
    pub y: f32,
    pub t: i32,
    pub ltr: char,
    pub col: u32,
}

/// One musical spark (a PAST-JS flourish, Baz's ask): a twinkling fleck flung from the
/// rose — press fans off the note tips, the catch's colour ring, the staff's shimmer.
/// Play-field coords; drawn by the overlay, ticked with the motes.
pub struct Spark {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub t: i32,
    pub life: i32,
    pub col: u32,
}

pub struct Dest {
    pub name: String,
    pub rx: i32,
    pub ry: i32,
    pub dist: i32, // rooms away at cast time (the picker's "N AWAY" column)
    /// YOUR HOUSE — always the top entry, and the landing aims for the doorstep.
    pub home: bool,
}

#[derive(PartialEq)]
pub enum Phase {
    Play,
    Replay,
    Dest,
    /// The js WARP CHARGE: the song holds the hero rooted while two rings spin up
    /// around him — pain breaks it, completion teleports (warp_fx draws it).
    Warp,
}

/// The js warp clocks: channel frames before the teleport, materialise-flash after.
pub const WARP_CHARGE: i32 = 84;
pub const WARP_ARRIVE: i32 = 22;

/// Frames left of the landing flash + shockwave (set as the teleport fires).
#[derive(Resource, Default)]
pub struct WarpArrive(pub i32);

pub struct FluteState {
    pub phase: Phase,
    pub t: i64,
    pub seq: String,
    pub glow: [i32; 4],
    pub song: Option<&'static SongDef>,
    /// A learned song whose notes are all played but whose LAST note is still held — it
    /// casts on the RELEASE of that note, not its press (Baz), so a phrase resolves when you
    /// let the final note ring out.
    pub armed: Option<&'static SongDef>,
    pub ri: usize,
    pub rt: i32,
    pub flash: i32,
    pub motes: Vec<Mote>,
    pub sparks: Vec<Spark>,
    /// Which note is being HELD right now (js fluteVoice, monophonic — a press steals
    /// the rest). app/audio's flute_hold_tick reads this and keeps the looping voice
    /// alive for exactly as long as the flag stands.
    pub held: [bool; 4],
    pub dests: Vec<Dest>,
    pub di: usize,
    /// The warp channel (Phase::Warp): clock, accelerating spin, the hp that arms
    /// the damage-cancel, and where the song is carrying you.
    pub wt: i32,
    pub wspin: f32,
    pub whp: i32,
    pub wdest: (i32, i32),
    pub whome: bool,
}

/// Play-mode in flight (js fluting) — the player is rooted while the flute is up.
#[derive(Resource, Default)]
pub struct Fluting(pub Option<FluteState>);

/// The Bell Canticle's one-blast attack entity: despawns after its frames run out.
#[derive(Component)]
pub struct FluteFx(pub u32);

/// "Carry me to (rx, ry)" — handled by the loader (Song of Returning). Written when
/// the warp CHARGE completes, not when the destination is picked.
#[derive(Message)]
pub struct WarpTo {
    pub rx: i32,
    pub ry: i32,
    /// Land at the house doorstep (js placeAtHouseDoor), not the room centre.
    pub home: bool,
}

/// Teach a song (js learnSong): needs an instrument in the bag; toasts + dedups.
pub fn learn_song(
    learned: &mut LearnedSongs,
    inv: &PlayerInv,
    log: &mut super::rewards::LootLog,
    id: &str,
    silent: bool,
) -> bool {
    let Some(s) = songs::get(id) else { return false };
    if learned.0.contains(s.id) || !inv.has_item("flute") {
        return false;
    }
    learned.0.insert(s.id);
    if !silent {
        log.add("song", &format!("SONG LEARNED: {}", s.name), 1, 0xd8b8ff, false, true);
    }
    true
}

/// Getting a flute back-teaches every songbook you'd already read (js
/// catchUpSongsFromBooks) — watches the bag so ANY acquisition path counts.
fn catch_up_tick(
    inv: Res<PlayerInv>,
    gathered: Res<super::gather::GatherState>,
    mut learned: ResMut<LearnedSongs>,
    mut log: ResMut<super::rewards::LootLog>,
    mut had: Local<bool>,
) {
    let has = inv.has_item("flute");
    if has && !*had {
        for id in gathered.tomes.iter() {
            if let Some(b) = crate::lore_books::get(id)
                && let Some(song) = b.teaches
            {
                learn_song(&mut learned, &inv, &mut log, song, false);
            }
        }
    }
    *had = has;
}

/// Mana seeps back over time (js: +2 into the accumulator, 80 -> one point).
/// Worn gear folds in here: maxmana raises the pool, manaregen feeds the trickle
/// (Salt Crown / Stillwater Pearl).
fn mana_regen(mut mana: ResMut<Mana>, inv: Res<crate::inventory::PlayerInv>) {
    let gear_max = crate::items::gear_stat(&inv, "maxmana").round() as i32;
    mana.max = (MANA_BASE + gear_max).max(1);
    if mana.flash > 0 {
        mana.flash -= 1;
    }
    if mana.cast_cool > 0 {
        mana.cast_cool -= 1; // a fresh cast holds the trickle (js castCool)
        return;
    }
    if mana.cur >= mana.max {
        mana.cur = mana.cur.min(mana.max);
        mana.accum = 0;
        return;
    }
    mana.accum += 2 + crate::items::gear_stat(&inv, "manaregen").round() as i32;
    if mana.accum >= 80 {
        mana.accum -= 80;
        mana.cur = (mana.cur + 1).min(mana.max);
    }
}

/// The Canticle's blast ring lives 2 ticks, then leaves.
fn flute_fx_tick(mut commands: Commands, mut q: Query<(Entity, &mut FluteFx)>) {
    for (e, mut fx) in &mut q {
        if fx.0 == 0 {
            commands.entity(e).despawn();
        } else {
            fx.0 -= 1;
        }
    }
}

/// A struck sleeper wakes instantly (js: any hit clears the sleep status).
fn wake_on_hit(mut hits: MessageReader<HitLanded>, mut mobs: Query<&mut Mob>) {
    for hit in hits.read() {
        if let Ok(mut m) = mobs.get_mut(hit.target) {
            m.sleep = 0;
        }
    }
}

/// The world/cast context (grouped under the 16-param cap).
#[derive(SystemParam)]
pub struct FluteCtx<'w> {
    pub clock: ResMut<'w, FrameClock>,
    pub weather: ResMut<'w, super::weather::WeatherState>,
    pub farm: ResMut<'w, super::farm::FarmTiles>,
    pub farm_dirty: ResMut<'w, super::farm::FarmDirty>,
    pub town_names: Res<'w, super::banners::TownNames>,
    pub cur: Res<'w, CurRoom>,
    pub world: Res<'w, GameWorld>,
    pub inside: Res<'w, Inside>,
    pub in_dungeon: Res<'w, InDungeon>,
    pub sliding: Res<'w, SlideActive>,
    pub mana: ResMut<'w, Mana>,
    pub statuses: ResMut<'w, super::status::Statuses>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub sfx: MessageWriter<'w, super::sfx::Sfx>,
    pub warps: MessageWriter<'w, WarpTo>,
    /// FluteCtx sits AT the 16-field cap — new resources nest here (the SocialCtx idiom).
    pub extra: FluteExtra<'w>,
}

/// The overflow slice of the flute's context (FluteCtx is at Bevy's 16-field cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct FluteExtra<'w> {
    /// The Song of Opening rings out — caves.rs answers.
    pub openings: MessageWriter<'w, super::caves::OpeningSung>,
    /// The Song of Returning's HOME entry needs to know where home is.
    pub house: Res<'w, super::home::PlayerHouse>,
    /// The landing flash's countdown, armed as the teleport fires.
    pub arrive: ResMut<'w, WarpArrive>,
}

const NOTE_ACTS: [Action; 4] = [Action::Up, Action::Down, Action::Left, Action::Right];

/// The whole play-mode state machine (js updateFlute), one fixed tick at a time.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn flute_tick(
    mut commands: Commands,
    mut input: ResMut<ActionState>,
    mut fluting: ResMut<Fluting>,
    mut ctx: FluteCtx,
    learned: Res<LearnedSongs>,
    inv: Res<PlayerInv>,
    fishing: Res<super::fishing::Fishing>,
    mut players: Query<(&mut Player, &mut Health)>,
    mut mobs: Query<&mut Mob>,
    ptr: Res<crate::input::Pointer>,
) {
    let Ok((mut p, mut health)) = players.single_mut() else { return };

    // --- No tune in flight: does a flute-slot press raise it? (the rod's pattern) ---
    if fluting.0.is_none() {
        for (i, action) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
            if input.pressed(action)
                && inv.slots[i].and_then(|uid| inv.id_of(uid)) == Some("flute")
                && p.cooldowns[i] == 0
            {
                input.consume(action);
                p.cooldowns[i] = 18;
                if ctx.inside.0.is_some() {
                    ctx.log.add("song", "NOT THE PLACE FOR A TUNE", 1, 0xb8a0d8, false, true);
                    ctx.sfx.write(super::sfx::Sfx("tink"));
                } else if fishing.0.is_none() && !ctx.sliding.0 {
                    fluting.0 = Some(FluteState {
                        phase: Phase::Play,
                        t: 0,
                        seq: String::new(),
                        glow: [0; 4],
                        song: None,
                        armed: None,
                        ri: 0,
                        rt: 0,
                        flash: 0,
                        motes: vec![],
                        sparks: vec![],
                        held: [false; 4],
                        dests: vec![],
                        di: 0,
                        wt: 0,
                        wspin: 0.0,
                        whp: 0,
                        wdest: (0, 0),
                        whome: false,
                    });
                    ctx.sfx.write(super::sfx::Sfx("open"));
                }
                break;
            }
        }
        return;
    }

    // Take-run: work on the OWNED state (casts + closes reassign fluting.0 freely).
    let mut f = fluting.0.take().expect("checked above");
    let mut closed = false;
    let mut cast: Option<&'static SongDef> = None;
    f.t += 1;
    if f.flash > 0 {
        f.flash -= 1;
    }
    for g in &mut f.glow {
        if *g > 0 {
            *g -= 1; // the compass arrows cool back down
        }
    }
    for m in &mut f.motes {
        m.t += 1;
    }
    f.motes.retain(|m| m.t < 40);
    // Sparks drift, slow, and curl upward as they die — music rises.
    for s in &mut f.sparks {
        s.t += 1;
        s.x += s.vx;
        s.y += s.vy;
        s.vx *= 0.93;
        s.vy = s.vy * 0.93 - 0.03;
    }
    f.sparks.retain(|s| s.t < s.life);
    // The rose's centre in play-field coords (the overlay anchors there too) and the
    // seeded scatter hash — the motes' no-RNG idiom, widened.
    let (rcx, rcy) = ((PX_W as f32 / 2.0).round(), PX_H as f32 - 34.0);
    let hash = |k: i64| ((k.wrapping_mul(2654435761) >> 7) & 1023) as f32 / 1023.0;

    match f.phase {
        Phase::Play => {
            // The staff SHIMMERS: a parchment fleck lifts off it every few beats.
            if f.t % 5 == 0 {
                let sx = rcx - 108.0 + hash(f.t) * 216.0;
                f.sparks.push(Spark {
                    x: sx,
                    y: rcy - 10.0 + hash(f.t + 3) * 20.0,
                    vx: (hash(f.t + 7) - 0.5) * 0.2,
                    vy: -0.15 - hash(f.t + 11) * 0.2,
                    t: 0,
                    life: 34,
                    col: 0xd8d6be,
                });
            }
            if input.pressed(Action::Slot2) {
                input.consume(Action::Slot2);
                closed = true; // lower the flute
                ctx.sfx.write(super::sfx::Sfx("open"));
            }
            for (i, act) in NOTE_ACTS.into_iter().enumerate() {
                if closed {
                    break;
                }
                if !input.pressed(act) {
                    continue;
                }
                let n = &songs::NOTES[i];
                // The live voice is js noteOn/noteOff: the press raises the HELD flag
                // (stealing any other note — monophonic) and app/audio's flute_hold_tick
                // sounds the looping tone + breath chiff. No one-shot here: that voice
                // holds for as long as the arrow does. (Replay still uses the one-shots,
                // the js note() split.)
                f.held = [false; 4];
                f.held[i] = true;
                f.glow[i] = 12;
                // The press FANS SPARKS off that note's rose tip, in its colour.
                let (tox, toy): (f32, f32) = match n.ltr {
                    'U' => (0.0, -14.0),
                    'D' => (0.0, 14.0),
                    'L' => (-14.0, 0.0),
                    _ => (14.0, 0.0),
                };
                for j in 0..9i64 {
                    let k = f.t.wrapping_mul(31).wrapping_add(j * 97 + i as i64);
                    let ang = toy.atan2(tox) + (hash(k) - 0.5) * 1.4;
                    let sp = 0.8 + hash(k + 13) * 1.6;
                    f.sparks.push(Spark {
                        x: rcx + tox,
                        y: rcy + toy,
                        vx: ang.cos() * sp,
                        vy: ang.sin() * sp,
                        t: 0,
                        life: 16 + (hash(k + 29) * 14.0) as i32,
                        col: if hash(k + 41) < 0.25 { 0xffffff } else { n.col },
                    });
                }
                f.seq.push(n.ltr);
                if f.seq.len() > 10 {
                    f.seq = f.seq.split_off(f.seq.len() - 10);
                }
                f.motes.push(Mote {
                    x: p.x + 3.0 + ((f.t * 37) % 10) as f32, // seeded scatter, no RNG needed
                    y: p.y - 4.0,
                    t: 0,
                    ltr: n.ltr,
                    col: n.col,
                });
                f.armed = None; // a fresh note re-opens the phrase (steals any prior arm)
                if let Some(hit) = songs::match_tail(&f.seq) {
                    if learned.0.contains(hit.id) {
                        // Every note is played — but the melody only CATCHES on the RELEASE of
                        // this final note (Baz: cast on the up, not the down). Arm it and let
                        // it ring; the release pass below fires the catch when you let go.
                        f.armed = Some(hit);
                    } else {
                        // A real melody you were never taught — it won't take.
                        f.seq.clear();
                        ctx.log.add("song", "YOU HAVENT LEARNED THAT SONG", 1, 0xb8a0d8, false, true);
                        ctx.sfx.write(super::sfx::Sfx("tink"));
                    }
                }
                break; // one note a tick
            }
            // HELD notes SUSTAIN (js noteOn/noteOff): the LOOPING tone lives in
            // app/audio's flute_hold_tick, keyed off f.held — one continuous voice,
            // not re-triggered one-shots. Here: letting go releases the flag (the
            // voice breath-offs), holding keeps the glow warm, motes rising and the
            // tip trickling sparks. The melody took its letter at the press.
            if closed {
                f.held = [false; 4];
            } else {
                for (i, act) in NOTE_ACTS.into_iter().enumerate() {
                    if !input.held(act) {
                        f.held[i] = false;
                        continue;
                    }
                    if !f.held[i] {
                        continue; // held key without a live voice (stolen by a newer note)
                    }
                    f.glow[i] = f.glow[i].max(6); // lit for as long as you hold
                    if f.t % 14 != 0 {
                        continue;
                    }
                    let n = &songs::NOTES[i];
                    f.motes.push(Mote {
                        x: p.x + 3.0 + ((f.t * 37) % 10) as f32,
                        y: p.y - 4.0,
                        t: 0,
                        ltr: n.ltr,
                        col: n.col,
                    });
                    // A lighter trickle off the tip than the press fan.
                    let (tox, toy): (f32, f32) = match n.ltr {
                        'U' => (0.0, -14.0),
                        'D' => (0.0, 14.0),
                        'L' => (-14.0, 0.0),
                        _ => (14.0, 0.0),
                    };
                    for j in 0..4i64 {
                        let k = f.t.wrapping_mul(29).wrapping_add(j * 83 + i as i64);
                        let ang = toy.atan2(tox) + (hash(k) - 0.5) * 1.4;
                        let sp = 0.6 + hash(k + 13) * 1.2;
                        f.sparks.push(Spark {
                            x: rcx + tox,
                            y: rcy + toy,
                            vx: ang.cos() * sp,
                            vy: ang.sin() * sp,
                            t: 0,
                            life: 14 + (hash(k + 29) * 12.0) as i32,
                            col: if hash(k + 41) < 0.25 { 0xffffff } else { n.col },
                        });
                    }
                }
            }
            // The armed melody CATCHES on RELEASE: once every note key is up — you let the
            // final note ring out and let go — it sings itself back + casts (Baz: cast on the
            // up of the last note, not the down).
            if !closed
                && !NOTE_ACTS.iter().any(|&a| input.held(a))
                && let Some(hit) = f.armed.take()
            {
                f.phase = Phase::Replay;
                f.song = Some(hit);
                f.ri = 0;
                f.rt = 24;
                f.seq.clear();
                f.flash = 22;
                f.held = [false; 4]; // the replay's voice takes over
                // The catch ERUPTS: a slow ring of all four note colours + white off the rose.
                for j in 0..26usize {
                    let ang = j as f32 / 26.0 * std::f32::consts::TAU;
                    let sp = 1.2 + ((j * 7) % 5) as f32 * 0.25;
                    let col = if j % 5 == 4 { 0xffffff } else { songs::NOTES[j % 4].col };
                    f.sparks.push(Spark { x: rcx, y: rcy, vx: ang.cos() * sp, vy: ang.sin() * sp, t: 0, life: 26, col });
                }
                ctx.sfx.write(super::sfx::Sfx("songmatch"));
            }
        }
        Phase::Replay => {
            f.rt -= 1;
            if f.rt <= 0 {
                let song = f.song.expect("replay always carries its song");
                if f.ri < song.notes.len() {
                let ltr = song.notes.as_bytes()[f.ri] as char;
                let i = songs::note_idx(ltr);
                ctx.sfx.write(super::sfx::Sfx(match i {
                    0 => "noteU",
                    1 => "noteD",
                    2 => "noteL",
                    _ => "noteR",
                }));
                f.glow[i] = 12;
                f.motes.push(Mote {
                    x: p.x + 3.0 + ((f.t * 53) % 10) as f32,
                    y: p.y - 6.0,
                    t: 0,
                    ltr,
                    col: songs::NOTES[i].col,
                });
                    f.ri += 1;
                    f.rt = if f.ri >= song.notes.len() { 16 } else { 11 };
                } else {
                    cast = Some(song);
                }
            }
        }
        Phase::Dest => {
            if input.pressed(Action::Slot2) {
                input.consume(Action::Slot2);
                closed = true;
                ctx.sfx.write(super::sfx::Sfx("open"));
            }
            let n = f.dests.len();
            if n == 0 {
                closed = true;
            }
            if !closed && input.pressed(Action::Up) {
                f.di = (f.di + n - 1) % n;
                ctx.sfx.write(super::sfx::Sfx("menuMove"));
            }
            if !closed && input.pressed(Action::Down) {
                f.di = (f.di + 1) % n;
                ctx.sfx.write(super::sfx::Sfx("menuMove"));
            }
            // Mouse: the list SCROLLS, so hover does nothing — a click selects a
            // destination, clicking the selection warps there.
            let mut dest_click = false;
            if !closed && input_click(&ptr) {
                use super::room_render::{PLAY_X, PLAY_Y};
                use crate::room::{PX_H, PX_W};
                let vis = n.min(6);
                let start = f.di.saturating_sub(2).min(n.saturating_sub(vis));
                let (pw, ph) = (160.0, 34.0 + vis as f32 * 10.0);
                let bx = (PLAY_X + PX_W as f32 / 2.0 - pw / 2.0).round();
                let by = (PLAY_Y + PX_H as f32 / 2.0 - ph / 2.0).round();
                for i in 0..vis {
                    if ptr.over(bx + 4.0, by + 19.0 + i as f32 * 10.0 - 2.0, pw - 8.0, 9.0) {
                        if f.di != start + i {
                            f.di = start + i;
                            ctx.sfx.write(super::sfx::Sfx("menuMove"));
                        } else {
                            dest_click = true;
                        }
                    }
                }
            }
            if !closed && (input.pressed(Action::Slot1) || input.pressed(Action::Interact) || dest_click) {
                input.consume(Action::Slot1);
                input.consume(Action::Interact);
                let d = &f.dests[f.di];
                ctx.mana.cur -= songs::get("returning").map_or(10, |s| s.mana);
                // The js WARP CHARGE: the song holds you rooted in the LIVE world while
                // the rings spin up — the teleport fires only if the channel completes.
                f.wdest = (d.rx, d.ry);
                f.whome = d.home;
                f.wt = 0;
                f.wspin = 0.0;
                f.whp = health.hp;
                f.phase = Phase::Warp;
                ctx.sfx.write(super::sfx::Sfx("warpCharge"));
            }
        }
        Phase::Warp => {
            f.wt += 1;
            f.wspin += 0.14 + f.wt as f32 * 0.008; // accelerating spin (js)
            if f.wt % 16 == 0 {
                ctx.sfx.write(super::sfx::Sfx("warpTick"));
            }
            if health.hp < f.whp {
                // took damage -> the song breaks (js cancelWarp)
                ctx.log.add("song", "THE SONG BREAKS", 1, 0xfc8868, false, true);
                ctx.sfx.write(super::sfx::Sfx("warpFail"));
                closed = true;
            } else if f.wt >= WARP_CHARGE {
                ctx.warps.write(WarpTo { rx: f.wdest.0, ry: f.wdest.1, home: f.whome });
                ctx.sfx.write(super::sfx::Sfx("warpGo"));
                ctx.extra.arrive.0 = WARP_ARRIVE;
                closed = true;
            }
        }
    }
    if !closed {
        fluting.0 = Some(f); // still up (a cast may still lower it below)
    }
    if let Some(song) = cast {
        cast_song(song, &mut commands, &mut fluting, &mut ctx, &mut p, &mut health, &mut mobs);
    }
}

/// The cast branches (js castSong) — anywhere-songs first, then the overworld-only set.
fn cast_song(
    song: &'static SongDef,
    commands: &mut Commands,
    fluting: &mut Fluting,
    ctx: &mut FluteCtx,
    p: &mut Player,
    _health: &mut Health,
    mobs: &mut Query<&mut Mob>,
) {
    if ctx.mana.cur < song.mana {
        ctx.log.add("song", &format!("TOO WEARY TO FINISH IT - NEED {} MANA", song.mana), 1, 0xfc8868, false, true);
        ctx.sfx.write(super::sfx::Sfx("warpFail"));
        fluting.0 = None;
        return;
    }
    let (px, py) = (p.x + 8.0, p.y + 9.0);
    match song.id {
        // The combat song — works ANYWHERE.
        "canticle" => {
            ctx.mana.cur -= song.mana;
            fluting.0 = None;
            // One tolling blast through the normal combat machinery: a short-lived
            // 104px ring that hits + shoves everything around you.
            commands.spawn((
                Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(1), persistent: false, knock: 2.5 },
                Hitbox { x: px - 52.0, y: py - 52.0, w: 104.0, h: 104.0 },
                FluteFx(2),
                RoomActor,
            ));
            ctx.sfx.write(super::sfx::Sfx("bellring"));
        }
        "wardsong" => {
            ctx.mana.cur -= song.mana;
            fluting.0 = None;
            ctx.statuses.add("ward", 600); // +2 defense rides the status system now
            ctx.log.add("song", "A WARDING HUM SETTLES OVER YOU", 1, 0x7fb0e0, false, true);
            ctx.sfx.write(super::sfx::Sfx("cast"));
        }
        "lullaby" => {
            ctx.mana.cur -= song.mana;
            fluting.0 = None;
            let mut n = 0;
            for mut m in mobs.iter_mut() {
                let d = crate::actors::mobs::MOB_DEFS[m.def].kind;
                if d == "golem" {
                    continue; // stone doesn't dream (js applyStatus immunity)
                }
                let (dx, dy) = ((m.x + 8.0) - px, (m.y + 8.0) - py);
                if dx * dx + dy * dy <= 52.0 * 52.0 {
                    m.sleep = 300;
                    n += 1;
                }
            }
            let msg = if n > 0 { "A HUSH FALLS - THEY SLEEP" } else { "THE HUSH FINDS NO ONE TO STILL" };
            ctx.log.add("song", msg, 1, 0xa894e0, false, true);
            ctx.sfx.write(super::sfx::Sfx(if n > 0 { "sleep" } else { "menuMove" }));
        }
        _ if ctx.inside.0.is_some() || ctx.in_dungeon.0.is_some() => {
            ctx.log.add("song", "THE SONG ECHOES STRANGELY DOWN HERE", 1, 0xb8a0d8, false, true);
            ctx.sfx.write(super::sfx::Sfx("warpFail"));
            fluting.0 = None;
        }
        "returning" => {
            // No mana spent yet — that happens when you pick a destination.
            let mut dests: Vec<Dest> = ctx
                .town_names
                .0
                .iter()
                .filter_map(|(key, name)| {
                    let (rx, ry) = key.split_once(',').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))?;
                    if (rx, ry) == (ctx.cur.rx, ctx.cur.ry) {
                        return None;
                    }
                    let dist = (((rx - ctx.cur.rx).pow(2) + (ry - ctx.cur.ry).pow(2)) as f64).sqrt().round() as i32;
                    Some(Dest { name: name.to_uppercase(), rx, ry, dist, home: false })
                })
                .collect();
            dests.sort_by_key(|d| d.dist);
            // HOME leads the list when you have one (Baz) — the hearth outranks every town.
            if let Some(h) = ctx.extra.house.0.as_ref().filter(|h| h.room != (ctx.cur.rx, ctx.cur.ry)) {
                let dist =
                    ((((h.room.0 - ctx.cur.rx).pow(2) + (h.room.1 - ctx.cur.ry).pow(2)) as f64).sqrt()).round() as i32;
                dests.insert(0, Dest { name: "HOME".into(), rx: h.room.0, ry: h.room.1, dist, home: true });
            }
            if dests.is_empty() {
                ctx.log.add("song", "NOWHERE CALLS YOU BACK YET", 1, 0xb8a0d8, false, true);
                ctx.sfx.write(super::sfx::Sfx("warpFail"));
                fluting.0 = None;
                return;
            }
            if let Some(f) = &mut fluting.0 {
                f.phase = Phase::Dest;
                f.dests = dests;
                f.di = 0;
            }
            ctx.sfx.write(super::sfx::Sfx("menuConfirm"));
        }
        "stormcall" => {
            ctx.mana.cur -= song.mana;
            fluting.0 = None;
            let cur = ctx.weather.cur;
            let wet = matches!(cur, "rain" | "thunderstorm" | "snow" | "blizzard" | "sandstorm");
            if wet {
                ctx.weather.command("clear", super::gather::DAY_LEN / 2);
                ctx.log.add("song", "THE SKY CLEARS", 1, 0x9ad0ff, false, true);
            } else {
                let biome = ctx.world.0.biome_key_at(ctx.cur.rx, ctx.cur.ry);
                let w = crate::weather::precip_for(biome);
                ctx.weather.command(w, super::gather::DAY_LEN / 2);
                let msg = match w {
                    "snow" => "SNOW DRIFTS DOWN",
                    "sandstorm" => "THE SANDS RISE TO DANCE",
                    "overcast" => "CLOUDS GATHER - NO RAIN FALLS HERE",
                    _ => "RAIN COMES TO YOUR CALL",
                };
                ctx.log.add("song", msg, 1, 0x7090d8, false, true);
            }
            ctx.sfx.write(super::sfx::Sfx("thunder"));
        }
        "sunsong" => {
            ctx.mana.cur -= song.mana;
            fluting.0 = None;
            if super::lighting::day_darkness(ctx.clock.0) > 0.5 {
                // Night -> the next morning.
                let day_len = super::gather::DAY_LEN;
                ctx.clock.0 = (ctx.clock.0 + 1).div_euclid(day_len) * day_len + day_len;
                ctx.log.add("song", "DAWN BREAKS", 1, 0xfcd23b, false, true);
                ctx.sfx.write(super::sfx::Sfx("wake"));
            } else {
                // Day -> nightfall (js: 0.4 of the day past noon-zero = dusk).
                let day_len = super::gather::DAY_LEN;
                let base = ctx.clock.0.div_euclid(day_len) * day_len + (day_len as f64 * 0.4).round() as i64;
                ctx.clock.0 = if base > ctx.clock.0 { base } else { base + day_len };
                ctx.log.add("song", "NIGHT FALLS", 1, 0x9ab0e0, false, true);
                ctx.sfx.write(super::sfx::Sfx("sleep"));
            }
        }
        "opening" => {
            // The stones are caves.rs business: it answers with a split stone (mana
            // spent only if something answers) or lets the notes fade.
            fluting.0 = None;
            ctx.extra.openings.write(super::caves::OpeningSung { mana: song.mana });
        }
        "greensong" => {
            let n = ctx.farm.ripen_room((ctx.cur.rx, ctx.cur.ry));
            fluting.0 = None;
            if n == 0 {
                ctx.log.add("song", "NOTHING SOWN HERE ANSWERS", 1, 0xb8a0d8, false, true);
                ctx.sfx.write(super::sfx::Sfx("warpFail"));
                return;
            }
            ctx.mana.cur -= song.mana;
            ctx.farm_dirty.0 = true;
            let msg = if n == 1 { "A CROP LEAPS TO FRUIT".to_string() } else { format!("THE FIELDS LEAP TO FRUIT ({n})") };
            ctx.log.add("song", &msg, 1, 0x7ad86a, false, true);
            ctx.sfx.write(super::sfx::Sfx("itemget"));
        }
        _ => {
            fluting.0 = None;
        }
    }
}

// --- The overlay: motes, the note compass, the catch flash, the song banner and the
// Returning destination picker. Rebuilt each frame while the flute is up (a handful of
// tinted sprites off four cached arrow bakes — no per-frame image churn). ---

#[derive(Component)]
struct FluteUi;

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn flute_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    fluting: Res<Fluting>,
    bindings: Res<Bindings>,
    input: Res<ActionState>,
    players: Query<&Player>,
    old: Query<Entity, With<FluteUi>>,
    mut arrows: Local<Option<[Handle<Image>; 4]>>,
    mut edges: Local<Option<[Handle<Image>; 4]>>,
    mut rose_img: Local<Option<Handle<Image>>>,
    mut halo_img: Local<Option<Handle<Image>>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    // The rose is screen-anchored (bottom-centre), so we only need to know a player exists.
    let (Some(f), Ok(_)) = (&fluting.0, players.single()) else { return };
    // Four white arrow bakes (U D L R), tinted per use.
    let arrows = arrows.get_or_insert_with(|| {
        let mk = |ltr: char, images: &mut Assets<Image>| {
            use bevy::asset::RenderAssetUsages;
            use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
            let mut img = Image::new_fill(
                Extent3d { width: 7, height: 7, depth_or_array_layers: 1 },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            for (x, y) in songs::arrow_cells(ltr) {
                if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) {
                    px.copy_from_slice(&[255, 255, 255, 255]);
                }
            }
            images.add(img)
        };
        [mk('U', &mut images), mk('D', &mut images), mk('L', &mut images), mk('R', &mut images)]
    });
    // The matching 1px-dilated masks — the js drawArrow `edge` outline, so every arrow
    // reads on any background (staff lines, grass, the banner).
    let edges = edges.get_or_insert_with(|| {
        let mk = |ltr: char, images: &mut Assets<Image>| {
            use bevy::asset::RenderAssetUsages;
            use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
            let mut img = Image::new_fill(
                Extent3d { width: 9, height: 9, depth_or_array_layers: 1 },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            for (x, y) in songs::arrow_cells(ltr) {
                for dy in 0..3i32 {
                    for dx in 0..3i32 {
                        if let Ok(px) = img.pixel_bytes_mut(UVec3::new((x + dx) as u32, (y + dy) as u32, 0)) {
                            px.copy_from_slice(&[255, 255, 255, 255]);
                        }
                    }
                }
            }
            images.add(img)
        };
        [mk('U', &mut images), mk('D', &mut images), mk('L', &mut images), mk('R', &mut images)]
    });
    let tint = |c: u32, a: f32| {
        Color::srgba((c >> 16 & 255) as f32 / 255.0, (c >> 8 & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, a)
    };
    let arrow = |commands: &mut Commands, ltr: char, x: f32, y: f32, col: u32, a: f32, z: f32| {
        let mut e = Sprite::from_image(edges[songs::note_idx(ltr)].clone());
        e.color = tint(0x0a0c12, a);
        commands.spawn((e, at(x.round() - 1.0, y.round() - 1.0, 9.0, 9.0, z - 0.003), PIXEL_LAYER, FluteUi));
        let mut s = Sprite::from_image(arrows[songs::note_idx(ltr)].clone());
        s.color = tint(col, a);
        commands.spawn((s, at(x.round(), y.round(), 7.0, 7.0, z), PIXEL_LAYER, FluteUi));
    };

    // --- The bottom-centre COMPASS ROSE over its sheet-music staff (js drawFlute
    // verbatim): the baked backdrop = fading dark band + five staff lines + measure
    // bars + the gold-edged diamond; live sprites = note studs, press ripples, the
    // four arrows, the breathing centre gem, and the played tail written at pitch. ---
    if f.phase == Phase::Play {
        let rcx = PLAY_X + (PX_W as f32 / 2.0).round();
        let rcy = PLAY_Y + PX_H as f32 - 34.0;
        let rose = rose_img.get_or_insert_with(|| {
            use bevy::asset::RenderAssetUsages;
            use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
            const W: i32 = 240; // band width (STW 108 + 12 fade each side, both sides)
            const H: i32 = 55; // diamond tip to tip (27 up + 27 down)
            let (cx, cy) = (120i32, 27i32);
            let mut buf = vec![[0f32; 4]; (W * H) as usize];
            let mut blend = |x: i32, y: i32, c: (f32, f32, f32), a: f32| {
                if !(0..W).contains(&x) || !(0..H).contains(&y) || a <= 0.0 {
                    return;
                }
                let px = &mut buf[(y * W + x) as usize];
                let na = a + px[3] * (1.0 - a);
                if na > 0.0 {
                    px[0] = (c.0 * a + px[0] * px[3] * (1.0 - a)) / na;
                    px[1] = (c.1 * a + px[1] * px[3] * (1.0 - a)) / na;
                    px[2] = (c.2 * a + px[2] * px[3] * (1.0 - a)) / na;
                }
                px[3] = na;
            };
            let dark = (6.0 / 255.0, 8.0 / 255.0, 14.0 / 255.0);
            let parch = (216.0 / 255.0, 214.0 / 255.0, 190.0 / 255.0);
            // The readability band, fading out at both ends (js linear gradient).
            for y in (cy - 16)..(cy + 17) {
                for x in 0..W {
                    let t = x as f32 / (W - 1) as f32;
                    let a = if t < 0.15 {
                        0.55 * t / 0.15
                    } else if t > 0.85 {
                        0.55 * (1.0 - t) / 0.15
                    } else {
                        0.55
                    };
                    blend(x, y, dark, a);
                }
            }
            // Five staff lines with their own end-fade, then the measure bars.
            for i in 0..5 {
                let y = cy - 10 + i * 5;
                for x in 12..228 {
                    let u = (x - 12) as f32 / 215.0;
                    let a = if u < 0.16 {
                        0.4 * u / 0.16
                    } else if u > 0.84 {
                        0.4 * (1.0 - u) / 0.16
                    } else {
                        0.4
                    };
                    blend(x, y, parch, a);
                }
            }
            for mx in [-78i32, -44, 44, 78] {
                for y in (cy - 10)..(cy + 11) {
                    blend(cx + mx, y, parch, 0.25);
                }
            }
            // The rose diamond: dark fill, soft gold stroke (canvas 23.5 stroke ~ two rings).
            let gold = (232.0 / 255.0, 200.0 / 255.0, 96.0 / 255.0);
            for y in 0..H {
                for x in 0..W {
                    let d = (x - cx).abs() + (y - cy).abs();
                    if d <= 27 {
                        blend(x, y, dark, 0.78);
                    }
                    if d == 23 {
                        blend(x, y, gold, 0.45);
                    } else if d == 24 {
                        blend(x, y, gold, 0.25);
                    }
                }
            }
            let mut img = Image::new_fill(
                Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            for y in 0..H {
                for x in 0..W {
                    let p = buf[(y * W + x) as usize];
                    if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) {
                        px.copy_from_slice(&[
                            (p[0] * 255.0) as u8,
                            (p[1] * 255.0) as u8,
                            (p[2] * 255.0) as u8,
                            (p[3] * 255.0) as u8,
                        ]);
                    }
                }
            }
            images.add(img)
        });
        commands.spawn((
            Sprite::from_image(rose.clone()),
            at(rcx - 120.0, rcy - 27.0, 240.0, 55.0, layers::FLUTE_UI - 0.02),
            PIXEL_LAYER,
            FluteUi,
        ));
        // The rose GLOWS (past-js flourish): a soft radial halo breathing behind the
        // diamond — warm gold at rest, flushing to the ringing note's colour, flaring
        // wide and bright on a song catch.
        let halo = halo_img.get_or_insert_with(|| {
            use bevy::asset::RenderAssetUsages;
            use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
            let mut img = Image::new_fill(
                Extent3d { width: 96, height: 96, depth_or_array_layers: 1 },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            for y in 0..96u32 {
                for x in 0..96u32 {
                    let d = ((x as f32 - 47.5).powi(2) + (y as f32 - 47.5).powi(2)).sqrt();
                    let a = (1.0 - d / 48.0).max(0.0);
                    if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                        px.copy_from_slice(&[255, 255, 255, (a * a * 0.85 * 255.0) as u8]);
                    }
                }
            }
            images.add(img)
        });
        let pulse = 0.5 + 0.5 * (f.t as f32 * 0.1).sin();
        let gmax = f.glow.iter().copied().max().unwrap_or(0);
        let (hcol, ha) = if gmax > 0 {
            let ring = f.glow.iter().position(|g| *g == gmax).unwrap_or(0);
            (songs::NOTES[ring].col, 0.14 + 0.18 * gmax as f32 / 12.0)
        } else {
            (0xe8c860, 0.09 + 0.06 * pulse)
        };
        let mut h = Sprite::from_image(halo.clone());
        h.color = tint(hcol, ha);
        commands.spawn((h, at(rcx - 48.0, rcy - 48.0, 96.0, 96.0, layers::FLUTE_UI - 0.03), PIXEL_LAYER, FluteUi));
        if f.flash > 0 {
            // The catch flare: the halo swells past the staff for the flash's beat.
            let fa = (f.flash as f32 / 22.0) * 0.4;
            let mut fh = Sprite::from_image(halo.clone());
            fh.color = tint(0xfff0c0, fa);
            fh.custom_size = Some(Vec2::splat(170.0));
            commands.spawn((fh, at(rcx - 85.0, rcy - 85.0, 170.0, 170.0, layers::FLUTE_UI - 0.035), PIXEL_LAYER, FluteUi));
        }
        // Per-note point: stud at the rose tip, a press ripple, and the arrow itself.
        for (i, n) in songs::NOTES.iter().enumerate() {
            let (ox, oy): (f32, f32) = match n.ltr {
                'U' => (0.0, -14.0),
                'D' => (0.0, 14.0),
                'L' => (-14.0, 0.0),
                _ => (14.0, 0.0),
            };
            let g = f.glow[i];
            commands.spawn((
                Sprite::from_color(tint(n.col, 0.9), Vec2::new(2.0, 2.0)),
                at((rcx + ox * 1.93 - 1.0).round(), (rcy + oy * 1.93 - 1.0).round(), 2.0, 2.0, layers::FLUTE_UI - 0.01),
                PIXEL_LAYER,
                FluteUi,
            ));
            if g > 0 {
                // Press ripple (js: an expanding stroked circle; a square ring here —
                // 12 frames of flash, reads the same).
                let rr = 6.0 + (12 - g) as f32 * 0.9;
                let a = (g as f32 / 12.0) * 0.7;
                for (sx, sy, sw, sh) in crate::ui::border_strips(rcx + ox - rr, rcy + oy - rr, rr * 2.0, rr * 2.0, 1.0) {
                    commands.spawn((
                        Sprite::from_color(tint(n.col, a), Vec2::new(sw, sh)),
                        at(sx.round(), sy.round(), sw, sh, layers::FLUTE_UI - 0.01),
                        PIXEL_LAYER,
                        FluteUi,
                    ));
                }
            }
            let col = if g > 8 { 0xffffff } else { n.col };
            let a = if g > 0 { 1.0 } else { 0.55 };
            arrow(&mut commands, n.ltr, rcx + ox - 3.0, rcy + oy - 3.0, col, a, layers::FLUTE_UI);
        }
        // The centre gem breathes while you play.
        for (gx, gy, gw, gh) in [(rcx - 1.0, rcy - 2.0, 2.0, 4.0), (rcx - 2.0, rcy - 1.0, 4.0, 2.0)] {
            commands.spawn((
                Sprite::from_color(tint(0xe8c860, 1.0), Vec2::new(gw, gh)),
                at(gx, gy, gw, gh, layers::FLUTE_UI + 0.01),
                PIXEL_LAYER,
                FluteUi,
            ));
        }
        commands.spawn((
            Sprite::from_color(tint(0xfff4c0, 0.25 + pulse * 0.3), Vec2::new(2.0, 2.0)),
            at(rcx - 1.0, rcy - 1.0, 2.0, 2.0, layers::FLUTE_UI + 0.02),
            PIXEL_LAYER,
            FluteUi,
        ));
        // The melody so far, written onto the staff at pitch, oldest fading out.
        let all: Vec<char> = f.seq.chars().collect();
        let tail = &all[all.len().saturating_sub(8)..];
        let n = tail.len();
        for (i, ltr) in tail.iter().enumerate() {
            let pitch = match ltr {
                'D' => 0,
                'L' => 1,
                'R' => 2,
                _ => 3, // U rides the top line
            };
            let nx = rcx - 36.0 - (n - 1 - i) as f32 * 9.0;
            let ny = rcy + 6.0 - pitch as f32 * 5.0;
            let a = 0.3 + 0.7 * ((i + 1) as f32 / n as f32);
            arrow(&mut commands, *ltr, nx - 3.0, ny - 3.0, songs::NOTES[songs::note_idx(*ltr)].col, a, layers::FLUTE_UI + 0.01);
        }
        // Bottom-right: the way back down.
        let hint = format!("{} LOWER THE FLUTE", bindings.prompt(Action::Slot2, input.pad_present));
        let (img, w) = font::bake_text(&hint, 0x6a7280, &mut images);
        let iw = (w + (w & 1)) as f32;
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + PX_W as f32 - 6.0 - iw, PLAY_Y + PX_H as f32 - 10.0, iw, 6.0, layers::FLUTE_UI + 0.02),
            PIXEL_LAYER,
            FluteUi,
        ));
    }
    // Played notes drift up off the flute as little coloured arrows.
    for m in &f.motes {
        let sway = (((m.t + m.x as i32) as f32) * 0.25).sin() * 2.0;
        let a = (1.0 - m.t as f32 / 40.0).max(0.0);
        arrow(&mut commands, m.ltr, PLAY_X + m.x + sway, PLAY_Y + m.y - m.t as f32 * 0.55, m.col, a, layers::FLUTE_UI + 0.01);
    }
    // Musical sparks — the press fans, the catch's colour ring, the staff shimmer —
    // twinkling as they fade (they ride through the catch into the replay banner).
    for s in &f.sparks {
        let fade = (1.0 - s.t as f32 / s.life as f32).max(0.0);
        let tw = 0.55 + 0.45 * ((s.t as f32 * 1.1 + s.x * 0.7).sin()).abs();
        let px = if s.col == 0xd8d6be { 1.0 } else { 2.0 }; // shimmer stays fine-grained
        commands.spawn((
            Sprite::from_color(tint(s.col, fade * tw), Vec2::splat(px)),
            at((PLAY_X + s.x).round(), (PLAY_Y + s.y).round(), px, px, layers::FLUTE_UI + 0.03),
            PIXEL_LAYER,
            FluteUi,
        ));
    }
    // The catch flash: a warm wash over the field.
    if f.flash > 0 {
        let a = (f.flash as f32 / 22.0) * 0.3;
        commands.spawn((
            Sprite::from_color(tint(0xffe8b0, a), Vec2::new(PX_W as f32, PX_H as f32)),
            at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, layers::FLUTE_UI - 0.05),
            PIXEL_LAYER,
            FluteUi,
        ));
    }
    // The song banner (replay): the melody announces itself.
    if f.phase == Phase::Replay
        && let Some(song) = f.song
    {
        let bcy = PLAY_Y + 52.0;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.016, 0.024, 0.047, 0.78), Vec2::new(PX_W as f32, 32.0)),
            at(PLAY_X, bcy, PX_W as f32, 32.0, layers::FLUTE_UI),
            PIXEL_LAYER,
            FluteUi,
        ));
        let (img, w) = font::bake_text(song.name, song.col, &mut images);
        let iw = (w + (w & 1)) as f32;
        commands.spawn((
            Sprite::from_image(img),
            at((PLAY_X + PX_W as f32 / 2.0 - iw / 2.0).round(), bcy + 6.0, iw, 6.0, layers::FLUTE_UI + 0.02),
            PIXEL_LAYER,
            FluteUi,
        ));
        // Its notes light one by one as they sing back.
        let total = song.notes.len() as f32;
        let x0 = PLAY_X + PX_W as f32 / 2.0 - (total * 11.0) / 2.0;
        for (i, ltr) in song.notes.chars().enumerate() {
            let lit = i < f.ri;
            let col = if lit { songs::NOTES[songs::note_idx(ltr)].col } else { 0x3a3a44 };
            arrow(&mut commands, ltr, x0 + i as f32 * 11.0, bcy + 17.0, col, if lit { 1.0 } else { 0.6 }, layers::FLUTE_UI + 0.02);
        }
    }
    // The destination picker (Song of Returning).
    if f.phase == Phase::Dest {
        let vis = f.dests.len().min(6);
        let start = (f.di.saturating_sub(2)).min(f.dests.len().saturating_sub(vis));
        let (pw, ph) = (160.0, 34.0 + vis as f32 * 10.0);
        let bx = (PLAY_X + PX_W as f32 / 2.0 - pw / 2.0).round();
        let by = (PLAY_Y + PX_H as f32 / 2.0 - ph / 2.0).round();
        commands.spawn((
            Sprite::from_color(Color::srgba(0.023, 0.03, 0.055, 0.92), Vec2::new(pw, ph)),
            at(bx, by, pw, ph, layers::FLUTE_UI),
            PIXEL_LAYER,
            FluteUi,
        ));
        for (sx, sy, sw, sh) in crate::ui::border_strips(bx, by, pw, ph, 1.0) {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x8a, 0x7a, 0x4a), Vec2::new(sw, sh)),
                at(sx, sy, sw, sh, layers::FLUTE_UI + 0.01),
                PIXEL_LAYER,
                FluteUi,
            ));
        }
        let center = |c: &mut Commands, i: &mut Assets<Image>, t: &str, y: f32, col: u32| {
            let (img, w) = font::bake_text(t, col, i);
            let iw = (w + (w & 1)) as f32;
            c.spawn((
                Sprite::from_image(img),
                at((bx + pw / 2.0 - iw / 2.0).round(), y, iw, 6.0, layers::FLUTE_UI + 0.02),
                PIXEL_LAYER,
                FluteUi,
            ));
        };
        center(&mut commands, &mut images, "THE SONG CARRIES YOU TO...", by + 6.0, 0x9ad0ff);
        for i in 0..vis {
            let d = &f.dests[start + i];
            let sel = start + i == f.di;
            let yy = by + 19.0 + i as f32 * 10.0;
            if sel {
                commands.spawn((
                    Sprite::from_color(Color::srgba(0.6, 0.81, 1.0, 0.16), Vec2::new(pw - 8.0, 9.0)),
                    at(bx + 4.0, yy - 2.0, pw - 8.0, 9.0, layers::FLUTE_UI + 0.01),
                    PIXEL_LAYER,
                    FluteUi,
                ));
            }
            let (img, w) = font::bake_text(&d.name, if sel { 0xfcfcfc } else { 0x9aa0aa }, &mut images);
            let iw = (w + (w & 1)) as f32;
            commands.spawn((
                Sprite::from_image(img),
                at(bx + 14.0, yy, iw, 6.0, layers::FLUTE_UI + 0.02),
                PIXEL_LAYER,
                FluteUi,
            ));
            let dl = format!("{} AWAY", d.dist);
            let (img, w) = font::bake_text(&dl, if sel { 0x8ab0d0 } else { 0x5a6a7a }, &mut images);
            let iw = (w + (w & 1)) as f32;
            commands.spawn((
                Sprite::from_image(img),
                at(bx + pw - 8.0 - iw, yy, iw, 6.0, layers::FLUTE_UI + 0.02),
                PIXEL_LAYER,
                FluteUi,
            ));
        }
        let mana = songs::get("returning").map_or(10, |s| s.mana);
        let hint = format!(
            "{} GO - {} STAY - {mana} MANA",
            bindings.prompt(Action::Slot1, input.pad_present),
            bindings.prompt(Action::Slot2, input.pad_present)
        );
        center(&mut commands, &mut images, &hint, by + ph - 10.0, 0x8a8a92);
    }
}

/// Marker on every warp-fx sprite (rebuilt each frame, immediate-mode).
#[derive(Component)]
struct WarpFxUi;

/// Bake the js radial-gradient glow once: a 64px disc whose alpha falls off with
/// the square of distance (reads like the 'lighter' gradient the js painted).
fn radial_glow(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const S: usize = 64;
    let mut data = vec![0u8; S * S * 4];
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - 31.5, y as f32 - 31.5);
            let a = (1.0 - (dx * dx + dy * dy).sqrt() / 32.0).clamp(0.0, 1.0);
            let i = (y * S + x) * 4;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (a * a * 255.0) as u8;
        }
    }
    images.add(Image::new(
        Extent3d { width: S as u32, height: S as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

/// The js drawWarpFx, sprite by sprite: CHARGE = rising radial glow + two
/// counter-rotating rings of squares tightening in (cyan + arcane purple, squashed
/// into portal perspective) + sparks climbing the column; ARRIVE = a screen flash
/// and an expanding shockwave ring.
#[allow(clippy::too_many_arguments)] // it IS the effect's arity
fn warp_fx(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    fluting: Res<Fluting>,
    arrive: Res<WarpArrive>,
    players: Query<&Player>,
    old: Query<Entity, With<WarpFxUi>>,
    mut glow: Local<Option<Handle<Image>>>,
) {
    use std::f32::consts::TAU;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Ok(p) = players.single() else { return };
    let (cx, cy) = (PLAY_X + p.x + 8.0, PLAY_Y + p.y + 9.0);
    let z = 9.6;
    if let Some(f) = fluting.0.as_ref().filter(|f| f.phase == Phase::Warp) {
        let prog = (f.wt as f32 / WARP_CHARGE as f32).min(1.0);
        let img = glow.get_or_insert_with(|| radial_glow(&mut images)).clone();
        let gr = 20.0 + prog * 30.0;
        let mut g = Sprite::from_image(img);
        g.custom_size = Some(Vec2::splat(gr * 2.0));
        g.color = Color::srgba(190.0 / 255.0, 160.0 / 255.0, 1.0, 0.22 + prog * 0.4);
        commands.spawn((g, at(cx - gr, cy - gr, gr * 2.0, gr * 2.0, z - 0.05), PIXEL_LAYER, WarpFxUi));
        for ring in 0..2u32 {
            let dir = if ring == 1 { -1.0 } else { 1.0 };
            let base = f.wspin * dir * (1.0 + ring as f32 * 0.4);
            let rad = (if ring == 1 { 30.0f32 } else { 22.0 }) * (1.0 - prog * 0.6) + 4.0;
            let col = if ring == 1 {
                Color::srgba(127.0 / 255.0, 216.0 / 255.0, 1.0, 0.5 + prog * 0.5)
            } else {
                Color::srgba(184.0 / 255.0, 144.0 / 255.0, 1.0, 0.5 + prog * 0.5)
            };
            let s = 1.0 + (prog * 2.0).round();
            for i in 0..10 {
                let a = base + i as f32 * (TAU / 10.0);
                let (px, py) = (cx + a.cos() * rad, cy + a.sin() * rad * 0.72); // squashed = portal perspective
                commands.spawn((
                    Sprite::from_color(col, Vec2::splat(s)),
                    at((px - s / 2.0).round(), (py - s / 2.0).round(), s, s, z),
                    PIXEL_LAYER,
                    WarpFxUi,
                ));
            }
        }
        for i in 0..6 {
            // sparks rising up the column (js)
            let a = (i * 61) as f32 * TAU / 360.0;
            let px = cx + (a + f.wt as f32 * 0.05).cos() * 9.0;
            let py = cy + 18.0 - ((f.wt as f32 * 2.6 + i as f32 * 30.0) % 48.0);
            commands.spawn((
                Sprite::from_color(Color::srgba(224.0 / 255.0, 206.0 / 255.0, 1.0, 0.7), Vec2::new(1.0, 2.0)),
                at(px.round(), py.round(), 1.0, 2.0, z + 0.02),
                PIXEL_LAYER,
                WarpFxUi,
            ));
        }
    } else if arrive.0 > 0 {
        let prog = 1.0 - arrive.0 as f32 / WARP_ARRIVE as f32;
        let fa = (1.0 - prog / 0.5).max(0.0);
        if fa > 0.0 {
            commands.spawn((
                Sprite::from_color(Color::srgba(216.0 / 255.0, 206.0 / 255.0, 1.0, 0.6 * fa), Vec2::new(PX_W as f32, PX_H as f32)),
                at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, 12.5),
                PIXEL_LAYER,
                WarpFxUi,
            ));
        }
        // the expanding shockwave, drawn as a ring of dots
        let rad = prog * 64.0;
        let s = ((1.0 - prog) * 4.0 + 1.0).round().max(1.0);
        let col = Color::srgba(184.0 / 255.0, 144.0 / 255.0, 1.0, (1.0 - prog) * 0.7);
        for i in 0..24 {
            let a = i as f32 * (TAU / 24.0);
            let (px, py) = (cx + a.cos() * rad, cy + a.sin() * rad);
            commands.spawn((
                Sprite::from_color(col, Vec2::splat(s)),
                at((px - s / 2.0).round(), (py - s / 2.0).round(), s, s, z),
                PIXEL_LAYER,
                WarpFxUi,
            ));
        }
    }
}

/// Burn the landing flash down on the fixed clock.
fn arrive_tick(mut arrive: ResMut<WarpArrive>) {
    if arrive.0 > 0 {
        arrive.0 -= 1;
    }
}

pub struct FlutePlugin;

impl Plugin for FlutePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mana>()
            .init_resource::<LearnedSongs>()
            .init_resource::<Fluting>()
            .init_resource::<WarpArrive>()
            .add_message::<WarpTo>()
            .add_systems(
                bevy::app::FixedUpdate,
                (flute_tick, mana_regen, flute_fx_tick, wake_on_hit, catch_up_tick, arrive_tick)
                    .before(super::play::EndTick)
                    .run_if(playing),
            )
            .add_systems(Update, (flute_overlay, warp_fx).run_if(playing));
    }
}
