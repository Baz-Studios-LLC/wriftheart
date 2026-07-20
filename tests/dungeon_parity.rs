//! dungeon_parity.rs — pins src/dungeon's generator bit-exact against the live js
//! (fixtures from tools/extract_dungeon.mjs). Every room, door, lock, decor prop, pit
//! and enemy across seven seed/theme/opts cases — one rng-order slip fails loudly.

use wriftheart::dungeon::{self, Dir, Door, GenOpts, RoomType};

include!("data/dungeon_golden.rs");

fn opts(floors: Option<usize>, room_count: Option<usize>, no_locks: bool, maze: bool, guildhall: bool) -> GenOpts {
    GenOpts { floors, room_count, no_locks, maze, guildhall, rift: false }
}

fn type_str(t: RoomType) -> &'static str {
    match t {
        RoomType::Start => "start",
        RoomType::Arrival => "arrival",
        RoomType::Normal => "normal",
        RoomType::Stairs => "stairs",
        RoomType::Treasure => "treasure",
        RoomType::Boss => "boss",
    }
}

fn dir_str(d: Dir) -> &'static str {
    match d {
        Dir::N => "n",
        Dir::S => "s",
        Dir::W => "w",
        Dir::E => "e",
    }
}

/// Rebuild the extractor's flat lines from a generated dungeon.
fn dump(id: &str, d: &dungeon::Dungeon) -> Vec<String> {
    let mut lines = vec![format!("case {id} theme={} floors={}", d.theme.key, d.floors.len())];
    for (f, fl) in d.floors.iter().enumerate() {
        // Hidden-vault rooms are an rs-only extension (the js secret leads to a
        // transient side-scroll room, never a rooms-map entry) — the golden pins
        // everything the js generates, so the dump skips them. The vault pass
        // consumes no rng, so every js-derived line stays bit-exact.
        let mut keys: Vec<(i32, i32)> = fl.rooms.iter().filter(|(_, r)| !r.vault).map(|(&k, _)| k).collect();
        keys.sort_by_key(|&(x, y)| (y, x));
        for (rx, ry) in keys {
            let r = &fl.rooms[&(rx, ry)];
            let dch = |dir: Dir| match r.door(dir) {
                Door::None => '-',
                Door::Open => 'o',
                Door::Wide => 'w',
            };
            let doors: String = [Dir::N, Dir::S, Dir::W, Dir::E].into_iter().map(dch).collect();
            let ch = r.chest.map_or("-".into(), |(x, y)| format!("{x},{y}"));
            let pits = if r.pits.is_empty() {
                "-".into()
            } else {
                r.pits.iter().map(|(c, rr)| format!("{c},{rr}")).collect::<Vec<_>>().join(";")
            };
            let dn = r.stairs_down.map_or("-".into(), |(x, y)| format!("{x},{y}"));
            let up = r.stairs_up.map_or("-".into(), |(x, y)| format!("{x},{y}"));
            lines.push(format!(
                "{id} f{f} r{rx},{ry} t={} d={doors} ch={ch} k={} bk={} sec={} pits={pits} dn={dn} up={up} gw={} mir={}",
                type_str(r.rtype),
                r.key as u8,
                r.bosskey as u8,
                r.secret.map_or("-".into(), |(c, rr)| format!("{c},{rr}")),
                r.gwing.unwrap_or("-"),
                r.mirror as u8,
            ));
            let dec = if r.decor.is_empty() {
                "-".into()
            } else {
                r.decor
                    .iter()
                    .map(|dd| {
                        format!(
                            "{}@{},{}{}{}",
                            dd.kind,
                            dd.c,
                            dd.r,
                            if dd.detail { "*" } else { "" },
                            dd.corner.map(|c| format!(":{c}")).unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(";")
            };
            lines.push(format!("{id} f{f} dec r{rx},{ry} {dec}"));
            let foes = if r.enemies.is_empty() {
                "-".into()
            } else {
                r.enemies.iter().map(|e| format!("{}@{},{}", e.kind, e.x, e.y)).collect::<Vec<_>>().join(";")
            };
            lines.push(format!("{id} f{f} foe r{rx},{ry} {foes}"));
        }
        // The js sorts "rx,ry:dir" strings lexicographically — reproduce that exact order.
        let mut locks: Vec<String> = fl.locked.iter().map(|((x, y), dir)| format!("{x},{y}:{}", dir_str(*dir))).collect();
        locks.sort();
        let locks = if locks.is_empty() {
            "-".into()
        } else {
            locks
                .into_iter()
                .map(|k| {
                    let parts: Vec<&str> = k.rsplitn(2, ':').collect();
                    let (coord, dir) = (parts[1], parts[0]);
                    let (x, y) = coord.split_once(',').unwrap();
                    let key = ((x.parse().unwrap(), y.parse().unwrap()), match dir {
                        "n" => Dir::N,
                        "s" => Dir::S,
                        "w" => Dir::W,
                        _ => Dir::E,
                    });
                    format!("{k}{}", if fl.ornate.contains(&key) { "!" } else { "" })
                })
                .collect::<Vec<_>>()
                .join(";")
        };
        lines.push(format!("{id} f{f} lock {locks}"));
        lines.push(format!(
            "{id} f{f} deep {} gim {}",
            fl.deep_key.map_or("-".into(), |(x, y)| format!("{x},{y}")),
            fl.gimmick.unwrap_or("-"),
        ));
    }
    lines
}

#[test]
fn generator_matches_js() {
    let cases: Vec<(&str, u32, &str, GenOpts)> = vec![
        ("crypt2", 1337, "crypt", opts(Some(2), None, false, false, false)),
        ("minicave", 42, "cave", opts(Some(1), Some(5), false, false, false)),
        ("saltmaze", 99, "saltmaze", opts(Some(5), None, false, true, false)),
        ("guildhall", 7, "guildhall", opts(None, None, false, false, true)),
        ("rift", 2024, "riftvault", opts(Some(1), Some(9), true, false, false)),
        ("ruinsdefault", 555, "ruins", opts(None, None, false, false, false)),
        ("tomb4", 31337, "tomb", opts(Some(4), None, false, false, false)),
    ];
    let mut ours = Vec::new();
    for (id, seed, theme, o) in cases {
        ours.extend(dump(id, &dungeon::generate(seed, theme, &o)));
    }
    assert_eq!(ours.len(), DUNGEON_GOLDEN.len(), "line counts differ");
    for (i, (a, b)) in ours.iter().zip(DUNGEON_GOLDEN.iter()).enumerate() {
        assert_eq!(a, b, "line {i} diverges");
    }
}

#[test]
fn mimics_only_trick_in_plain_chestless_rooms() {
    // The redesigned mimic (see PORT.md): its whole trick depends on invariants —
    // it may ONLY grow in a plain room holding no real chest or key (so treasure
    // chests stay trustworthy), on a clear tile, and the same seed must always grow
    // the same mimics (re-entry can never reroll the trick).
    let mut total = 0;
    for seed in [1337u32, 42, 99, 2024, 555, 31337, 7, 123456, 777, 90210] {
        let o = opts(Some(3), None, false, false, false);
        let a = dungeon::generate(seed, "crypt", &o);
        let b = dungeon::generate(seed, "crypt", &o);
        for (fa, fb) in a.floors.iter().zip(b.floors.iter()) {
            for (k, ra) in &fa.rooms {
                assert_eq!(ra.mimic, fb.rooms[k].mimic, "mimic spot must be seed-stable");
                let Some((mx, my)) = ra.mimic else { continue };
                total += 1;
                assert_eq!(ra.rtype, RoomType::Normal, "mimic outside a plain room");
                assert!(ra.chest.is_none() && !ra.key && !ra.bosskey, "mimic beside real treasure");
                assert!(ra.secret.is_none(), "mimic in a push-block room");
                assert!(!ra.pits.contains(&(mx / 16, my / 16)), "mimic on a pit");
            }
        }
    }
    assert!(total > 0, "no mimic across 10 seeds x 3 floors - the roll is dead");
}

#[test]
fn vaults_pair_with_secrets() {
    // Every push-block secret hides exactly one sealed vault (rs-only extension):
    // deterministic key, back-linked, doorless, holding the cache chest.
    for seed in [1337u32, 42, 99, 555, 31337] {
        let d = dungeon::generate(seed, "crypt", &opts(Some(3), None, false, false, false));
        for fl in &d.floors {
            for (&k, r) in &fl.rooms {
                if r.secret.is_some() && !r.vault {
                    let vk = r.vault_key.expect("secret room missing its vault");
                    assert_eq!(vk, (k.0 + 100, k.1 + 100), "vault key shape");
                    let v = &fl.rooms[&vk];
                    assert!(v.vault, "vault flag");
                    assert_eq!(v.vault_of, Some(k), "vault back-link");
                    assert!(v.chest.is_some(), "vault cache chest");
                    for dir in [Dir::N, Dir::S, Dir::W, Dir::E] {
                        assert_eq!(v.door(dir), Door::None, "vaults are sealed");
                    }
                }
            }
        }
    }
}
