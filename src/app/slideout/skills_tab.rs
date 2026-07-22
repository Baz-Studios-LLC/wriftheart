//! skills_tab.rs — the passive tree's page: the constellation rendered in the slide-out
//! (void + twinkling stars + branch nebulae + lit paths + nodes), cone-based directional
//! cursor, allocate/refund, camera lerp toward the cursor, and the tooltip band.
//!
//! Data + graph logic live in crate::skilltree; this file is interaction + sprites.
//! Points are earned one per LEVEL (rewards::gain_xp); a leaf-safe refund costs
//! REFUND_COST coin, exactly the js.

use super::{SlideOut, SlideOutUi, PAD, PANEL_W, Z};
use crate::app::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::skilltree::{self, branch_color, nodes};
use crate::ui::label;
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use std::collections::HashSet; // skilltree's set type (std, not bevy's)
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const AREA_TOP: f32 = 18.0; // below the tab strip

/// Coin per refunded node (js REFUND_COST — leaf nodes only).
pub const REFUND_COST: i64 = 40;

/// The player's allocated tree + unspent points (earned one per LEVEL — see
/// `rewards::gain_xp`; a fresh hero starts with none, like the js).
#[derive(Resource, Default)]
pub struct TreeAlloc {
    pub taken: HashSet<usize>,
    pub points: i32,
}

/// DERIVED stat totals the live systems read: the passive tree + the hero's traits folded
/// together (js player.stat = skills + Traits.stat), recomputed on every allocation change
/// and on each day/night flip (quirk traits). Stats whose systems haven't ported (crit,
/// leech, regen, iframes, ...) stay banked here until their owners arrive.
#[derive(Resource, Default)]
pub struct TreeStats {
    pub melee: f64,
    pub move_bonus: f64,
    pub maxhp: f64,
    pub gather: f64,
    pub magnet: f64,
    pub luck: f64, // scales drop odds (js luckMult = 1 + luck)
    pub coin: f64, // scales collected copper (js GOLD stat)
    pub defense: f64, // flat armor (traits today; armor nodes/gear fold in as they port)
    pub crit: f64,    // crit chance (combat.rs CritChance rolls it)
    pub critmult: f64, // crit damage bonus over the x2 base
    pub leech: f64,
    pub regen: f64,
    pub iframes: f64,  // extra mercy frames on the player's hurt profile
    pub knock: f64,    // extra swing knockback (fraction)
    pub spell: f64,    // wand/spell damage (fraction)
    pub maxmana: f64,  // flat max MP
    pub manaregen: f64, // flat MP per regen tick
    pub haste: f64,    // attack-speed: shrinks weapon cooldowns (fraction)
}

pub fn recompute(alloc: &TreeAlloc, traits: &[String], night: bool, inv: &crate::inventory::PlayerInv) -> TreeStats {
    let s = |name: &str| {
        skilltree::stat(&alloc.taken, name) + crate::traits::stat(traits, night, name) + crate::items::gear_stat(inv, name)
    };
    TreeStats {
        melee: s("melee"),
        move_bonus: s("move"),
        maxhp: s("maxhp"),
        gather: s("gather"),
        magnet: s("magnet"),
        luck: s("luck"),
        coin: s("coin"),
        defense: s("defense"),
        crit: s("crit"),
        critmult: s("critmult"),
        knock: s("knock"),
        spell: s("spell"),
        maxmana: s("maxmana"),
        manaregen: s("manaregen"),
        haste: s("haste"),
        leech: s("leech"),
        regen: s("regen"),
        iframes: s("iframes"),
    }
}

/// Worn gear changed -> the stat sums refresh (js refreshStats on equipGear).
pub fn gear_refresh(
    inv: Res<crate::inventory::PlayerInv>,
    alloc: Res<TreeAlloc>,
    ident: Res<crate::app::identity::HeroIdent>,
    night: Res<crate::app::identity::Night>,
    mut tstats: ResMut<TreeStats>,
) {
    if inv.is_changed() {
        *tstats = recompute(&alloc, &ident.traits, night.0, &inv);
    }
}

/// Page state: the cursor node + the lerping camera (in tree coordinates). `drift`
/// is set while a mouse drag has carried the camera away from the cursor - the lerp
/// lets go until the cursor moves again (else it would fight the drag every frame).
#[derive(Resource)]
pub struct SkillsState {
    pub cursor: usize,
    pub cam: Vec2,
    pub drift: bool,
}

impl Default for SkillsState {
    fn default() -> Self {
        Self { cursor: skilltree::start(), cam: Vec2::ZERO, drift: false }
    }
}

/// Baked shapes: filled/ring circles (r 3/5/7), the keystone diamond, a soft halo — all
/// white, tinted per branch at spawn.
#[derive(Resource)]
pub struct SkillArt {
    fill: [Handle<Image>; 3],
    ring: [Handle<Image>; 3],
    diamond_fill: Handle<Image>,
    diamond_ring: Handle<Image>,
    halo: Handle<Image>,
    /// A vertical edge-fade strip, LINEAR-sampled: stretched along a rotated link quad it
    /// gives the soft anti-aliased line edges the canvas has (MSAA is off — see canvas.rs).
    line: Handle<Image>,
    /// Two WRIFT star-dust sheets (the map backdrop's own seeds), panel + one tile
    /// wide so the scroll window always stays inside them.
    wrift: [Handle<Image>; 2],
}

/// SUPERSAMPLED shape rasterizer: 4x4 coverage per pixel becomes the alpha, giving the
/// soft anti-aliased edges a canvas `arc()` stroke has — hard 1-bit circles read far more
/// pixelated than the JS under the integer upscale. Canvas stays even-sized (see above).
fn shape(images: &mut Assets<Image>, r: i32, f: impl Fn(f32, f32) -> f32) -> Handle<Image> {
    let s = (2 * r + 2) as u32;
    let c = s as f32 / 2.0;
    let mut img = Image::new_fill(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..s {
        for x in 0..s {
            let mut cov = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = x as f32 + (sx as f32 + 0.5) / 4.0 - c;
                    let dy = y as f32 + (sy as f32 + 0.5) / 4.0 - c;
                    cov += f(dx, dy).clamp(0.0, 1.0);
                }
            }
            let a = cov / 16.0;
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0))
            {
                px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

impl SkillArt {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let circle = |images: &mut Assets<Image>, r: i32| {
            shape(images, r, move |dx, dy| if dx.hypot(dy) <= r as f32 + 0.4 { 1.0 } else { 0.0 })
        };
        let ring = |images: &mut Assets<Image>, r: i32| {
            shape(images, r, move |dx, dy| {
                if (dx.hypot(dy) - r as f32).abs() <= 0.6 { 1.0 } else { 0.0 }
            })
        };
        SkillArt {
            fill: [circle(images, 3), circle(images, 5), circle(images, 7)],
            ring: [ring(images, 3), ring(images, 5), ring(images, 7)],
            diamond_fill: shape(images, 7, |dx, dy| if dx.abs() + dy.abs() <= 7.4 { 1.0 } else { 0.0 }),
            diamond_ring: shape(images, 7, |dx, dy| {
                if (dx.abs() + dy.abs() - 6.7).abs() <= 0.7 { 1.0 } else { 0.0 }
            }),
            line: {
                let mut img = Image::new_fill(
                    Extent3d { width: 2, height: 8, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    &[0, 0, 0, 0],
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                // Alpha profile down the strip: fade, solid core, fade.
                for (y, a) in [(1u32, 60u8), (2, 200), (3, 255), (4, 255), (5, 200), (6, 60)] {
                    for x in 0..2 {
                        if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                            px.copy_from_slice(&[255, 255, 255, a]);
                        }
                    }
                }
                img.sampler = ImageSampler::linear();
                images.add(img)
            },
            // Soft radial falloff for node halos + branch nebulae. QUADRATIC, and dimmer
            // than the JS numbers: Bevy alpha-blends in linear space, where the same
            // alphas bloom far brighter than the canvas (the codex-overlay lesson).
            halo: shape(images, 48, |dx, dy| {
                let t = (1.0 - dx.hypot(dy) / 48.0).max(0.0);
                t * t * 0.5
            }),
            wrift: {
                let sw = PANEL_W as u32 + crate::gfx::wrift::WRIFT_T;
                let sh = (CANVAS_H as f32 - AREA_TOP) as u32 + crate::gfx::wrift::WRIFT_T;
                [images.add(crate::gfx::wrift::wrift_sheet(0x57a11, sw, sh)), images.add(crate::gfx::wrift::wrift_sheet(0x57a12, sw, sh))]
            },
        }
    }
    fn node(&self, kind: &str) -> (Handle<Image>, Handle<Image>, f32) {
        match kind {
            "keystone" => (self.diamond_fill.clone(), self.diamond_ring.clone(), 7.0),
            "notable" | "start" => (self.fill[1].clone(), self.ring[1].clone(), 5.0),
            _ => (self.fill[0].clone(), self.ring[0].clone(), 3.0),
        }
    }
}

/// Root the tree hangs from (children in tree-local coords; the camera moves the root).
#[derive(Component)]
pub struct TreeRoot;
/// The pulsing white cursor ring.
#[derive(Component)]
pub struct CursorRing;
/// A parallax star-dust sheet behind the constellation (Baz: the map's WRIFT
/// backdrop, here too). The sprite sits FIXED over the panel; the scroll lives in
/// its texture rect - the map can translate its layers because the codex owns the
/// whole screen, but here anything oversized would bleed over the sidebar.
#[derive(Component)]
pub struct WriftLayer {
    k: f32,
    drift: Vec2,
}

/// Run condition: slide-out open on the SKILLS tab.
pub fn active(screen: Res<State<Screen>>, so: Res<SlideOut>) -> bool {
    *screen.get() == Screen::SlideOut && super::TABS[so.tab] == "SKILLS"
}

/// Cursor movement + allocate/refund — presses on the fixed clock like every menu.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn skills_input(
    state: Res<ActionState>,
    mut so: ResMut<SlideOut>,
    mut st: ResMut<SkillsState>,
    mut alloc: ResMut<TreeAlloc>,
    mut tstats: ResMut<TreeStats>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    ident: Res<crate::app::identity::HeroIdent>,
    night: Res<crate::app::identity::Night>,
    ptr: Res<crate::input::Pointer>,
    // The drag gear (Baz: drag the tree around like the map) - buttons, anchor, travel.
    mouse: (Res<ButtonInput<MouseButton>>, Local<Option<Vec2>>, Local<f32>),
) {
    let pressed_dir = [Action::Up, Action::Down, Action::Left, Action::Right]
        .iter()
        .any(|&a| state.pressed(a));
    if pressed_dir {
        // Direction from ALL held movement keys — diagonals reach the 45° branches.
        let dx = state.held(Action::Right) as i32 as f64 - state.held(Action::Left) as i32 as f64;
        let dy = state.held(Action::Down) as i32 as f64 - state.held(Action::Up) as i32 as f64;
        if let Some(next) = skilltree::nav(st.cursor, dx, dy) {
            st.cursor = next;
            st.drift = false; // keyboard nav hands the camera back to the lerp
            so.dirty = true;
        }
    }
    // MOUSE (Baz: drag the constellation around like the map): hold-and-drag pans the
    // tree under the cursor; a STILL click (released without real travel) selects the
    // node under it, and a second still click on the selected node allocates. The
    // node-to-canvas map inverts skills_anim's root transform: node = centre + (x,y) - cam.
    let (mbtn, mut drag, mut travel) = mouse;
    let mut click_alloc = false;
    let in_area = ptr.pos.filter(|p| p.y >= AREA_TOP && p.x >= SIDEBAR_W);
    if mbtn.pressed(MouseButton::Left) {
        if let Some(pos) = in_area {
            if let Some(last) = *drag {
                let d = pos - last;
                *travel += d.length();
                if *travel > 3.0 {
                    st.cam -= d; // the tree rides WITH the cursor
                    st.drift = true;
                }
                *drag = Some(pos);
            } else {
                // Arm on HELD-in-area, not just_pressed - the map drag's fixed-tick rule.
                *drag = Some(pos);
                *travel = 0.0;
            }
        }
    } else {
        if drag.take().is_some()
            && *travel <= 3.0
            && let Some(pos) = in_area
        {
            let (centre, cam) = (area_centre(), st.cam.round());
            let mut best: Option<(usize, f32)> = None;
            for (i, nd) in nodes().iter().enumerate() {
                let r = match nd.kind {
                    "keystone" => 7.0,
                    "notable" | "start" => 5.0,
                    _ => 3.0,
                };
                let nc = centre + Vec2::new(nd.x as f32, nd.y as f32) - cam;
                let d = nc.distance(pos);
                if d <= r + 2.0 && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
            if let Some((i, _)) = best {
                if st.cursor != i {
                    st.cursor = i;
                    st.drift = false; // selecting hands the camera back to the lerp
                    so.dirty = true;
                } else {
                    click_alloc = true;
                }
            }
        }
        *travel = 0.0;
    }
    let cur = st.cursor;
    let n = &nodes()[cur];
    if (state.pressed(Action::Slot1) || click_alloc)
        && !alloc.taken.contains(&cur)
        && cur != skilltree::start()
        && alloc.points >= n.cost as i32
        && skilltree::linked_to_tree(&alloc.taken, cur)
        && !skilltree::lane_sealed(&alloc.taken, cur)
    {
        alloc.taken.insert(cur);
        alloc.points -= n.cost as i32;
        *tstats = recompute(&alloc, &ident.traits, night.0, &inv);
        so.dirty = true;
    }
    if state.pressed(Action::Slot3)
        && cur != skilltree::start()
        && alloc.taken.contains(&cur)
        && skilltree::leaf_safe(&alloc.taken, cur)
        && inv.money >= REFUND_COST
    {
        // A respec costs coin (js REFUND_COST) — the points come back, the copper doesn't.
        inv.money -= REFUND_COST;
        alloc.taken.remove(&cur);
        alloc.points += n.cost as i32;
        *tstats = recompute(&alloc, &ident.traits, night.0, &inv);
        so.dirty = true;
    }
}

/// Camera lerp + cursor pulse + star twinkle, every render frame (the JS per-frame feel).
pub fn skills_anim(
    time: Res<Time>,
    mut st: ResMut<SkillsState>,
    mut root: Query<&mut Transform, (With<TreeRoot>, Without<CursorRing>)>,
    mut ring: Query<&mut Transform, (With<CursorRing>, Without<TreeRoot>)>,
    mut wrift: Query<(&WriftLayer, &mut Sprite)>,
) {
    let cur = &nodes()[st.cursor];
    let target = Vec2::new(cur.x as f32, cur.y as f32);
    if !st.drift {
        let cam = st.cam;
        st.cam = cam + (target - cam) * 0.18;
    }
    let centre = area_centre();
    if let Ok(mut tf) = root.single_mut() {
        let base = at(centre.x, centre.y, 0.0, 0.0, Z + 0.2);
        tf.translation.x = base.translation.x - st.cam.x.round();
        tf.translation.y = base.translation.y + st.cam.y.round();
    }
    let t = time.elapsed_secs();
    let t_px = crate::gfx::wrift::WRIFT_T as f32;
    for (l, mut s) in &mut wrift {
        let sx = (((st.cam.x * l.k + t * l.drift.x) % t_px + t_px) % t_px).round();
        let sy = (((st.cam.y * l.k + t * l.drift.y) % t_px + t_px) % t_px).round();
        if let Some(r) = s.rect.as_mut() {
            let (w0, h0) = (r.width(), r.height());
            *r = Rect::new(sx, sy, sx + w0, sy + h0);
        }
    }
    let pulse = 0.5 + 0.5 * (t * 6.7).sin();
    if let Ok(mut tf) = ring.single_mut() {
        tf.scale = Vec3::splat(1.0 + 0.25 * pulse);
    }
}

fn area_centre() -> Vec2 {
    Vec2::new(SIDEBAR_W + PANEL_W / 2.0, AREA_TOP + (CANVAS_H as f32 - AREA_TOP) / 2.0)
}

/// Build the whole page (called from the slide-out redraw).
#[allow(clippy::too_many_arguments)] // it IS the page's arity
pub fn draw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    art: &SkillArt,
    st: &SkillsState,
    alloc: &TreeAlloc,
    bindings: &Bindings,
    pad: bool,
) {
    let x0 = SIDEBAR_W;
    let (w, h) = (PANEL_W, CANVAS_H as f32);
    let ns = nodes();
    let tag = || SlideOutUi;
    let allocated = |i: usize| i == skilltree::start() || alloc.taken.contains(&i);
    let tint = |c: (u8, u8, u8), a: f32| Color::srgba_u8(c.0, c.1, c.2, (a * 255.0) as u8);

    // The void base - the codex overlay's exact near-black, so MAP and SKILLS read
    // as one sky (Baz).
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x05, 0x05, 0x08), Vec2::new(w, h - AREA_TOP)),
        at(x0, AREA_TOP, w, h - AREA_TOP, Z + 0.1),
        PIXEL_LAYER,
        tag(),
    ));
    // Opaque band under the tab strip + a separator line: the constellation is NOT clipped,
    // so without this it bleeds up behind the tabs (the JS clips to its offscreen area).
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x06, 0x08, 0x0e), Vec2::new(w, AREA_TOP)),
        at(x0, 0.0, w, AREA_TOP, Z + 0.7),
        PIXEL_LAYER,
        tag(),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x36), Vec2::new(w, 1.0)),
        at(x0, AREA_TOP - 1.0, w, 1.0, Z + 0.75),
        PIXEL_LAYER,
        tag(),
    ));
    // THE WRIFT: two dust sheets sliding at 0.35x / 0.65x of the tree pan (the
    // map's exact layers), under the fixed twinkle stars, over the void base.
    for (li, k, drift) in [(0usize, 0.35f32, Vec2::new(0.8, 0.45)), (1, 0.65, Vec2::new(1.4, 0.8))] {
        let mut s = Sprite::from_image(art.wrift[li].clone());
        s.custom_size = Some(Vec2::new(w, h - AREA_TOP));
        s.rect = Some(Rect::new(0.0, 0.0, w, h - AREA_TOP));
        commands.spawn((
            s,
            at(x0, AREA_TOP, w, h - AREA_TOP, Z + 0.12 + li as f32 * 0.02),
            PIXEL_LAYER,
            WriftLayer { k, drift },
            tag(),
        ));
    }

    // The tree root — every constellation sprite is a child in tree-local coords.
    let centre = area_centre();
    let root = commands
        .spawn((at(centre.x, centre.y, 0.0, 0.0, Z + 0.2), Visibility::default(), TreeRoot, tag()))
        .id();
    let local = |x: f32, y: f32, dz: f32| Transform::from_xyz(x, -y, dz);
    let child = |commands: &mut Commands, bundle: (Sprite, Transform)| {
        let e = commands.spawn((bundle.0, bundle.1, PIXEL_LAYER, tag())).id();
        commands.entity(root).add_child(e);
        e
    };

    // Branch nebulae (faint colour identity behind each arm).
    for (bi, key) in ["war", "bld", "for", "blw", "wnd", "pre", "mag", "gth", "crf"].iter().enumerate() {
        let a = bi as f32 * std::f32::consts::PI / 3.0;
        let mut s = Sprite::from_image(art.halo.clone());
        s.custom_size = Some(Vec2::splat(192.0));
        s.color = tint(branch_color(key), 0.05);
        child(commands, (s, local(a.cos() * 112.0, a.sin() * 112.0, 0.0)));
    }

    // Links (each once): lit paths glow gold over a soft branch-colour bloom.
    for (i, n) in ns.iter().enumerate() {
        for &l in &n.links {
            if l < i {
                continue;
            }
            let b = &ns[l];
            let (ax, ay) = (n.x as f32, n.y as f32);
            let (bx, by) = (b.x as f32, b.y as f32);
            let len = (bx - ax).hypot(by - ay);
            let ang = (-(by - ay)).atan2(bx - ax); // tree y-down -> local y-up
            let mid = local((ax + bx) / 2.0, (ay + by) / 2.0, 0.1);
            let lit = allocated(i) && allocated(l);
            let ckey = if n.id == "start" { &b.id } else { &n.id };
            let c = branch_color(ckey);
            if lit {
                let mut glow = Sprite::from_image(art.line.clone());
                glow.custom_size = Some(Vec2::new(len, 5.0));
                glow.color = tint(c, 0.20);
                let mut tf = mid;
                tf.rotation = Quat::from_rotation_z(ang);
                child(commands, (glow, tf));
            }
            let mut line = Sprite::from_image(art.line.clone());
            line.custom_size = Some(Vec2::new(len, if lit { 2.5 } else { 1.5 }));
            line.color = if lit { Color::srgb_u8(0xff, 0xe9, 0xa8) } else { tint(c, 0.55) };
            let mut tf = mid;
            tf.translation.z = 0.15;
            tf.rotation = Quat::from_rotation_z(ang);
            child(commands, (line, tf));
        }
    }

    // Nodes over links: halo (allocated / affordable), fill, ring, cursor.
    for (i, n) in ns.iter().enumerate() {
        let al = allocated(i);
        let sealed = !al && skilltree::lane_sealed(&alloc.taken, i);
        let can = !al && !sealed && alloc.points >= n.cost as i32 && skilltree::linked_to_tree(&alloc.taken, i);
        let c = branch_color(&n.id);
        let (fill_img, ring_img, r) = art.node(n.kind);
        if al || can {
            let mut halo = Sprite::from_image(art.halo.clone());
            halo.custom_size = Some(Vec2::splat((r + 6.0) * 2.0));
            halo.color = tint(c, if al { 0.32 } else { 0.16 });
            child(commands, (halo, local(n.x as f32, n.y as f32, 0.2)));
        }
        let mut fill = Sprite::from_image(fill_img);
        fill.color = if al {
            tint(c, 1.0)
        } else if n.kind == "start" {
            Color::srgb_u8(0xff, 0xd8, 0x65)
        } else if sealed {
            Color::srgb_u8(0x12, 0x15, 0x1c) // the road not taken goes dark
        } else {
            Color::srgb_u8(0x23, 0x2a, 0x38)
        };
        child(commands, (fill, local(n.x as f32, n.y as f32, 0.3)));
        let mut ring = Sprite::from_image(ring_img);
        ring.color = if al {
            Color::srgb_u8(0xff, 0xf2, 0xc8)
        } else if can {
            Color::srgb_u8(0xaa, 0xd7, 0xff)
        } else if sealed {
            tint(c, 0.22)
        } else {
            tint(c, 0.55)
        };
        child(commands, (ring, local(n.x as f32, n.y as f32, 0.4)));
        if i == st.cursor {
            let mut cring = Sprite::from_image(art.ring[2].clone());
            cring.custom_size = Some(Vec2::splat((r + 3.0) * 2.0));
            cring.color = Color::WHITE;
            let e = commands
                .spawn((cring, local(n.x as f32, n.y as f32, 0.5), PIXEL_LAYER, CursorRing, tag()))
                .id();
            commands.entity(root).add_child(e);
        }
    }

    // Header + tooltip band (fixed to the panel, over the tree).
    let cur = &ns[st.cursor];
    let pts = format!("PTS {}", alloc.points);
    label(commands, images, &pts, x0 + PAD, AREA_TOP + 5.0, if alloc.points > 0 { 0x9aff9a } else { 0x8a94a0 }, Z + 0.9, tag());
    let al = allocated(st.cursor);
    let sealed_cur = !al && skilltree::lane_sealed(&alloc.taken, st.cursor);
    let can = !al && !sealed_cur && alloc.points >= cur.cost as i32 && skilltree::linked_to_tree(&alloc.taken, st.cursor);
    let hint = if al && st.cursor != skilltree::start() && skilltree::leaf_safe(&alloc.taken, st.cursor) {
        format!("{} REFUND {REFUND_COST}C", bindings.prompt(Action::Slot3, pad))
    } else if can {
        format!("{} ALLOCATE", bindings.prompt(Action::Slot1, pad))
    } else {
        String::new()
    };
    if !hint.is_empty() {
        let hw = font::measure(&hint) as f32;
        label(commands, images, &hint, x0 + w - PAD - hw, AREA_TOP + 5.0, 0xfce0a8, Z + 0.9, tag());
    }
    let lines = skilltree::stat_lines(cur);
    let ty = h - 10.0 - lines.len() as f32 * 9.0 - 10.0;
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(8, 10, 14), Vec2::new(w, h - ty + 4.0)),
        at(x0, ty - 4.0, w, h - ty + 4.0, Z + 0.8),
        PIXEL_LAYER,
        tag(),
    ));
    // Separator over the tooltip band — the same 1px rule the CHAR page frames with.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x36), Vec2::new(w, 1.0)),
        at(x0, ty - 5.0, w, 1.0, Z + 0.85),
        PIXEL_LAYER,
        tag(),
    ));
    let cost_tag = if al || st.cursor == skilltree::start() {
        if al && st.cursor != skilltree::start() { " - TAKEN".to_string() } else { String::new() }
    } else if sealed_cur {
        " - OTHER PATH WALKED".to_string()
    } else {
        format!("  {} {}", cur.cost, if cur.cost == 1 { "PT" } else { "PTS" })
    };
    let name_color = match cur.kind {
        "keystone" => 0xffb347,
        "notable" => 0xffd865,
        _ => 0xcfe0ec,
    };
    let title = format!("{}{}", cur.name, cost_tag);
    label(commands, images, &title, x0 + PAD, ty, name_color, Z + 0.9, tag());
    for (i, (txt, bad)) in lines.iter().enumerate() {
        label(commands, images, txt, x0 + PAD, ty + 10.0 + i as f32 * 9.0, if *bad { 0xfc7460 } else { 0x9aff9a }, Z + 0.9, tag());
    }
}
