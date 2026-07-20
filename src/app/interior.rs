//! interior.rs — building interiors + enter/exit (port of js enterInterior/exitInterior
//! and the interiors.js scenes, extracted as fill-rect display lists).
//!
//! Stand in a doorway and press INTERACT to step inside: the overworld room swaps for the
//! interior scene (one baked image + a foreground bottom-wall strip the player tucks
//! behind), its own solidity grid, and the building's folk as live villagers. Walk onto
//! the doorway mat to leave — you land back on the doorstep, with a cooldown so the door
//! doesn't swallow you again. Shops/services (the interactables) arrive next increment.
//!
//! DEVIATIONS (flagged): one furniture layout per building KIND (the js reseeds per
//! building; folk still vary per building); the bottom wall is fully solid (the js lets
//! your lower body tuck 8px into it); interiors read as DAYLIT (their warm light sources
//! land with the lighting glow pass).

use super::battle::RoomActor;
use super::play::{ActiveRoot, CurGrid, CurRoom, GameWorld, Player};
use super::room_render::{child, RoomRoot, PLAY_X, PLAY_Y};
use super::room_props::RoomBlockers;
use super::save::SaveCtx;
use super::screen::playing;
use super::title::loader::{swap_world_room, SwapCtx};
use crate::actors::interiors_art::{InteriorDef, INTERIORS};
use crate::actors::villager::Villager;
use crate::combat::Health;
use crate::gfx::at;
use crate::input::{Action, ActionState};
use crate::room::{RoomGrid, PX_H, PX_W, TILE};
use crate::worldgen::generate::RoomMap;
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Which building the player is inside (None = the overworld). Carries the doorstep to
/// return to; the room coords never change while inside.
#[derive(Resource, Default)]
pub struct Inside(pub Option<InsideState>);

pub struct InsideState {
    pub def: &'static InteriorDef,
    return_pos: (f32, f32),
    /// The building's identity seed (js iseed, door-salted) — vendor stock rolls from it.
    pub iseed: u32,
    /// Sold-out ledger key for this vendor (js currentShopKey: "rx,ry,stockKind,bsalt").
    pub shop_key: Option<String>,
    /// The KEEPER's relationship key (person 0) — their hearts earn you house rates.
    pub keeper_key: Option<String>,
}

/// Frames the door trigger is ignored right after entering/leaving (js homeCooldown).
#[derive(Resource, Default)]
pub struct DoorCooldown(pub u32);

/// Rasterized interior scenes, baked from the display lists on first entry.
#[derive(Resource, Default)]
pub struct InteriorArt(HashMap<&'static str, Handle<Image>>);

pub struct InteriorPlugin;

impl Plugin for InteriorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inside>()
            .init_resource::<DoorCooldown>()
            .init_resource::<InteriorArt>()
            .add_systems(
                bevy::app::FixedUpdate,
                (door_enter, door_exit).before(super::play::EndTick).run_if(playing),
            );
    }
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// Stand in a doorway + press INTERACT -> inside (js: nearDoor && pressed('interact')).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
/// A pressable doorway: (interior kind, door x, door y, trigger zone).
type DoorCand = (String, i32, i32, (f32, f32, f32, f32));

/// door_enter's read-side references (the fn sits at the 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct DoorRefs<'w, 's> {
    pub gather: Res<'w, super::gather::GatherState>,
    pub in_dungeon: Res<'w, super::dungeon::InDungeon>,
    pub cave_doors: Query<'w, 's, &'static super::caves::CaveDoor>,
    pub house: Res<'w, super::home::PlayerHouse>,
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn door_enter(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut inside: ResMut<Inside>,
    mut cooldown: ResMut<DoorCooldown>,
    mut art: ResMut<InteriorArt>,
    cur: Res<CurRoom>,
    world: Res<GameWorld>,
    mut grid: ResMut<CurGrid>,
    mut blockers: ResMut<RoomBlockers>,
    mut root: ResMut<ActiveRoot>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health, &mut crate::combat::Knockback)>,
    mut banners: ResMut<super::banners::Banners>,
    refs: DoorRefs,
) {
    if cooldown.0 > 0 {
        cooldown.0 -= 1;
    }
    if inside.0.is_some() || refs.in_dungeon.0.is_some() || cooldown.0 > 0 || !input.pressed(Action::Interact) {
        return;
    }
    let Ok((mut p, mut health, mut kb)) = players.single_mut() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0); // js player.hitbox
    // Doors are derived from the room's entity layout on the press — no live door
    // state. A shop-dest CAVE DOOR joins the list as the HIDDEN SHOP's way in.
    let mut cands: Vec<DoorCand> = world
        .0
        .room_entities(cur.rx, cur.ry)
        .into_iter()
        .filter_map(|e| {
            let kind = match e.kind {
                "town" => e.sub.clone(),
                "shop" => "shop".to_string(),
                _ => return None,
            };
            Some((kind, e.x, e.y, ((e.x - 4) as f32, (e.y + 8) as f32, 24.0, 18.0)))
        })
        .collect();
    for d in &refs.cave_doors {
        if d.dest == "shop" {
            cands.push(("caveshop".to_string(), d.x as i32, d.y as i32, super::caves::door_zone(d)));
        }
    }
    // The player's built home (app/home.rs) — its door opens the "house" interior (bed + chest).
    if let Some(h) = &refs.house.0
        && h.room == (cur.rx, cur.ry)
    {
        cands.push(("house".to_string(), h.x as i32, h.y as i32, super::home::door_zone(h.x, h.y)));
    }
    for (kind, ex, ey, door) in cands {
        let kind: &str = &kind;
        if !overlap(hitbox, door) {
            continue;
        }
        let Some(def) = INTERIORS.iter().find(|d| d.kind == kind) else { continue };
        input.consume(Action::Interact); // the door eats the press (nothing else fires)

        // --- The swap: the overworld room leaves, the scene stands up. ---
        commands.entity(root.0).despawn();
        for a in &actors {
            commands.entity(a).despawn();
        }
        let img = scene_image(&mut art, def, &mut images);
        let new_root = commands
            .spawn((Transform::default(), Visibility::default(), RoomRoot))
            .id();
        child(
            &mut commands,
            new_root,
            Sprite::from_image(img.clone()),
            at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, 1.0),
        );
        // The bottom (front) wall redrawn OVER the actors, minus the door gap, so the
        // player tucks behind it on the way out (js drawForeground).
        let wy = (PX_H - TILE) as f32;
        let (gx0, gx1) = (128.0, 176.0); // (MIDC-1)*T .. (MIDC+2)*T
        for (x0, w) in [(0.0, gx0), (gx1, PX_W as f32 - gx1)] {
            let mut sprite = Sprite::from_image(img.clone());
            sprite.rect = Some(Rect::new(x0, wy, x0 + w, wy + TILE as f32));
            child(&mut commands, new_root, sprite, at(PLAY_X + x0, PLAY_Y + wy, w, TILE as f32, 8.6));
        }
        // The building's folk — the keeper (first, holds their post) + patrons, each a
        // stable identity from the building's seed (js iseed, door-salted).
        let bs = ((ex as u32).wrapping_mul(40503)) ^ ((ey as u32).wrapping_add(7)).wrapping_mul(2654435761);
        let iseed = ((cur.rx as u32).wrapping_mul(73856093))
            ^ ((cur.ry as u32).wrapping_mul(19349663))
            ^ bs
            ^ world.0.seed;
        for (i, (px, py, still, line)) in def.people.iter().enumerate() {
            let ve = child(
                &mut commands,
                new_root,
                Sprite::default(),
                at(PLAY_X + *px as f32, PLAY_Y + *py as f32, 16.0, 16.0, 5.0),
            );
            let vseed = iseed ^ ((i as u32 + 1).wrapping_mul(0x9e3779b9));
            let mut v = Villager::new(*px as f32, *py as f32, vseed, line.to_string());
            // Named people (js pkey "i:rx,ry:kind:doorX,doorY:i") — the FIRST is the
            // keeper and wears their trade in the name (js titleFor).
            v.identify(
                format!("i:{},{}:{}:{},{}:{}", cur.rx, cur.ry, kind, ex, ey, i),
                if i == 0 { crate::people::title_for(vseed, kind) } else { crate::people::name_for(vseed).to_string() },
            );
            if *still {
                v.hold_post();
            }
            v.stagger();
            commands.entity(ve).insert(v);
        }

        // Lore tomes: libraries keep free shelves; ~1 in 3 other buildings keeps one on
        // its furniture — deterministic per location + kind, read ones vanish (js).
        let (brx, bry) = (cur.rx, cur.ry);
        if kind == "library" {
            let base = ((brx as u32).wrapping_mul(92837)) ^ ((bry as u32).wrapping_mul(689287));
            for (i, (sc, sr)) in [(3, 5), (6, 5), (9, 5), (12, 5)].into_iter().enumerate() {
                let id = crate::lore_books::book_id_for("library", base.wrapping_add(i as u32 * 7));
                if !refs.gather.tomes.contains(id) {
                    super::gather::spawn_book(&mut commands, &mut images, id, (sc * 16) as f32, (sr * 16) as f32);
                }
            }
        } else if !def.book_spots.is_empty() {
            let mut kh: u32 = 0;
            for ch in kind.bytes() {
                kh = kh.wrapping_mul(31).wrapping_add(ch as u32);
            }
            let h = ((brx as u32).wrapping_mul(92837)) ^ ((bry as u32).wrapping_mul(689287)) ^ kh;
            if h % 100 < 34 {
                let place = match kind {
                    "tavern" => "tavern",
                    "church" | "temple" => "chapel",
                    "house" | "home" | "cottage" => "home",
                    _ => "town",
                };
                let (bx, by) = def.book_spots[h as usize % def.book_spots.len()];
                let id = crate::lore_books::book_id_for(place, h >> 4);
                if !refs.gather.tomes.contains(id) {
                    super::gather::spawn_book(&mut commands, &mut images, id, bx as f32, by as f32);
                }
            }
        }

        // Interior solidity replaces the room grid; prop blockers clear.
        grid.0 = interior_grid(def);
        blockers.0 = vec![];
        root.0 = new_root;
        // Vendors get their sold-out ledger key (js currentShopKey — the bsalt is the
        // door's coords, so two same-kind shops on one street stay distinct shelves).
        let shop_key = (!def.stock.is_empty() || def.kind == "shop")
            .then(|| format!("{},{},{},{},{}", cur.rx, cur.ry, def.stock, ex, ey));
        let keeper_key = (!def.people.is_empty())
            .then(|| format!("i:{},{}:{}:{},{}:0", cur.rx, cur.ry, kind, ex, ey));
        inside.0 = Some(InsideState { def, return_pos: (p.x, p.y), iseed, shop_key, keeper_key });
        p.x = def.spawn.0 as f32;
        p.y = def.spawn.1 as f32;
        p.facing = crate::actors::hero::Facing::Up;
        health.invuln = 20;
        kb.timer = 0; // js enterInterior: knockTimer = 0
        cooldown.0 = 45;
        banners.interior(def.title);
        return;
    }
}

/// Walk onto the doorway mat -> back to the doorstep (js exit handling + homeCooldown).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn door_exit(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cooldown: ResMut<DoorCooldown>,
    mut ctx: SaveCtx,
    mut swap: SwapCtx,
    caves: Res<super::caves::CrackCaves>,
    songs_opened: Res<super::caves::OpenedSongstones>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<&mut Player>,
) {
    // (Inside rides in SwapCtx — swap_world_room clears it for every outdoor stand-up.)
    let Some(state) = &swap.inside.0 else { return };
    let Ok(mut p) = players.single_mut() else { return };
    let feet = (p.x + 2.0, p.y + 8.0, 12.0, 8.0);
    let (ex, ey, ew, eh) = state.def.exit;
    if !overlap(feet, (ex as f32, ey as f32, ew as f32, eh as f32)) {
        return;
    }
    let (rx, ry) = (ctx.cur.rx, ctx.cur.ry);
    let back = state.return_pos;
    // The interior root despawns inside swap_world_room (it IS the active root).
    swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &caves, &songs_opened, &actors, rx, ry);
    p.x = back.0;
    p.y = back.1;
    p.facing = crate::actors::hero::Facing::Down;
    cooldown.0 = 45;
}

/// Bake a scene's display list into its image (cached per kind).
fn scene_image(art: &mut InteriorArt, def: &'static InteriorDef, images: &mut Assets<Image>) -> Handle<Image> {
    if let Some(h) = art.0.get(def.kind) {
        return h.clone();
    }
    let (w, h) = (PX_W as usize, PX_H as usize);
    let mut buf = vec![0u8; w * h * 4];
    for (x, y, rw, rh, rgba) in def.rects {
        let (sr, sg, sb, sa) = (
            (rgba >> 24) as u8,
            (rgba >> 16) as u8,
            (rgba >> 8) as u8,
            *rgba as u8,
        );
        for yy in (*y).max(0)..(y + rh).min(h as i16) {
            for xx in (*x).max(0)..(x + rw).min(w as i16) {
                let i = (yy as usize * w + xx as usize) * 4;
                if sa == 255 {
                    buf[i..i + 4].copy_from_slice(&[sr, sg, sb, 255]);
                } else {
                    // src-over blend (the js shadows paint with alpha).
                    let a = sa as u32;
                    for (k, s) in [(0, sr), (1, sg), (2, sb)] {
                        buf[i + k] = ((s as u32 * a + buf[i + k] as u32 * (255 - a)) / 255) as u8;
                    }
                    buf[i + 3] = buf[i + 3].max(sa);
                }
            }
        }
    }
    let img = images.add(Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    art.0.insert(def.kind, img.clone());
    img
}

/// The interior's solidity as a RoomGrid ('#' rows -> tree-solid tiles).
fn interior_grid(def: &InteriorDef) -> RoomGrid {
    let map = RoomMap {
        map: def.solid.iter().map(|r| r.replace('#', "T")).collect(),
        prot: std::collections::HashSet::default(),
    };
    RoomGrid::from_map(&map)
}
