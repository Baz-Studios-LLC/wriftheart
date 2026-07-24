//! guildhall.rs — the five guilds and their restoration BUNDLES (Baz: Coral Island
//! museum scale — "you should have to submit a TON of bundles and the bundles
//! should be big"). Every CITY keeps a boarded guildhall; inside, each wing hangs
//! a SET of named bundles, each a themed haul with chunky counts. Every filled
//! bundle pays its own reward; filling a wing's WHOLE set brings that guild home —
//! the city-wide perk and the wing's capstone prize (app/guildhall.rs wires both).

/// What counts for a requirement line (js match closures, as data).
#[derive(Clone, Copy)]
pub enum ReqMatch {
    Kind(&'static str),
    Ids(&'static [&'static str]),
    RareFish,
    Dish,
}

pub struct Req {
    pub label: &'static str,
    pub n: i32,
    pub matches: ReqMatch,
}

/// One named donation set. `id` keys the save — unique across EVERY wing.
pub struct Bundle {
    pub id: &'static str,
    pub name: &'static str,
    pub reqs: &'static [Req],
    /// The bundle's own thank-you: (item id, qty).
    pub reward: (&'static str, i32),
}

pub struct Wing {
    pub id: &'static str,
    pub name: &'static str,
    pub crest: u32,
    pub desc: &'static str,
    pub bundles: &'static [Bundle],
    pub perk_desc: &'static str,
    pub loot_desc: &'static str,
}

pub static WINGS: [Wing; 5] = [
    Wing {
        id: "tillers", name: "THE TILLERS", crest: 0x7ee08a,
        desc: "THE FARMERS GUILD - THEY MADE THE VALLEYS FEED THE TOWNS.",
        bundles: &[
            Bundle {
                id: "firstfurrow", name: "THE FIRST FURROW",
                reqs: &[
                    Req { label: "ANY CROPS", n: 12, matches: ReqMatch::Kind("CROP") },
                    Req { label: "SEED PACKETS", n: 6, matches: ReqMatch::Kind("SEED") },
                    Req { label: "FRESH EGGS", n: 4, matches: ReqMatch::Ids(&["egg"]) },
                ],
                reward: ("tomatoseed", 3),
            },
            Bundle {
                id: "marketgarden", name: "THE MARKET GARDEN",
                reqs: &[
                    Req { label: "ANY CROPS", n: 20, matches: ReqMatch::Kind("CROP") },
                    Req { label: "FRESH EGGS", n: 8, matches: ReqMatch::Ids(&["egg"]) },
                    Req { label: "PAILS OF MILK", n: 6, matches: ReqMatch::Ids(&["milk"]) },
                ],
                reward: ("pumpkinseed", 3),
            },
            Bundle {
                id: "harvesttithe", name: "THE HARVEST TITHE",
                reqs: &[
                    Req { label: "ANY CROPS", n: 30, matches: ReqMatch::Kind("CROP") },
                    Req { label: "SEED PACKETS", n: 10, matches: ReqMatch::Kind("SEED") },
                    Req { label: "COOKED DISHES", n: 2, matches: ReqMatch::Dish },
                ],
                reward: ("cranberryseed", 4),
            },
        ],
        perk_desc: "A PRODUCE STALL OPENS IN THE MARKET",
        loot_desc: "A PACKET OF RARE SEEDS",
    },
    Wing {
        id: "anglers", name: "THE ANGLERS", crest: 0x7090d8,
        desc: "THE FISHERS GUILD - EVERY RIVER KNEW THEIR LINES.",
        bundles: &[
            Bundle {
                id: "dailycatch", name: "THE DAILY CATCH",
                reqs: &[Req { label: "ANY FISH", n: 10, matches: ReqMatch::Kind("FISH") }],
                reward: ("potion", 2),
            },
            Bundle {
                id: "riversurvey", name: "THE RIVER SURVEY",
                reqs: &[
                    Req { label: "ANY FISH", n: 16, matches: ReqMatch::Kind("FISH") },
                    Req { label: "RARE CATCHES", n: 2, matches: ReqMatch::RareFish },
                ],
                reward: ("greaterpotion", 1),
            },
            Bundle {
                id: "deeplegend", name: "THE LEGEND OF THE DEEP",
                reqs: &[
                    Req { label: "ANY FISH", n: 24, matches: ReqMatch::Kind("FISH") },
                    Req { label: "RARE CATCHES", n: 5, matches: ReqMatch::RareFish },
                ],
                reward: ("gem", 2),
            },
        ],
        perk_desc: "THE MARKET PAYS EXTRA FOR FISH HERE",
        loot_desc: "THE ANGLERS LUCKY HOOK",
    },
    Wing {
        id: "smiths", name: "THE SMITHS", crest: 0xe0903a,
        desc: "THE FORGE GUILD - THEIR HAMMERS RANG BEFORE THE BELLS DID.",
        bundles: &[
            Bundle {
                id: "coldforge", name: "THE COLD FORGE",
                reqs: &[
                    Req { label: "COPPER ORE", n: 10, matches: ReqMatch::Ids(&["copper"]) },
                    Req { label: "STONE", n: 14, matches: ReqMatch::Ids(&["stone"]) },
                ],
                reward: ("potion", 2),
            },
            Bundle {
                id: "ringinganvil", name: "THE RINGING ANVIL",
                reqs: &[
                    Req { label: "IRON ORE", n: 10, matches: ReqMatch::Ids(&["iron"]) },
                    Req { label: "STONE", n: 10, matches: ReqMatch::Ids(&["stone"]) },
                    Req { label: "GEMS", n: 2, matches: ReqMatch::Ids(&["gem"]) },
                ],
                reward: ("gem", 2),
            },
            Bundle {
                id: "masterorder", name: "THE MASTERWORK ORDER",
                reqs: &[
                    Req { label: "SILVER ORE", n: 8, matches: ReqMatch::Ids(&["silver"]) },
                    Req { label: "GOLD ORE", n: 5, matches: ReqMatch::Ids(&["gold"]) },
                    Req { label: "GEMS", n: 4, matches: ReqMatch::Ids(&["gem"]) },
                ],
                reward: ("mithril", 2),
            },
        ],
        perk_desc: "THE BLACKSMITH STOCKS FINER GEAR",
        loot_desc: "A MASTERWORK WEAPON",
    },
    Wing {
        id: "scholars", name: "THE SCHOLARS", crest: 0xc878ff,
        desc: "THE LEARNED GUILD - THEY WROTE DOWN EVERYTHING WE FORGOT.",
        bundles: &[
            Bundle {
                id: "openshelves", name: "THE OPEN SHELVES",
                reqs: &[
                    Req { label: "GEMS", n: 6, matches: ReqMatch::Ids(&["gem"]) },
                    Req { label: "MONSTER LEATHER", n: 8, matches: ReqMatch::Ids(&["leather"]) },
                ],
                reward: ("potion", 2),
            },
            Bundle {
                id: "bestiary", name: "THE BESTIARY PAGES",
                reqs: &[
                    Req { label: "MONSTER LEATHER", n: 14, matches: ReqMatch::Ids(&["leather"]) },
                    Req { label: "SPIDER STRING", n: 8, matches: ReqMatch::Ids(&["string"]) },
                    Req { label: "HERBS", n: 8, matches: ReqMatch::Ids(&["herb"]) },
                ],
                reward: ("greaterpotion", 1),
            },
            Bundle {
                id: "grandarchive", name: "THE GRAND ARCHIVE",
                reqs: &[
                    Req { label: "GEMS", n: 10, matches: ReqMatch::Ids(&["gem"]) },
                    Req { label: "SPIDER STRING", n: 12, matches: ReqMatch::Ids(&["string"]) },
                    Req { label: "RARE CATCHES", n: 2, matches: ReqMatch::RareFish },
                ],
                reward: ("gem", 3),
            },
        ],
        perk_desc: "THE LIBRARY SELLS TOMES FOR HALF",
        loot_desc: "A LESSON WORTH A SKILL POINT",
    },
    Wing {
        id: "provisioners", name: "THE PROVISIONERS", crest: 0xffd34d,
        desc: "THE KITCHEN GUILD - NO FESTIVAL FED ITSELF.",
        bundles: &[
            Bundle {
                id: "soupkitchen", name: "THE SOUP KITCHEN",
                reqs: &[
                    Req { label: "COOKED DISHES", n: 4, matches: ReqMatch::Dish },
                    Req { label: "MEAT", n: 8, matches: ReqMatch::Ids(&["meat"]) },
                    Req { label: "HERBS", n: 8, matches: ReqMatch::Ids(&["herb"]) },
                ],
                reward: ("potion", 2),
            },
            Bundle {
                id: "longtable", name: "THE LONG TABLE",
                reqs: &[
                    Req { label: "COOKED DISHES", n: 6, matches: ReqMatch::Dish },
                    Req { label: "MEAT", n: 14, matches: ReqMatch::Ids(&["meat"]) },
                    Req { label: "FRESH EGGS", n: 6, matches: ReqMatch::Ids(&["egg"]) },
                ],
                reward: ("greaterpotion", 2),
            },
            Bundle {
                id: "festivallarder", name: "THE FESTIVAL LARDER",
                reqs: &[
                    Req { label: "COOKED DISHES", n: 8, matches: ReqMatch::Dish },
                    Req { label: "HERBS", n: 12, matches: ReqMatch::Ids(&["herb"]) },
                    Req { label: "PAILS OF MILK", n: 6, matches: ReqMatch::Ids(&["milk"]) },
                ],
                reward: ("gem", 2),
            },
        ],
        perk_desc: "THE INN RESTS YOU FREE IN THIS CITY",
        loot_desc: "A FEAST FOR THE ROAD",
    },
];

pub fn wing(id: &str) -> Option<&'static Wing> {
    WINGS.iter().find(|w| w.id == id)
}

/// Does a bag item satisfy a line? (js req.match(def, id)).
pub fn req_matches(m: ReqMatch, id: &str) -> bool {
    let Some(def) = crate::items::get(id) else { return false };
    match m {
        ReqMatch::Kind(k) => def.kind == k,
        ReqMatch::Ids(ids) => ids.contains(&id),
        ReqMatch::RareFish => def.kind == "FISH" && !matches!(def.rarity, crate::items::Rarity::Common | crate::items::Rarity::Uncommon),
        ReqMatch::Dish => def.dish,
    }
}

/// How much of one bundle is donated. done = every line filled.
pub fn bundle_progress(b: &Bundle, counts: &[i32]) -> (i32, i32, bool) {
    let (mut have, mut need) = (0, 0);
    for (i, r) in b.reqs.iter().enumerate() {
        need += r.n;
        have += counts.get(i).copied().unwrap_or(0).min(r.n);
    }
    (have, need, have >= need)
}

/// Is this wing's guild home? Every bundle done — or the wing's own id in `done`,
/// which older saves (and the shot harness) recorded before bundles existed.
pub fn wing_home(done: &[String], w: &Wing) -> bool {
    done.iter().any(|d| d == w.id) || w.bundles.iter().all(|b| done.iter().any(|d| d == b.id))
}

/// Sugar over [`wing_home`] for the perk sites that key by id.
pub fn home_by_id(done: &[String], id: &str) -> bool {
    wing(id).is_some_and(|w| wing_home(done, w))
}

/// How many of the five guilds are home (drives the exterior stage + capstone).
pub fn wings_home(done: &[String]) -> usize {
    WINGS.iter().filter(|w| wing_home(done, w)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wings_are_sound() {
        // Wing ids AND bundle ids share the save's `done` list — all must be unique.
        let mut ids: Vec<&str> = WINGS.iter().map(|w| w.id).collect();
        ids.extend(WINGS.iter().flat_map(|w| w.bundles.iter().map(|b| b.id)));
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate wing/bundle id");
        for w in &WINGS {
            assert!(!w.bundles.is_empty(), "{}: a wing with no bundles", w.id);
            for b in w.bundles {
                // A typo'd item id would make a line unfillable (or a reward vanish) forever.
                assert!(crate::items::get(b.reward.0).is_some(), "{}: unknown reward {}", b.id, b.reward.0);
                for r in b.reqs {
                    if let ReqMatch::Ids(ids) = r.matches {
                        for id in ids {
                            assert!(crate::items::get(id).is_some(), "{}: unknown item {id}", b.id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn legacy_wing_ids_still_read_as_home() {
        let done = vec!["tillers".to_string()];
        assert!(home_by_id(&done, "tillers"), "pre-bundle saves keep their wings");
        assert!(!home_by_id(&done, "smiths"));
        assert_eq!(wings_home(&done), 1);
    }
}
