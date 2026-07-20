//! villager.rs — the friendly town wanderer (port of the entities.js `villager`): their
//! own stable random look (seed-keyed), ambling near home, facing you when you're close.
//! Never hostile, no health. The speech bubble + names arrive with the dialogue port.
//!
//! DEVIATION (flagged): the js villager is solid to the PLAYER too; our blockers are
//! static rects, so v1 villagers dodge the player themselves but don't block him.

use crate::actors::hero::{build_frames, random_look, HeroFrames};
use crate::app::play::Player;
use crate::app::room_props::RoomBlockers;
use crate::app::play::CurGrid;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::room::{PX_H, PX_W};
use crate::worldgen::rng::Mulberry32;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Baked frame banks per villager seed — a town's regulars keep their faces across
/// visits without re-baking ten sprites a head.
#[derive(Resource, Default)]
pub struct VillagerArt(pub HashMap<u32, HeroFrames>);

impl VillagerArt {
    pub fn frames(&mut self, seed: u32, images: &mut Assets<Image>) -> &HeroFrames {
        self.0.entry(seed).or_insert_with(|| build_frames(&random_look(seed), images))
    }
}

#[derive(Component)]
pub struct Villager {
    pub x: f32,
    pub y: f32,
    pub seed: u32,
    pub line: String, // what they'll SAY next — chatWith swaps in tiered dialogue
    /// Their unchanging character line (greeting() falls back to it — the js reuses
    /// `line` for both, which is stable there but loses the original once we vary lines).
    pub stock_line: String,
    /// Relationship-ledger key (js pkey) — None for unnamed folk (festival extras).
    pub pkey: Option<String>,
    /// Display name/title (js pname) — keepers wear their trade.
    pub pname: Option<String>,
    /// Frames left on their speech bubble (js chatT — they speak when spoken to).
    pub chat_t: u32,
    home: (f32, f32),
    facing: usize, // Facing as usize (down/up/right/left)
    anim: u32,
    dir: (f32, f32),
    move_t: i32,
    still: bool, // keepers hold their post — face you when near, never wander
}

impl Villager {
    pub fn new(x: f32, y: f32, seed: u32, line: String) -> Self {
        Self {
            x,
            y,
            seed,
            stock_line: line.clone(),
            line,
            pkey: None,
            pname: None,
            chat_t: 0,
            home: (x, y),
            facing: 0,
            anim: 0,
            dir: (0.0, 0.0),
            move_t: 0,
            still: false,
        }
    }
    /// Name them (js: v.pkey/v.pname — a NAMED person the ledger can track).
    pub fn identify(&mut self, pkey: String, pname: String) {
        self.pkey = Some(pkey);
        self.pname = Some(pname);
    }
    /// Shopkeepers and stage bards stand their ground (js villager `still`).
    pub fn hold_post(&mut self) {
        self.still = true;
    }
    /// Stagger the first wander delay per identity so a town doesn't march in lockstep.
    pub fn stagger(&mut self) {
        self.move_t = (Mulberry32::new(self.seed.max(1)).next_f64() * 70.0) as i32;
    }
}

/// The amble (js villager.update): near the player they stop and face him; otherwise
/// pick a drift direction every second or two, leashed ~44px to home.
pub fn villager_tick(
    grid: Res<CurGrid>,
    blockers: Res<RoomBlockers>,
    mut rng: ResMut<crate::app::battle::GameRng>,
    players: Query<&Player>,
    mut villagers: Query<&mut Villager>,
) {
    let player = players.single().ok();
    for mut v in &mut villagers {
        let (pdx, pdy, pd) = player.map_or((0.0, 0.0, f32::MAX), |p| {
            let (dx, dy) = (p.x - v.x, p.y - v.y);
            (dx, dy, dx.hypot(dy))
        });
        if v.still {
            // js: face the player inside 48px, else settle facing down.
            v.facing = if pd < 48.0 { face_of(pdx, pdy) } else { 0 };
            v.anim = 0;
            continue;
        }
        if pd < 40.0 {
            v.facing = face_of(pdx, pdy);
            v.dir = (0.0, 0.0);
            v.anim = 0;
            continue;
        }
        v.move_t -= 1;
        if v.move_t <= 0 {
            v.move_t = 50 + (rng.0.next_f64() * 70.0) as i32;
            v.dir = match (rng.0.next_f64() * 6.0) as u32 {
                0 => (-1.0, 0.0),
                1 => (1.0, 0.0),
                2 => (0.0, -1.0),
                3 => (0.0, 1.0),
                _ => (0.0, 0.0),
            };
        }
        // Drift back toward home past the leash.
        if v.x - v.home.0 > 44.0 {
            v.dir.0 = -1.0;
        } else if v.home.0 - v.x > 44.0 {
            v.dir.0 = 1.0;
        }
        if v.y - v.home.1 > 44.0 {
            v.dir.1 = -1.0;
        } else if v.home.1 - v.y > 44.0 {
            v.dir.1 = 1.0;
        }
        let (mx, my) = (v.dir.0 * 0.35, v.dir.1 * 0.35);
        if mx != 0.0 || my != 0.0 {
            let (nx, ny) = (v.x + mx, v.y + my);
            let (bx, by, bw, bh) = (nx + 3.0, ny + 8.0, 10.0, 6.0);
            let in_bounds =
                nx >= 4.0 && ny >= 4.0 && nx <= (PX_W - 18) as f32 && ny <= (PX_H - 18) as f32;
            let player_hit = player.is_some_and(|p| {
                bx < p.x + 13.0 && bx + bw > p.x + 3.0 && by < p.y + 15.0 && by + bh > p.y + 2.0
            });
            if in_bounds
                && !grid.0.box_hits_solid(bx, by, bw, bh)
                && !blockers.blocks((v.x + 3.0, v.y + 8.0, bw, bh), (bx, by, bw, bh))
                && !player_hit
            {
                v.x = nx;
                v.y = ny;
            }
        }
        if v.dir.0 != 0.0 {
            v.facing = if v.dir.0 < 0.0 { 3 } else { 2 };
        } else if v.dir.1 != 0.0 {
            v.facing = if v.dir.1 < 0.0 { 1 } else { 0 };
        }
        if v.dir.0 != 0.0 || v.dir.1 != 0.0 {
            v.anim += 1;
        } else {
            v.anim = 0; // stopping snaps to the standing pose
        }
    }
}

fn face_of(dx: f32, dy: f32) -> usize {
    if dx.abs() > dy.abs() {
        if dx < 0.0 { 3 } else { 2 }
    } else if dy < 0.0 {
        1
    } else {
        0
    }
}

/// Push each villager's gait frame + position into its sprite (js villager.draw: the
/// step frames carry the hero's 1px body-bob).
pub fn sync_villagers(
    mut art: ResMut<VillagerArt>,
    mut images: ResMut<Assets<Image>>,
    mut q: Query<(&Villager, &mut Sprite, &mut Transform)>,
) {
    for (v, mut sprite, mut tf) in &mut q {
        let frames = art.frames(v.seed, &mut images);
        let fi = ((v.anim / 8) % 4) as usize;
        let img = &frames.frames[v.facing][fi];
        if sprite.image != *img {
            sprite.image = img.clone();
        }
        let bob = if fi & 1 == 1 { 1.0 } else { 0.0 };
        *tf = crate::gfx::at(
            PLAY_X + v.x.round(),
            PLAY_Y + v.y.round() - bob,
            16.0,
            16.0,
            actor_z(v.y.round() + 16.0),
        );
    }
}
