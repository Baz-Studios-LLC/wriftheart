//! Door-graph consistency: every dungeon doorway must be TWO-SIDED and lead to a real
//! room, locks must lock both faces, and every stairway must land somewhere that can
//! send you back. A break in any of these plays as Baz's bug report verbatim —
//! "sometimes you enter a room and can't get out" — because navigate() silently vetoes
//! a missing door/room, and a one-sided door is a one-way trap.

use wriftheart::dungeon::{generate, Dir, Door, GenOpts};

/// Walk one generated dungeon and panic (with the full address) on any inconsistency.
fn check(seed: u32, theme: &str, opts: &GenOpts, label: &str) {
    let d = generate(seed, theme, opts);
    for (f, fl) in d.floors.iter().enumerate() {
        for (&k, room) in &fl.rooms {
            for dir in Dir::ALL {
                if room.door(dir) == Door::None {
                    continue;
                }
                let (dx, dy) = dir.vec();
                let nk = (k.0 + dx, k.1 + dy);
                // The MIRROR haze-hall's fake doors are the riddle: every way LOOKS
                // open by design and navigate() walks you back in (judged before its
                // missing-room veto) — doors to nowhere are legal there alone.
                let Some(n) = fl.rooms.get(&nk) else {
                    assert!(
                        room.mirror,
                        "{label} seed {seed} floor {f}: room {k:?} door {dir:?} leads to NOTHING"
                    );
                    continue;
                };
                assert!(
                    n.door(dir.opp()) != Door::None || room.mirror,
                    "{label} seed {seed} floor {f}: door {k:?}->{nk:?} is ONE-SIDED (a one-way trap)"
                );
                assert_eq!(
                    fl.locked.contains(&(k, dir)),
                    fl.locked.contains(&(nk, dir.opp())),
                    "{label} seed {seed} floor {f}: lock {k:?}<->{nk:?} {dir:?} is one-faced"
                );
                assert_eq!(
                    fl.ornate.contains(&(k, dir)),
                    fl.ornate.contains(&(nk, dir.opp())),
                    "{label} seed {seed} floor {f}: ornate {k:?}<->{nk:?} {dir:?} is one-faced"
                );
            }
            // Stairs down must land on the next floor, on a room that can send you back.
            if let Some(t) = room.stairs_down {
                let below = d.floors.get(f + 1).unwrap_or_else(|| {
                    panic!("{label} seed {seed} floor {f}: stairs down at {k:?} but no floor below")
                });
                let arrive = below.rooms.get(&t).unwrap_or_else(|| {
                    panic!("{label} seed {seed} floor {f}: stairs down at {k:?} land on missing room {t:?}")
                });
                assert!(
                    arrive.stairs_up.is_some(),
                    "{label} seed {seed} floor {f}: stairs down {k:?} -> {t:?} has NO WAY BACK UP"
                );
            }
            if let Some(t) = room.stairs_up
                && f > 0
            {
                assert!(
                    d.floors[f - 1].rooms.contains_key(&t),
                    "{label} seed {seed} floor {f}: stairs up at {k:?} point at missing room {t:?}"
                );
            }
            // A vault must know its parent (its stairs climb back there).
            if room.vault {
                let p = room.vault_of.unwrap_or_else(|| {
                    panic!("{label} seed {seed} floor {f}: vault {k:?} has no parent to climb back to")
                });
                assert!(
                    fl.rooms.contains_key(&p),
                    "{label} seed {seed} floor {f}: vault {k:?} parent {p:?} does not exist"
                );
            }
        }
    }
}

/// From the start/arrival room, BFS over doored neighbours (locks are key-openable, so
/// they still count as passages). Every non-vault room must be reachable — an isolated
/// room is a room you can never walk into (Baz: "can't walk right... like a wall").
fn check_reachable(seed: u32, theme: &str, opts: &GenOpts, label: &str) {
    let d = generate(seed, theme, opts);
    for (f, fl) in d.floors.iter().enumerate() {
        let start = fl
            .rooms
            .iter()
            .find(|(_, r)| matches!(r.rtype, RoomType::Start | RoomType::Arrival))
            .map(|(&k, _)| k)
            .unwrap_or_else(|| panic!("{label} seed {seed} floor {f}: no start/arrival room"));
        let mut seen = std::collections::HashSet::from([start]);
        let mut stack = vec![start];
        while let Some(k) = stack.pop() {
            let room = &fl.rooms[&k];
            for dir in Dir::ALL {
                if room.door(dir) == Door::None {
                    continue;
                }
                let (dx, dy) = dir.vec();
                let nk = (k.0 + dx, k.1 + dy);
                if fl.rooms.contains_key(&nk) && seen.insert(nk) {
                    stack.push(nk);
                }
            }
        }
        for (&k, r) in &fl.rooms {
            if r.vault || matches!(r.rtype, RoomType::Stairs) {
                continue; // vaults + stair-arrival rooms are reached by pads, not doors
            }
            assert!(
                seen.contains(&k),
                "{label} seed {seed} floor {f}: room {k:?} ({:?}) is UNREACHABLE from the entrance",
                r.rtype
            );
        }
    }
}

use wriftheart::dungeon::RoomType;

/// The collision layer: for every UNLOCKED door, the room's solid grid must actually
/// open a 3-tile gap in that wall. A door in the data with a solid wall in the grid is
/// exactly Baz's bug — you walk up to the "open" side and hit an invisible wall.
fn check_collision(seed: u32, theme: &str, opts: &GenOpts, label: &str) {
    use wriftheart::dungeon::{Dir, Door, COLS, MIDC, MIDR, ROWS};
    let mut d = generate(seed, theme, opts);
    for f in 0..d.floors.len() {
        d.floor = f; // room_view reads the cursor floor — point it at the floor under test
        let keys: Vec<((i32, i32), bool)> =
            d.floors[f].rooms.iter().map(|(&k, r)| (k, r.vault)).collect();
        for (k, is_vault) in keys {
            if is_vault {
                continue;
            }
            let Some(view) = d.room_view(k.0, k.1) else { continue };
            for dir in Dir::ALL {
                if d.floors[f].rooms[&k].door(dir) == Door::None || d.floors[f].locked.contains(&(k, dir)) {
                    continue; // no door / a locked door is meant to be a wall
                }
                // The centre tile of this wall's 3-tile gap must be walkable.
                let (r, c) = match dir {
                    Dir::N => (0usize, MIDC as usize),
                    Dir::S => ((ROWS - 1) as usize, MIDC as usize),
                    Dir::W => (MIDR as usize, 0usize),
                    Dir::E => (MIDR as usize, (COLS - 1) as usize),
                };
                assert!(
                    !view.solid[r][c],
                    "{label} seed {seed} floor {f}: room {k:?} has an OPEN {dir:?} door but the collision grid walls it off"
                );
            }
        }
    }
}

/// A locked door stays a SOLID WALL, so the hero shoving against it clamps ~22px short of
/// the edge — the tight 12px edge-walk zone can NEVER reach it, and a key would never
/// register (Baz: "I have the key but the door won't open"). The unlock therefore needs
/// the looser 26px push-zone (js tryLockedDoor `near`). This pins that the clamped hero
/// lands INSIDE that 26px zone but OUTSIDE the old 12px one — the exact bug + its fix.
#[test]
fn locked_door_is_reachable_only_by_the_push_zone() {
    use wriftheart::room::{RoomGrid, COLS, PX_W, ROWS};
    // A room whose E wall is solid the whole height == a locked E door (no gap carved).
    let rows: Vec<String> = (0..ROWS)
        .map(|r| {
            (0..COLS)
                .map(|c| if r == 0 || r == ROWS - 1 || c == 0 || c == COLS - 1 { 'M' } else { '.' })
                .collect()
        })
        .collect();
    let grid = RoomGrid::from_rows(rows);
    // Shove RIGHT from mid-room in the E-door lane (row 6), clamping like play::tick's
    // feet box (2, 8) 12x8 at the walk speed.
    let (bx, by, bw, bh) = (2.0f32, 8.0f32, 12.0f32, 8.0f32);
    let (mut px, py) = (8.0 * 16.0f32, 6.0 * 16.0f32);
    for _ in 0..400 {
        let nx = px + 1.3;
        if !grid.box_hits_solid(nx + bx, py + by, bw, bh) {
            px = nx;
        }
    }
    let cx = px + 8.0;
    assert!(
        cx >= PX_W as f32 - 26.0,
        "hero clamps at cx={cx} vs a locked E wall — the 26px push-zone (>= {}) is unreachable, so a key never registers",
        PX_W as f32 - 26.0
    );
    assert!(
        cx < PX_W as f32 - 12.0,
        "sanity: cx={cx} — the tight 12px edge-walk MUST be out of reach against a solid wall (else there was no bug to fix)"
    );
}

#[test]
fn open_doors_have_walkable_gaps() {
    let themes = ["cave", "tomb", "ruins", "castle", "stormspire", "searuin", "ossuary", "hollowroot", "saltmine"];
    for seed in 0..80u32 {
        let theme = themes[seed as usize % themes.len()];
        check_collision(seed.wrapping_mul(2654435761), theme, &GenOpts::default(), "biome");
        check_collision(seed, theme, &GenOpts { floors: Some(1 + (seed as usize % 3)), ..Default::default() }, "sized");
    }
    for seed in 0..40u32 {
        check_collision(seed, "castle", &GenOpts { floors: Some(4), ..Default::default() }, "castle");
        check_collision(seed, "saltmaze", &GenOpts { floors: Some(5), maze: true, ..Default::default() }, "saltmaze");
    }
}

#[test]
fn every_room_is_reachable() {
    let themes = ["cave", "tomb", "ruins", "castle", "stormspire", "searuin", "ossuary", "hollowroot"];
    for seed in 0..80u32 {
        let theme = themes[seed as usize % themes.len()];
        check_reachable(seed.wrapping_mul(2654435761), theme, &GenOpts::default(), "biome");
        check_reachable(seed, theme, &GenOpts { floors: Some(1 + (seed as usize % 3)), ..Default::default() }, "sized");
    }
    for seed in 0..40u32 {
        check_reachable(seed, "castle", &GenOpts { floors: Some(4), ..Default::default() }, "castle");
    }
}

#[test]
fn doors_are_two_sided_everywhere() {
    // The real entry points, across a wide seed sweep: regular biome dungeons at every
    // floor count, the castle, the saltmaze descent, the guildhall, and rift floors.
    let themes = ["cave", "tomb", "ruins", "castle", "stormspire", "searuin", "crystalcave", "ossuary", "hollowroot", "tarpit", "blightvault", "bellbarrow", "darkdepths", "windbarrow", "saltmine"];
    for seed in 0..60u32 {
        let theme = themes[seed as usize % themes.len()];
        check(seed.wrapping_mul(2654435761), theme, &GenOpts::default(), "biome");
        check(seed, theme, &GenOpts { floors: Some(1 + (seed as usize % 3)), ..Default::default() }, "sized");
    }
    for seed in 0..30u32 {
        check(seed, "castle", &GenOpts { floors: Some(4), ..Default::default() }, "castle");
        check(seed, "saltmaze", &GenOpts { floors: Some(5), maze: true, ..Default::default() }, "saltmaze");
        check(seed, "guildhall", &GenOpts { guildhall: true, ..Default::default() }, "guildhall");
        check(seed, "riftvault", &GenOpts { floors: Some(1), rift: true, no_locks: true, ..Default::default() }, "rift");
    }
}
