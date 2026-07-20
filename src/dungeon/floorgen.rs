//! floorgen.rs — the floor generators (js genFloor / genMazeFloor / genMirrorFloor /
//! genGuildhallFloor + generate). One mulberry32 stream drives the whole stack; every
//! iteration order that touches it is js-verbatim (see mod.rs PARITY note).

use super::decor::{place_decor, solid_decor_tiles};
use super::themes::{self, Theme};
use super::{DRoom, Dir, Door, Dungeon, Enemy, Floor, GenOpts, RoomType, COLS, MIDC, MIDR, ROWS, TILE};
use crate::worldgen::rng::Mulberry32;
use std::collections::{HashMap, HashSet};

/// js STEP — growth/BFS neighbour order (n, s, w, e as vectors).
const STEP: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

struct FloorOpt {
    f: usize,
    start_type: RoomType,
    is_last: bool,
    room_count: usize,
    no_locks: bool,
    rift: bool,
    // Maze floors:
    w: i32,
    h: i32,
    gimmick: Option<&'static str>,
    guard: &'static str,
}

fn connect(rooms: &mut HashMap<(i32, i32), DRoom>, ax: i32, ay: i32, bx: i32, by: i32) {
    if !rooms.contains_key(&(ax, ay)) || !rooms.contains_key(&(bx, by)) {
        return;
    }
    let (da, db) = if bx > ax {
        (Dir::E, Dir::W)
    } else if bx < ax {
        (Dir::W, Dir::E)
    } else if by > ay {
        (Dir::S, Dir::N)
    } else {
        (Dir::N, Dir::S)
    };
    rooms.get_mut(&(ax, ay)).unwrap().set_door(da, Door::Open);
    rooms.get_mut(&(bx, by)).unwrap().set_door(db, Door::Open);
}

/// BFS room -> distance from (0,0) over connected doors, in DISCOVERY order (the js Map
/// insertion order feeds the deepest-room tie-breaks).
fn dist_map(rooms: &HashMap<(i32, i32), DRoom>) -> Vec<((i32, i32), i32)> {
    let mut dist: Vec<((i32, i32), i32)> = vec![((0, 0), 0)];
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(0, 0)]);
    let mut qi = 0;
    while qi < dist.len() {
        let ((x, y), d) = dist[qi];
        qi += 1;
        let room = &rooms[&(x, y)];
        for dir in Dir::ALL {
            if room.door(dir) == Door::None {
                continue;
            }
            let (dx, dy) = dir.vec();
            let nk = (x + dx, y + dy);
            if rooms.contains_key(&nk) && seen.insert(nk) {
                dist.push((nk, d + 1));
            }
        }
    }
    dist
}

/// The farthest entry (strict `>` keeps the FIRST max, like the js insertion-order scan).
fn deepest(dist: &[((i32, i32), i32)], skip: &[(i32, i32)]) -> Option<(i32, i32)> {
    let mut best = None;
    let mut bd = -1;
    for &(k, d) in dist {
        if k == (0, 0) || skip.contains(&k) {
            continue;
        }
        if d > bd {
            bd = d;
            best = Some(k);
        }
    }
    best
}

/// Lock/unlock every door of a room (both sides of each edge).
fn lock_room(fl: &mut Floor, k: (i32, i32), grand: bool) {
    for dir in Dir::ALL {
        if fl.rooms[&k].door(dir) == Door::None {
            continue;
        }
        let (dx, dy) = dir.vec();
        let b = ((k.0 + dx, k.1 + dy), dir.opp());
        fl.locked.insert((k, dir));
        fl.locked.insert(b);
        if grand {
            fl.ornate.insert((k, dir));
            fl.ornate.insert(b);
        }
    }
}

fn unlock_room(fl: &mut Floor, k: (i32, i32)) {
    for dir in Dir::ALL {
        if fl.rooms[&k].door(dir) == Door::None {
            continue;
        }
        let (dx, dy) = dir.vec();
        fl.locked.remove(&(k, dir));
        fl.locked.remove(&((k.0 + dx, k.1 + dy), dir.opp()));
    }
}

/// js farthestSafeNormal: BFS that refuses to ENTER avoided (locked) rooms — a key must
/// sit somewhere reachable without it, or the dungeon deadlocks.
fn farthest_safe_normal(fl: &Floor, avoid: &[(i32, i32)]) -> Option<(i32, i32)> {
    let mut sd: Vec<((i32, i32), i32)> = vec![((0, 0), 0)];
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(0, 0)]);
    let mut qi = 0;
    while qi < sd.len() {
        let ((x, y), d) = sd[qi];
        qi += 1;
        let room = &fl.rooms[&(x, y)];
        for dir in Dir::ALL {
            if room.door(dir) == Door::None {
                continue;
            }
            let (dx, dy) = dir.vec();
            let nk = (x + dx, y + dy);
            if !fl.rooms.contains_key(&nk) || avoid.contains(&nk) || !seen.insert(nk) {
                continue;
            }
            sd.push((nk, d + 1));
        }
    }
    let mut best = None;
    let mut bd = -1;
    for &(k, d) in &sd {
        if fl.rooms[&k].rtype == RoomType::Normal && d > bd {
            bd = d;
            best = Some(k);
        }
    }
    best
}

/// Decor everywhere + enemies in the plain rooms (the shared tail of the floor gens).
/// `with_pits` — genFloor keeps foes off pit tiles; the maze gen has no pits.
fn populate(r: &mut Mulberry32, theme: &Theme, fl: &mut Floor, with_pits: bool) {
    let order = fl.order.clone();
    for k in order {
        let room = fl.rooms.get_mut(&k).unwrap();
        place_decor(r, theme.key, room);
        if room.rtype != RoomType::Normal {
            continue;
        }
        let mut occ: HashSet<(i32, i32)> = solid_decor_tiles(&room.decor).into_iter().collect();
        if with_pits {
            for &p in &room.pits {
                occ.insert(p);
            }
        }
        let n = 2 + (r.next_f64() * 3.0) as i32;
        let pool = themes::pool(theme.key);
        for _ in 0..n {
            let (mut c, mut rr);
            let mut tries = 0;
            loop {
                c = 3 + (r.next_f64() * (COLS - 6) as f64) as i32;
                rr = 3 + (r.next_f64() * (ROWS - 6) as f64) as i32;
                tries += 1;
                if !occ.contains(&(c, rr)) || tries >= 10 {
                    break;
                }
            }
            let kind = pool[(r.next_f64() * pool.len() as f64) as usize];
            room.enemies.push(Enemy { kind, x: c * TILE, y: rr * TILE });
        }
    }
}

/// js genFloor: a connected room web grown from (0,0); deepest room = boss (last floor)
/// or the stairs down; next-deepest = the treasure vault; then the keywork chain.
fn gen_floor(r: &mut Mulberry32, theme: &Theme, opt: &FloorOpt) -> Floor {
    let mut rooms = HashMap::new();
    let mut start = DRoom::new(0, 0, opt.start_type);
    start.visited = opt.f == 0;
    start.cleared = true;
    rooms.insert((0, 0), start);
    let mut order = vec![(0, 0)];
    let mut guard = 0;
    while rooms.len() < opt.room_count && guard < 400 {
        guard += 1;
        let (bx, by) = order[(r.next_f64() * order.len() as f64) as usize];
        let (dx, dy) = STEP[(r.next_f64() * 4.0) as usize];
        let (nx, ny) = (bx + dx, by + dy);
        if opt.start_type == RoomType::Start && nx == 0 && ny == 1 {
            continue; // reserve the start room's south wall for the ornate entrance
        }
        if let std::collections::hash_map::Entry::Vacant(e) = rooms.entry((nx, ny)) {
            e.insert(DRoom::new(nx, ny, RoomType::Normal));
            order.push((nx, ny));
        }
        connect(&mut rooms, bx, by, nx, ny);
    }
    let dist = dist_map(&rooms);
    let deep_key = deepest(&dist, &[]);
    let trea_key = deepest(&dist, &deep_key.map(|k| vec![k]).unwrap_or_default());
    if let Some(dk) = deep_key
        && !opt.is_last
    {
        let d = rooms.get_mut(&dk).unwrap();
        d.rtype = RoomType::Stairs; // shallower floors: the farthest room holds the stairs DOWN
        d.cleared = true;
    }
    if let Some(tk) = trea_key
        && rooms[&tk].rtype == RoomType::Normal
    {
        let t = rooms.get_mut(&tk).unwrap();
        t.rtype = RoomType::Treasure;
        t.cleared = true;
        let (cc, cr) = CHEST_SPOTS[(r.next_f64() * CHEST_SPOTS.len() as f64) as usize];
        t.chest = Some((cc * TILE, cr * TILE));
    }
    let mut fl = Floor { order, rooms, locked: HashSet::new(), ornate: HashSet::new(), deep_key, gimmick: None };
    // DUNGEON KEYWORK: the small key waits where you can reach it freely, the small lock
    // guards the vault, and inside sit the chest AND the ornate key for the boss door.
    let trea_is_vault = trea_key.is_some_and(|tk| fl.rooms[&tk].rtype == RoomType::Treasure);
    if opt.rift
        && opt.is_last
        && let Some(dk) = deep_key
    {
        // Rift floors: the deepest room is the champion's arena — no key-hunt, no locks
        // (speed is the rift's rhythm); navigate's runtime arena-seal still slams it.
        fl.rooms.get_mut(&dk).unwrap().rtype = RoomType::Boss;
    }
    if !opt.no_locks && opt.is_last && let Some(dk) = deep_key {
        fl.rooms.get_mut(&dk).unwrap().rtype = RoomType::Boss;
        lock_room(&mut fl, dk, true);
        if trea_is_vault {
            let tk = trea_key.unwrap();
            lock_room(&mut fl, tk, false);
            fl.rooms.get_mut(&tk).unwrap().bosskey = true;
            if let Some(kk) = farthest_safe_normal(&fl, &[dk, tk]) {
                fl.rooms.get_mut(&kk).unwrap().key = true;
            } else {
                unlock_room(&mut fl, tk); // nowhere safe for a small key: the vault stands open
            }
        } else {
            let kk = farthest_safe_normal(&fl, &[dk]).unwrap_or((0, 0));
            fl.rooms.get_mut(&kk).unwrap().bosskey = true;
        }
    } else if !opt.no_locks && deep_key.is_some() && trea_is_vault && r.next_f64() < 0.5 {
        let tk = trea_key.unwrap();
        lock_room(&mut fl, tk, false); // shallower floors: half the vaults are small-locked too
        if let Some(kk) = farthest_safe_normal(&fl, &[tk]) {
            fl.rooms.get_mut(&kk).unwrap().key = true;
        } else {
            unlock_room(&mut fl, tk);
        }
    } else if opt.is_last && let Some(dk) = deep_key {
        fl.rooms.get_mut(&dk).unwrap().rtype = RoomType::Boss; // noLocks (rifts): unbarred
    }
    populate(r, theme, &mut fl, true);
    fl
}

/// js genMazeFloor: a TRUE LABYRINTH — recursive-backtracker spanning tree over a full
/// W x H grid. Farthest room hosts guarded stairs; the farthest dead end hides the prize.
fn gen_maze_floor(r: &mut Mulberry32, theme: &Theme, opt: &FloorOpt) -> Floor {
    let (w, h) = (opt.w, opt.h);
    let mut rooms = HashMap::new();
    let mut order = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rooms.insert((x, y), DRoom::new(x, y, RoomType::Normal));
            order.push((x, y));
        }
    }
    {
        let s = rooms.get_mut(&(0, 0)).unwrap();
        s.rtype = opt.start_type;
        s.visited = opt.f == 0;
        s.cleared = true;
    }
    // The start room's south wall is reserved for the ornate way out: never carve (0,0)-(0,1).
    let bad_edge = |x: i32, y: i32, nx: i32, ny: i32| {
        opt.start_type == RoomType::Start && ((x == 0 && y == 0 && nx == 0 && ny == 1) || (x == 0 && y == 1 && nx == 0 && ny == 0))
    };
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(0, 0)]);
    let mut stack = vec![(0, 0)];
    while let Some(&(x, y)) = stack.last() {
        let open: Vec<(i32, i32)> = STEP
            .iter()
            .filter(|(dx, dy)| {
                let nk = (x + dx, y + dy);
                rooms.contains_key(&nk) && !seen.contains(&nk) && !bad_edge(x, y, nk.0, nk.1)
            })
            .copied()
            .collect();
        if open.is_empty() {
            stack.pop();
            continue;
        }
        let (dx, dy) = open[(r.next_f64() * open.len() as f64) as usize];
        connect(&mut rooms, x, y, x + dx, y + dy);
        seen.insert((x + dx, y + dy));
        stack.push((x + dx, y + dy));
    }
    let dist = dist_map(&rooms);
    let deep_key = deepest(&dist, &[]);
    let mut trea_key = None; // farthest DEAD END (degree 1)
    let mut td = -1;
    for &(k, d) in &dist {
        if k == (0, 0) || Some(k) == deep_key {
            continue;
        }
        let deg = Dir::ALL.iter().filter(|&&q| rooms[&k].door(q) != Door::None).count();
        if deg == 1 && d > td {
            td = d;
            trea_key = Some(k);
        }
    }
    let mut fl = Floor { order, rooms, locked: HashSet::new(), ornate: HashSet::new(), deep_key, gimmick: opt.gimmick };
    if opt.is_last && let Some(dk) = deep_key {
        fl.rooms.get_mut(&dk).unwrap().rtype = RoomType::Boss; // the Choirmaster, behind the ORNATE door
        lock_room(&mut fl, dk, true);
        let kk = trea_key.unwrap_or((0, 0));
        if fl.rooms.contains_key(&kk) {
            fl.rooms.get_mut(&kk).unwrap().bosskey = true; // the ornate key in the deepest dead end
        }
    } else if let Some(dk) = deep_key {
        let d = fl.rooms.get_mut(&dk).unwrap();
        d.rtype = RoomType::Stairs; // guarded stairs: fight (or dodge) the warden
        d.enemies = vec![Enemy { kind: opt.guard, x: (MIDC + 2) * TILE, y: (MIDR - 1) * TILE }];
        if let Some(tk) = trea_key {
            let t = fl.rooms.get_mut(&tk).unwrap();
            t.rtype = RoomType::Treasure;
            t.chest = Some((MIDC * TILE, MIDR * TILE));
            t.cleared = true;
        }
    }
    populate(r, theme, &mut fl, false);
    fl
}

/// js genMirrorFloor — the Saltmaze's LOST WOODS: one haze-hall where every exit repeats
/// the hall unless your feet sing the Maze Song (the app owns the step-tracking).
fn gen_mirror_floor(r: &mut Mulberry32, theme: &Theme, opt: &FloorOpt) -> Floor {
    let mut rooms = HashMap::new();
    let mut arrival = DRoom::new(0, 0, opt.start_type);
    arrival.visited = opt.f == 0;
    arrival.cleared = true;
    arrival.set_door(Dir::E, Door::Open); // the honest way back out
    let mut hall = DRoom::new(1, 0, RoomType::Normal);
    hall.cleared = true;
    hall.mirror = true; // the repeating hall (kept enemy-free: it's a riddle, not a fight)
    for d in [Dir::W, Dir::N, Dir::S, Dir::E] {
        hall.set_door(d, Door::Open); // every way LOOKS open
    }
    let mut stairs = DRoom::new(1, 1, RoomType::Stairs);
    stairs.set_door(Dir::N, Door::Open); // the true way in (the song's last step)
    stairs.enemies = vec![Enemy { kind: opt.guard, x: (MIDC + 2) * TILE, y: (MIDR - 1) * TILE }];
    rooms.insert((0, 0), arrival);
    rooms.insert((1, 0), hall);
    rooms.insert((1, 1), stairs);
    let mut fl = Floor {
        order: vec![(0, 0), (1, 0), (1, 1)],
        rooms,
        locked: HashSet::new(),
        ornate: HashSet::new(),
        deep_key: Some((1, 1)),
        gimmick: Some("mirror"),
    };
    let order = fl.order.clone();
    for k in order {
        place_decor(r, theme.key, fl.rooms.get_mut(&k).unwrap()); // (entry/stairs stay clean)
    }
    fl
}

/// js genGuildhallFloor — a great hall TWO ROOMS TALL (joined by a 'wide' opening) with
/// the five guild wings branching off it. Peaceful: no enemies, every room pre-cleared.
fn gen_guildhall_floor(r: &mut Mulberry32, theme: &Theme) -> Floor {
    let mut rooms = HashMap::new();
    let mut order = Vec::new();
    let add = |rx: i32, ry: i32, rtype: RoomType, doors: &[(Dir, Door)], gwing: Option<&'static str>, rooms: &mut HashMap<(i32, i32), DRoom>, order: &mut Vec<(i32, i32)>| {
        let mut d = DRoom::new(rx, ry, rtype);
        d.visited = rx == 0 && ry == 0;
        d.cleared = true;
        d.gwing = gwing;
        for &(dir, k) in doors {
            d.set_door(dir, k);
        }
        rooms.insert((rx, ry), d);
        order.push((rx, ry));
    };
    add(0, 0, RoomType::Start, &[(Dir::N, Door::Wide), (Dir::E, Door::Open), (Dir::W, Door::Open)], None, &mut rooms, &mut order);
    add(0, -1, RoomType::Normal, &[(Dir::S, Door::Wide), (Dir::E, Door::Open), (Dir::W, Door::Open), (Dir::N, Door::Open)], None, &mut rooms, &mut order);
    add(-1, 0, RoomType::Normal, &[(Dir::E, Door::Open)], Some("tillers"), &mut rooms, &mut order);
    add(1, 0, RoomType::Normal, &[(Dir::W, Door::Open)], Some("anglers"), &mut rooms, &mut order);
    add(-1, -1, RoomType::Normal, &[(Dir::E, Door::Open)], Some("smiths"), &mut rooms, &mut order);
    add(1, -1, RoomType::Normal, &[(Dir::W, Door::Open)], Some("scholars"), &mut rooms, &mut order);
    add(0, -2, RoomType::Normal, &[(Dir::S, Door::Open)], Some("provisioners"), &mut rooms, &mut order);
    let mut fl = Floor { order, rooms, locked: HashSet::new(), ornate: HashSet::new(), deep_key: None, gimmick: None };
    let order = fl.order.clone();
    for k in order {
        let room = fl.rooms.get_mut(&k).unwrap();
        place_decor(r, theme.key, room); // dusty grandeur, nothing hostile
        room.enemies.clear();
    }
    fl
}

/// js Dungeon.generate — build the floor stack and link the stairs.
/// Chest anchor tiles — needn't sit dead-centre; corners/walls for variety, clear of
/// door lanes. Shared by the treasure room's chest and the mimic's fake one (using the
/// SAME table is part of the disguise: a mimic sits exactly where a chest would).
const CHEST_SPOTS: [(i32, i32); 9] = [
    (MIDC, MIDR),
    (2, 2),
    (COLS - 3, 2),
    (2, ROWS - 3),
    (COLS - 3, ROWS - 3),
    (4, 2),
    (COLS - 5, 2),
    (4, ROWS - 3),
    (COLS - 5, ROWS - 3),
];

pub fn generate(seed: u32, biome_theme: &str, opts: &GenOpts) -> Dungeon {
    let mut r = Mulberry32::new(seed);
    let theme = themes::theme(biome_theme); // unknown keys fall back to 'cave', like the js
    if opts.guildhall {
        return Dungeon { floors: vec![gen_guildhall_floor(&mut r, theme)], floor: 0, theme };
    }
    let num_floors = opts.floors.unwrap_or_else(|| {
        1 + usize::from(r.next_f64() < 0.6) + usize::from(r.next_f64() < 0.35) // size-scaled 1..3
    });
    // The Saltmaze's descent: galleries -> darkness -> the chant -> mirror halls -> the sanctum.
    const MAZE_GIMMICKS: [Option<&str>; 5] = [None, Some("dark"), Some("chant"), Some("mirror"), None];
    const MAZE_GUARDS: [&str; 4] = ["golem", "ogre", "revenant", "charbrute"];
    const MAZE_SIZE: [(i32, i32); 5] = [(4, 3), (4, 3), (5, 3), (5, 4), (4, 3)];
    let mut floors = Vec::new();
    for f in 0..num_floors {
        let mut o = FloorOpt {
            f,
            start_type: if f == 0 { RoomType::Start } else { RoomType::Arrival },
            is_last: f == num_floors - 1,
            room_count: 0,
            no_locks: opts.no_locks,
            rift: opts.rift,
            w: 4,
            h: 3,
            gimmick: None,
            guard: MAZE_GUARDS[f % MAZE_GUARDS.len()],
        };
        floors.push(if opts.maze {
            if MAZE_GIMMICKS.get(f).copied().flatten() == Some("mirror") {
                gen_mirror_floor(&mut r, theme, &o)
            } else {
                let (w, h) = MAZE_SIZE.get(f).copied().unwrap_or((4, 3));
                o.w = w;
                o.h = h;
                o.gimmick = MAZE_GIMMICKS.get(f).copied().flatten();
                gen_maze_floor(&mut r, theme, &o)
            }
        } else {
            o.room_count = opts.room_count.unwrap_or_else(|| 6 + (r.next_f64() * 4.0) as usize);
            gen_floor(&mut r, theme, &o)
        });
    }
    // Link each floor's stairs-down to the next floor's arrival (0,0).
    for f in 0..num_floors.saturating_sub(1) {
        let Some(down) = floors[f].deep_key else { continue };
        floors[f].rooms.get_mut(&down).unwrap().stairs_down = Some((0, 0));
        floors[f + 1].rooms.get_mut(&(0, 0)).unwrap().stairs_up = Some(down);
    }
    // MIMICS — Baz's redesign of the js one (game.js: a 16% roll swapped the treasure
    // chest for a visibly-different mob, which spoiled the trick on sight). Here ~5% of
    // PLAIN rooms — never one that holds a real chest or key — grow a pixel-perfect fake
    // at a regular chest anchor: a bonus chest is either luck or teeth. The roll and the
    // spot come off a standalone hash (NOT the gen rng) so existing seeds keep their
    // layouts and re-entry can never reroll it.
    for (f, fl) in floors.iter_mut().enumerate() {
        for (&(rx, ry), room) in fl.rooms.iter_mut() {
            if room.rtype != RoomType::Normal
                || room.chest.is_some()
                || room.key
                || room.bosskey
                || room.secret.is_some()
                || room.mirror
                || room.gwing.is_some()
            {
                continue;
            }
            let h = crate::worldgen::rng::hash(seed, rx, ry, 0x4d49_4d43 ^ (f as u32 + 1).wrapping_mul(52361));
            if h % 100 >= 5 {
                continue;
            }
            let occ: HashSet<(i32, i32)> = solid_decor_tiles(&room.decor).into_iter().collect();
            let start = (h >> 8) as usize % CHEST_SPOTS.len();
            for i in 0..CHEST_SPOTS.len() {
                let (cc, cr) = CHEST_SPOTS[(start + i) % CHEST_SPOTS.len()];
                if occ.contains(&(cc, cr)) || room.pits.contains(&(cc, cr)) {
                    continue;
                }
                room.mimic = Some((cc * TILE, cr * TILE));
                break;
            }
        }
    }
    // HIDDEN VAULTS: every push-block secret (place_decor's ~15% roll) gets its
    // sealed side-room in the same floor's map, keyed off-grid at parent+(100,100)
    // (deterministic — the ledger re-finds it across visits). Doors: none; contents:
    // one chest with the js secret-cache roll + the stairs back up.
    for fl in floors.iter_mut() {
        let parents: Vec<(i32, i32)> = fl
            .rooms
            .iter()
            .filter(|(_, r)| r.secret.is_some())
            .map(|(&k, _)| k)
            .collect();
        for (px, py) in parents {
            let vk = (px + 100, py + 100);
            let mut v = DRoom::new(vk.0, vk.1, RoomType::Normal);
            v.vault = true;
            v.vault_of = Some((px, py));
            v.cleared = true;
            v.chest = Some((9 * TILE, 5 * TILE));
            fl.rooms.insert(vk, v);
            fl.rooms.get_mut(&(px, py)).unwrap().vault_key = Some(vk);
        }
    }
    Dungeon { floors, floor: 0, theme }
}
