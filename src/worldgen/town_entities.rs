//! town_entities.rs — a town room's building/prop/villager layout (port of
//! `townEntities` in js/world.js). DETERMINISM RULES APPLY: every rng draw runs in the
//! exact JS order (a shifted stream moves every placement after it), and the shared
//! `used` set keeps streams off each other's tiles. Pinned descriptor-for-descriptor by
//! tests/town_entities_parity.rs against live-JS golden vectors.

use super::entities::RoomEntity;
use super::rng::{hash, Mulberry32};
use super::towns::TownRole;
use super::world::{World, COLS, ROWS};
use crate::room::TILE;

const SALT_TOWN: u32 = 0x77ab;

/// js TOWN_LINES — the villagers' ambient chatter pool.
pub const TOWN_LINES: [&str; 10] = [
    "WELCOME, TRAVELER!",
    "NICE WEATHER TODAY.",
    "BEWARE THE DUNGEONS.",
    "THE INN HAS A WARM BED.",
    "STAY A WHILE.",
    "I HEARD GOBLINS TO THE EAST.",
    "THE BLACKSMITH DOES FINE WORK.",
    "LOVELY DAY FOR A STROLL.",
    "MIND THE MONSTERS OUT THERE.",
    "SPARE SOME COIN?",
];

/// js Buildings.TOWN_KINDS — every kind except the inn (placed separately) and the
/// producestall (a restoration perk), in the DEFS declaration order (shuffles walk it).
const TOWN_KINDS: [&str; 17] = [
    "store", "blacksmith", "armory", "magic", "alchemist", "jeweler", "fletcher", "trader",
    "farmstall", "church", "temple", "tavern", "townhall", "library", "bakery", "home", "cottage",
];

/// One district's recipe (js D) — building count range, lead + pool, and dressing.
struct Recipe {
    min: i32,
    max: i32,
    lead: Option<&'static str>,
    pool: Vec<&'static str>,
    well: bool,
    torches: i32,
    npcs: (i32, i32),
    deco: i32,
    orchard: i32,
    fields: bool,
}

fn recipe(role: TownRole) -> Recipe {
    let base = Recipe {
        min: 0,
        max: 0,
        lead: None,
        pool: vec![],
        well: false,
        torches: 0,
        npcs: (0, 0),
        deco: 0,
        orchard: 0,
        fields: false,
    };
    match role {
        TownRole::Market => Recipe {
            min: 4,
            max: 7,
            lead: Some("inn"),
            pool: TOWN_KINDS.iter().copied().chain(["home", "cottage", "home"]).collect(),
            well: true,
            torches: 3,
            npcs: (3, 6),
            deco: 12,
            ..base
        },
        TownRole::Homes => Recipe {
            min: 5,
            max: 7,
            pool: vec!["home", "cottage", "home", "cottage", "home", "bakery", "cottage"],
            torches: 2,
            npcs: (4, 7),
            deco: 14,
            ..base
        },
        TownRole::Green => Recipe {
            min: 2,
            max: 3,
            lead: Some("church"),
            pool: vec!["cottage", "home"],
            torches: 1,
            npcs: (2, 3),
            deco: 10,
            orchard: 7,
            ..base
        },
        TownRole::Farmrow => Recipe {
            min: 2,
            max: 3,
            lead: Some("farmstall"),
            pool: vec!["cottage", "home"],
            torches: 1,
            npcs: (2, 4),
            deco: 6,
            fields: true,
            ..base
        },
        TownRole::Quarter => Recipe {
            min: 3,
            max: 5,
            lead: Some("tavern"),
            pool: vec!["bakery", "library", "townhall", "home", "cottage"],
            torches: 4,
            npcs: (3, 5),
            deco: 8,
            ..base
        },
        TownRole::Yards => Recipe {
            min: 1,
            max: 2,
            pool: vec!["cottage", "home"],
            torches: 1,
            npcs: (1, 2),
            deco: 12,
            orchard: 4,
            ..base
        },
        TownRole::Hall => Recipe { torches: 2, npcs: (1, 2), deco: 8, orchard: 3, ..base },
    }
}

impl World {
    /// Port of `townEntities(rx, ry)` — see the module docs for the determinism rules.
    pub(super) fn town_entities(&self, rx: i32, ry: i32) -> Vec<RoomEntity> {
        let role = self.town_role(rx, ry).unwrap_or(TownRole::Market);
        let room = self.generate(rx, ry);
        let map = &room.map;
        let ground_at = |c: i32, r: i32| -> bool {
            (0..ROWS).contains(&r)
                && (0..COLS).contains(&c)
                && map[r as usize].as_bytes().get(c as usize) == Some(&b'.')
        };
        let mut rng = Mulberry32::new(hash(self.seed, rx, ry, SALT_TOWN));
        let mut out: Vec<RoomEntity> = Vec::new();
        let mut used: Vec<bool> = vec![false; (COLS * ROWS) as usize];
        let key = |c: i32, r: i32| (r * COLS + c) as usize;
        // claim(): a building footprint owns a 3x4 patch of tiles around its plot.
        let claim = |used: &mut Vec<bool>, c: i32, r: i32| {
            for dc in -1..=1 {
                for dr in -2..=1 {
                    let (cc, rr) = (c + dc, r + dr);
                    if (0..COLS).contains(&cc) && (0..ROWS).contains(&rr) {
                        used[key(cc, rr)] = true;
                    }
                }
            }
        };
        // Fisher-Yates, the js loop verbatim (draw order matters).
        fn shuffle<T>(a: &mut [T], rng: &mut Mulberry32) {
            for i in (1..a.len()).rev() {
                let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
                a.swap(i, j);
            }
        }

        let mut plots: Vec<(i32, i32)> =
            vec![(3, 3), (7, 3), (11, 3), (15, 3), (3, 9), (7, 9), (11, 9), (15, 9)];
        shuffle(&mut plots, &mut rng);

        let mut d = recipe(role);
        if role == TownRole::Green && rng.next_f64() < 0.5 {
            d.lead = Some("temple"); // half the greens keep a temple instead
        }
        let n_bld = d.min + (rng.next_f64() * (d.max - d.min + 1) as f64).floor() as i32;
        let mut pool = d.pool.clone();
        shuffle(&mut pool, &mut rng);
        let mut kinds: Vec<&str> = d.lead.into_iter().collect();
        let take = (n_bld - d.lead.is_some() as i32).max(0) as usize;
        kinds.extend(pool.iter().copied().filter(|k| Some(*k) != d.lead).take(take));

        for (i, kind) in kinds.iter().enumerate().take(plots.len()) {
            let (c, r) = plots[i];
            out.push(RoomEntity {
                kind: "town",
                sub: (*kind).into(),
                x: c * TILE,
                y: r * TILE,
                seed: 0,
                champ: false,
                elite: false,
            });
            claim(&mut used, c, r);
        }
        // The hall district: the boarded-up GUILDHALL dominates its grounds.
        if role == TownRole::Hall {
            out.push(RoomEntity {
                kind: "guildhall",
                sub: String::new(),
                x: 6 * TILE,
                y: 3 * TILE,
                seed: 0,
                champ: false,
                elite: false,
            });
            for c in 4..=13 {
                for r in 1..=7 {
                    used[key(c, r)] = true;
                }
            }
        }
        // The market square keeps its fountain centrepiece + flanking braziers.
        if d.well {
            let (ccx, ccy) = (COLS >> 1, ROWS >> 1);
            out.push(ent_at("well", ccx, ccy));
            claim(&mut used, ccx, ccy);
            for (tc, tr) in [(ccx - 1, ccy - 1), (ccx + 1, ccy - 1)] {
                if ground_at(tc, tr) {
                    out.push(ent_at("torch", tc, tr));
                    used[key(tc, tr)] = true;
                }
            }
        }
        // A chapel green (or a city yard) grows an orchard of shade trees.
        if d.orchard > 0 {
            let (mut n_o, mut ot) = (0, 0);
            while n_o < d.orchard && ot < 80 {
                ot += 1;
                let c = 2 + (rng.next_f64() * (COLS - 4) as f64).floor() as i32;
                let r = 2 + (rng.next_f64() * (ROWS - 4) as f64).floor() as i32;
                if !ground_at(c, r) || used[key(c, r)] {
                    continue;
                }
                used[key(c, r)] = true;
                out.push(ent_at("oak", c, r));
                n_o += 1;
            }
        }
        // Farm rows: tidy east-west bands of worked bushes with the odd fallow row.
        if d.fields {
            for r in [4, 7, 10] {
                if rng.next_f64() < 0.25 {
                    continue;
                }
                for c in 3..COLS - 3 {
                    if !ground_at(c, r) || used[key(c, r)] {
                        continue;
                    }
                    if rng.next_f64() < 0.7 {
                        used[key(c, r)] = true;
                        out.push(ent_at("bush", c, r));
                    }
                }
            }
        }
        // Street braziers on open ground, per the district's night life.
        let (mut n_t, mut tt) = (0, 0);
        while n_t < d.torches && tt < 60 {
            tt += 1;
            let c = 2 + (rng.next_f64() * (COLS - 4) as f64).floor() as i32;
            let r = 2 + (rng.next_f64() * (ROWS - 4) as f64).floor() as i32;
            if !ground_at(c, r) || used[key(c, r)] {
                continue;
            }
            used[key(c, r)] = true;
            out.push(ent_at("torch", c, r));
            n_t += 1;
        }
        // The district's folk (each a stable, nameable person — their identity is `seed`).
        let (mut placed, mut tries) = (0, 0);
        let n_npc = d.npcs.0 + (rng.next_f64() * (d.npcs.1 - d.npcs.0 + 1) as f64).floor() as i32;
        while placed < n_npc && tries < 60 {
            tries += 1;
            let c = 4 + (rng.next_f64() * 11.0).floor() as i32;
            let r = 5 + (rng.next_f64() * 6.0).floor() as i32;
            if !ground_at(c, r) || used[key(c, r)] {
                continue;
            }
            used[key(c, r)] = true;
            let line = TOWN_LINES[(rng.next_f64() * TOWN_LINES.len() as f64).floor() as usize];
            out.push(RoomEntity {
                kind: "npc",
                sub: line.into(),
                x: c * TILE,
                y: r * TILE,
                seed: hash(self.seed, rx * 31 + c, ry * 17 + r, SALT_TOWN),
                champ: false,
                elite: false,
            });
            placed += 1;
        }
        // Decorative greenery on open ground (never the streets — groundAt is false there).
        let (mut deco, mut dt) = (0, 0);
        while deco < d.deco && dt < 90 {
            dt += 1;
            let c = 1 + (rng.next_f64() * (COLS - 2) as f64).floor() as i32;
            let r = 1 + (rng.next_f64() * (ROWS - 2) as f64).floor() as i32;
            if !ground_at(c, r) || used[key(c, r)] {
                continue;
            }
            used[key(c, r)] = true;
            out.push(ent_at(if rng.next_f64() < 0.72 { "flower" } else { "bush" }, c, r));
            deco += 1;
        }
        // Markets reserve a pitch for the Tillers' produce stall — the first unused plot
        // whose footprint stayed clear, chosen LAST with no rng draws (layout stability).
        if role == TownRole::Market {
            let in_foot = |x: i32, y: i32, c: i32, r: i32| {
                let (oc, or) = (x / TILE, y / TILE);
                oc >= c - 1 && oc <= c + 1 && or >= r - 2 && or <= r + 1
            };
            let solid = |k: &str| matches!(k, "town" | "well" | "guildhall");
            let pick = |out: &mut Vec<RoomEntity>, used: &mut Vec<bool>, allow_torch: bool, allow_npc: bool| -> bool {
                for (c, r) in plots.iter().skip(kinds.len()).copied() {
                    let blocked = out.iter().any(|o| {
                        (solid(o.kind)
                            || (!allow_torch && o.kind == "torch")
                            || (!allow_npc && o.kind == "npc"))
                            && in_foot(o.x, o.y, c, r)
                    });
                    if blocked {
                        continue;
                    }
                    let mut j = out.len();
                    while j > 0 {
                        j -= 1;
                        let o = &out[j];
                        if solid(o.kind) || !in_foot(o.x, o.y, c, r) {
                            continue;
                        }
                        if o.kind == "npc" {
                            // Step the villager to the nearest open ground (identity
                            // rides on `seed`, not position).
                            let mut moved = false;
                            'scan: for rr in 5..=10 {
                                for cc in 4..=14 {
                                    if !ground_at(cc, rr)
                                        || used[key(cc, rr)]
                                        || in_foot(cc * TILE, rr * TILE, c, r)
                                    {
                                        continue;
                                    }
                                    used[key(cc, rr)] = true;
                                    out[j].x = cc * TILE;
                                    out[j].y = rr * TILE;
                                    moved = true;
                                    break 'scan;
                                }
                            }
                            if !moved {
                                out.remove(j); // nowhere to stand — they stay home today
                            }
                        } else {
                            out.remove(j); // greenery/braziers are swept from the pitch
                        }
                    }
                    claim(used, c, r);
                    out.push(ent_at("stallspot", c, r));
                    return true;
                }
                false
            };
            if !pick(&mut out, &mut used, false, false) && !pick(&mut out, &mut used, true, false) {
                pick(&mut out, &mut used, true, true); // keep braziers, then villagers, when we can
            }
        }
        out
    }
}

fn ent_at(kind: &'static str, c: i32, r: i32) -> RoomEntity {
    RoomEntity { kind, sub: String::new(), x: c * TILE, y: r * TILE, seed: 0, champ: false, elite: false }
}
