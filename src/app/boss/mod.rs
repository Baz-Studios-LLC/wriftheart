//! boss/ — THE TEN: bespoke shard-dungeon bosses (BOSSES.md is the design
//! constitution). Baz scrapped the js template bosses (a 2x mob sprite in a crown
//! cycling six shared attacks) — each dungeon's guardian is authored here ONE AT A
//! TIME as a self-contained actor (the mimic pattern): its own art, its own
//! signature mechanic, its own file. Shared here: the theme dispatch, and the js
//! boss HP bar (name + third-mark ticks, reddening as the fight deepens).

pub mod all_eye;
pub mod ash_titan;
pub mod bone_colossus;
pub mod broodmother;
pub mod cavern_tyrant;
pub mod dread_knight;
pub mod rot_horror;
mod choirmaster;
pub mod briar_queen;
pub mod glacier_maw;
pub mod hive_queen;
pub mod hollow_star;
pub mod mummy_king;
pub mod mycelium_throne;
pub mod storm_herald;
pub mod unmaker;
pub mod warren_hydra;
pub mod wriftheart;

use bevy::prelude::*;

use crate::app::room_render::{PLAY_X, PLAY_Y};
use crate::combat::Health;
use crate::room::PX_W;

/// The authored-boss dispatch. False = this theme's true boss isn't built yet —
/// spawn_room_boss falls back to the elite stand-in (BOSSES.md tracks the roster).
pub(crate) fn spawn_authored(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut crate::app::room_props::RoomBlockers,
    theme_key: &str,
) -> bool {
    let _ = &blockers; // most guardians need no arena furniture
    match theme_key {
        "crypt" => {
            bone_colossus::spawn(commands, images, 130.0, 40.0);
            true
        }
        "vinewarren" => {
            warren_hydra::spawn(commands, images);
            true
        }
        "hivehollow" => {
            hive_queen::spawn(commands, images);
            true
        }
        "frostcavern" => {
            glacier_maw::spawn(commands, images, blockers);
            true
        }
        "bog" => {
            all_eye::spawn(commands, images);
            true
        }
        "petalhall" => {
            briar_queen::spawn(commands, images);
            true
        }
        "fungal" => {
            mycelium_throne::spawn(commands, images);
            true
        }
        "charhall" => {
            ash_titan::spawn(commands, images);
            true
        }
        "riftvault" => {
            unmaker::spawn(commands, images);
            true
        }
        "wriftvault" => {
            hollow_star::spawn(commands, images);
            true
        }
        "saltmaze" => {
            choirmaster::spawn(commands, images);
            true
        }
        // --- The six consolidated bespoke bosses (BOSSES.md): each covers the related themes
        //     THE TEN left on elite stand-ins, authored not templated. ---
        "cave" | "crystalcave" | "darkdepths" | "saltmine" => {
            cavern_tyrant::spawn(commands, images, blockers);
            true
        }
        "stormspire" | "windbarrow" => {
            storm_herald::spawn(commands, images);
            true
        }
        "tomb" | "ossuary" | "hollowroot" => {
            mummy_king::spawn(commands, images);
            true
        }
        "castle" => {
            dread_knight::spawn(commands, images);
            true
        }
        "searuin" | "tarpit" | "blightvault" => {
            rot_horror::spawn(commands, images);
            true
        }
        "ruins" | "bellbarrow" => {
            broodmother::spawn(commands, images);
            true
        }
        _ => false,
    }
}

/// On the boss entity: the HP bar's title (js bossName).
#[derive(Component)]
pub struct BossName(pub &'static str);

pub struct BossPlugin;
impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        let b = crate::combat::resolve_combat;
        app.add_systems(
            bevy::app::FixedUpdate,
            (
                // The six consolidated bespoke bosses (batch 1+2) + the crypt colossus.
                (
                    bone_colossus::tick.before(b),
                    bone_colossus::deaths.after(b),
                    cavern_tyrant::tick.before(b),
                    cavern_tyrant::deaths.after(b),
                    storm_herald::tick.before(b),
                    storm_herald::deaths.after(b),
                    mummy_king::tick.before(b),
                    mummy_king::deaths.after(b),
                    dread_knight::tick.before(b),
                    dread_knight::deaths.after(b),
                    rot_horror::tick.before(b),
                    rot_horror::deaths.after(b),
                    broodmother::tick.before(b),
                    broodmother::deaths.after(b),
                ),
                (
                    warren_hydra::tick.before(b),
                    warren_hydra::deaths.after(b),
                    hive_queen::tick.before(b),
                    hive_queen::deaths.after(b),
                    glacier_maw::tick.before(b),
                    glacier_maw::deaths.after(b),
                    all_eye::tick.before(b),
                    all_eye::deaths.after(b),
                    briar_queen::tick.before(b),
                    briar_queen::deaths.after(b),
                ),
                (
                    mycelium_throne::tick.before(b),
                    mycelium_throne::deaths.after(b),
                    ash_titan::tick.before(b),
                    ash_titan::deaths.after(b),
                    unmaker::tick.before(b),
                    unmaker::deaths.after(b),
                    hollow_star::tick.before(b),
                    hollow_star::deaths.after(b),
                    choirmaster::tick.before(b),
                    choirmaster::deaths.after(b),
                    wriftheart::tick.before(b),
                    wriftheart::wriftbane_hits.after(b),
                    wriftheart::deaths.after(b),
                ),
            )
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        )
        .add_systems(Update, boss_bar);
    }
}

/// The bar's five quads + the baked name (js drawDungeonHud: 168px bar centred at
/// the playfield top, third-mark ticks, fill reddening as thirds fall).
struct BarRig {
    back: Entity,
    name: Entity,
    bar_bg: Entity,
    fill: Entity,
    ticks: [Entity; 2],
    title: &'static str,
}

const BAR_W: f32 = 168.0;
const BAR_Z: f32 = 30.0;

fn boss_bar(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rig: Local<Option<BarRig>>,
    bosses: Query<(&BossName, &Health)>,
    mut sprites: Query<(&mut Sprite, &mut Transform)>,
) {
    let Some((name, h)) = bosses.iter().next() else {
        // The guardian is gone (or we left): the bar goes with it.
        if let Some(r) = rig.take() {
            for e in [r.back, r.name, r.bar_bg, r.fill, r.ticks[0], r.ticks[1]] {
                commands.entity(e).despawn();
            }
        }
        return;
    };
    let bx = PLAY_X + (PX_W as f32 - BAR_W) / 2.0;
    let by = PLAY_Y + 4.0;
    let frac = (h.hp.max(0) as f32 / h.max.max(1) as f32).clamp(0.0, 1.0);
    let col = if frac > 2.0 / 3.0 {
        0xe23030
    } else if frac > 1.0 / 3.0 {
        0xff9020
    } else {
        0xff5020
    };
    if rig.is_none() {
        let quad = |commands: &mut Commands, c: Color, x: f32, y: f32, w: f32, hh: f32, z: f32| {
            commands
                .spawn((Sprite::from_color(c, Vec2::new(w, hh)), crate::gfx::at(x, y, w, hh, z), crate::gfx::PIXEL_LAYER))
                .id()
        };
        let (img, tw) = crate::gfx::font::bake_text(name.0, 0xfca0a0, &mut images);
        let namee = commands
            .spawn((
                Sprite::from_image(img),
                crate::gfx::at(bx + (BAR_W - tw as f32) / 2.0, by, tw as f32, 7.0, BAR_Z + 0.2),
                crate::gfx::PIXEL_LAYER,
            ))
            .id();
        *rig = Some(BarRig {
            back: quad(&mut commands, Color::srgba(0.0, 0.0, 0.0, 0.6), bx - 3.0, by - 2.0, BAR_W + 6.0, 15.0, BAR_Z),
            name: namee,
            bar_bg: quad(&mut commands, Color::srgb_u8(0x3a, 0x10, 0x10), bx, by + 8.0, BAR_W, 4.0, BAR_Z + 0.1),
            fill: quad(&mut commands, Color::srgb_u8(0xe2, 0x30, 0x30), bx, by + 8.0, BAR_W, 4.0, BAR_Z + 0.2),
            ticks: [
                quad(&mut commands, Color::srgba(0.0, 0.0, 0.0, 0.7), bx + (BAR_W / 3.0).round(), by + 8.0, 1.0, 4.0, BAR_Z + 0.3),
                quad(&mut commands, Color::srgba(0.0, 0.0, 0.0, 0.7), bx + (2.0 * BAR_W / 3.0).round(), by + 8.0, 1.0, 4.0, BAR_Z + 0.3),
            ],
            title: name.0,
        });
        return;
    }
    let r = rig.as_mut().unwrap();
    if r.title != name.0 {
        // A different guardian took the stage (back-to-back arenas): rebake the title.
        let (img, tw) = crate::gfx::font::bake_text(name.0, 0xfca0a0, &mut images);
        if let Ok((mut s, mut t)) = sprites.get_mut(r.name) {
            s.image = img;
            *t = crate::gfx::at(bx + (BAR_W - tw as f32) / 2.0, by, tw as f32, 7.0, BAR_Z + 0.2);
        }
        r.title = name.0;
    }
    if let Ok((mut s, mut t)) = sprites.get_mut(r.fill) {
        let [rr, g, b] = [(col >> 16) as u8, (col >> 8) as u8, col as u8];
        s.color = Color::srgb_u8(rr, g, b);
        let w = (BAR_W * frac).round().max(0.0);
        s.custom_size = Some(Vec2::new(w.max(0.001), 4.0));
        *t = crate::gfx::at(bx, by + 8.0, w, 4.0, BAR_Z + 0.2);
    }
}
