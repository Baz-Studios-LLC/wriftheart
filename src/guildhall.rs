//! guildhall.rs — the five guilds and their restoration bundles (js guildhall.js,
//! pure data + helpers). Every CITY keeps a boarded guildhall; inside, each wing
//! wants a themed BUNDLE of donations. Filling it brings that guild home — a
//! city-wide perk and a one-time reward (app/guildhall.rs wires both).

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

pub struct Wing {
    pub id: &'static str,
    pub name: &'static str,
    pub crest: u32,
    pub desc: &'static str,
    pub reqs: &'static [Req],
    pub perk_desc: &'static str,
    pub loot_desc: &'static str,
}

pub static WINGS: [Wing; 5] = [
    Wing {
        id: "tillers", name: "THE TILLERS", crest: 0x7ee08a,
        desc: "THE FARMERS GUILD - THEY MADE THE VALLEYS FEED THE TOWNS.",
        reqs: &[
            Req { label: "ANY CROPS", n: 5, matches: ReqMatch::Kind("CROP") },
            Req { label: "SEED PACKETS", n: 3, matches: ReqMatch::Kind("SEED") },
            Req { label: "FRESH EGGS", n: 2, matches: ReqMatch::Ids(&["egg"]) },
        ],
        perk_desc: "A PRODUCE STALL OPENS IN THE MARKET",
        loot_desc: "A PACKET OF RARE SEEDS",
    },
    Wing {
        id: "anglers", name: "THE ANGLERS", crest: 0x7090d8,
        desc: "THE FISHERS GUILD - EVERY RIVER KNEW THEIR LINES.",
        reqs: &[
            Req { label: "ANY FISH", n: 6, matches: ReqMatch::Kind("FISH") },
            Req { label: "A RARE CATCH", n: 1, matches: ReqMatch::RareFish },
        ],
        perk_desc: "THE MARKET PAYS EXTRA FOR FISH HERE",
        loot_desc: "THE ANGLERS LUCKY HOOK",
    },
    Wing {
        id: "smiths", name: "THE SMITHS", crest: 0xe0903a,
        desc: "THE FORGE GUILD - THEIR HAMMERS RANG BEFORE THE BELLS DID.",
        reqs: &[
            Req { label: "COPPER ORE", n: 5, matches: ReqMatch::Ids(&["copper"]) },
            Req { label: "STONE", n: 6, matches: ReqMatch::Ids(&["stone"]) },
            Req { label: "A GEM", n: 1, matches: ReqMatch::Ids(&["gem"]) },
        ],
        perk_desc: "THE BLACKSMITH STOCKS FINER GEAR",
        loot_desc: "A MASTERWORK WEAPON",
    },
    Wing {
        id: "scholars", name: "THE SCHOLARS", crest: 0xc878ff,
        desc: "THE LEARNED GUILD - THEY WROTE DOWN EVERYTHING WE FORGOT.",
        reqs: &[
            Req { label: "GEMS", n: 4, matches: ReqMatch::Ids(&["gem"]) },
            Req { label: "MONSTER LEATHER", n: 3, matches: ReqMatch::Ids(&["leather"]) },
        ],
        perk_desc: "THE LIBRARY SELLS TOMES FOR HALF",
        loot_desc: "A LESSON WORTH A SKILL POINT",
    },
    Wing {
        id: "provisioners", name: "THE PROVISIONERS", crest: 0xffd34d,
        desc: "THE KITCHEN GUILD - NO FESTIVAL FED ITSELF.",
        reqs: &[
            Req { label: "COOKED DISHES", n: 2, matches: ReqMatch::Dish }, // the kitchen guild wants real cooking (js)
            Req { label: "MEAT", n: 2, matches: ReqMatch::Ids(&["meat"]) },
            Req { label: "HERBS", n: 2, matches: ReqMatch::Ids(&["herb"]) },
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

/// How much of a wing is donated (js wingProgress). done = every line filled.
pub fn wing_progress(w: &Wing, counts: &[i32]) -> (i32, i32, bool) {
    let (mut have, mut need) = (0, 0);
    for (i, r) in w.reqs.iter().enumerate() {
        need += r.n;
        have += counts.get(i).copied().unwrap_or(0).min(r.n);
    }
    (have, need, have >= need)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wings_are_sound() {
        let mut ids: Vec<_> = WINGS.iter().map(|w| w.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 5, "duplicate wing id");
        // Every fixed-id requirement points at a real item (a typo would make a
        // line unfillable forever).
        for w in &WINGS {
            for r in w.reqs {
                if let ReqMatch::Ids(ids) = r.matches {
                    for id in ids {
                        assert!(crate::items::get(id).is_some(), "{}: unknown item {id}", w.id);
                    }
                }
            }
        }
    }
}
