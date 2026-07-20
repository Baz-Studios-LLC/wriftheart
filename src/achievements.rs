//! achievements.rs — the 79 awards of the HALL OF DEEDS (port of js/achievements.js,
//! verbatim): each row measures a running stat snapshot ([`AchStats`]) against a goal.
//! The app side (app/awards.rs) builds snapshots, unlocks (once earned, stays earned —
//! saved), toasts, and renders the codex AWARDS tab.
//!
//! Entries carry a `cat` — the panel groups by category IN LIST ORDER, so keep each
//! category's entries together. Adding an award = one row here (plus, if it measures
//! something new, a field in the snapshot). Roughly half are HIDDEN — '? ? ?' until
//! earned — so the tab reads as surprises, not a spoiler list.

/// The stat snapshot an award reads (js achStats()). Fields the rust port can't
/// measure yet stay 0 — their awards simply wait for their systems.
#[derive(Default)]
pub struct AchStats {
    pub kills: f64,
    pub mobs: f64,
    pub mobs_total: f64,
    pub elites: f64,
    pub bosses: f64,
    pub gate: f64,
    pub dmg: f64,
    pub deaths: f64,
    pub level: f64,
    pub rooms: f64,
    pub walk: f64,
    pub towns: f64,
    pub dungeons: f64,
    pub encounters: f64,
    pub warps: f64,
    pub relics: f64,
    pub won: f64,
    pub kingsplitter: f64,
    pub rift_best: f64,
    pub riftfloors: f64,
    pub home: f64,
    pub sleeps: f64,
    pub crops: f64,
    pub eggs: f64,
    pub milk: f64,
    pub pets: f64,
    pub animals: f64,
    pub fish: f64,
    pub bigfish: f64,
    pub junk: f64,
    pub crafts: f64,
    pub blueprints: f64,
    pub tables: f64,
    pub trees: f64,
    pub rocks: f64,
    pub items: f64,
    pub items_total: f64,
    pub money: f64,
    pub coins_lifetime: f64,
    pub met: f64,
    pub hellos: f64,
    pub best_hearts: f64,
    pub gifts: f64,
    pub quests: f64,
    pub festivals: f64,
    pub wings: f64,
    pub full_halls: f64,
    pub songs: f64,
    pub songs_total: f64,
    pub songstones: f64,
    pub books: f64,
    pub books_total: f64,
    pub digs: f64,
    pub chests: f64,
}

pub struct AwardDef {
    pub cat: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    /// '? ? ?' until earned (the js HIDDEN set, folded in).
    pub hidden: bool,
    pub cur: fn(&AchStats) -> f64,
    pub goal: fn(&AchStats) -> f64,
}

/// The category banner colours (js ACH_CATCOL).
pub const CATCOL: &[(&str, u32)] = &[
    ("DEEDS OF WAR", 0xe08a6a),
    ("THE WIDE WORLD", 0x8ac97e),
    ("THE RIFT", 0xb48ae8),
    ("HEARTH AND HARVEST", 0xe8c860),
    ("THE MAKERS TRADE", 0x7ab8d8),
    ("THE FOLK", 0xe89ab0),
    ("SONG AND STORY", 0xcfc9a8),
];

pub fn cat_color(cat: &str) -> u32 {
    CATCOL.iter().find(|(c, _)| *c == cat).map_or(0xe8c860, |(_, col)| *col)
}

const fn a(
    cat: &'static str,
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    cur: fn(&AchStats) -> f64,
    goal: fn(&AchStats) -> f64,
) -> AwardDef {
    AwardDef { cat, id, name, desc, hidden: false, cur, goal }
}

const fn h(
    cat: &'static str,
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    cur: fn(&AchStats) -> f64,
    goal: fn(&AchStats) -> f64,
) -> AwardDef {
    AwardDef { cat, id, name, desc, hidden: true, cur, goal }
}

/// The whole hall, js order (79 awards across 7 categories; `h` rows are the HIDDEN set).
pub static LIST: &[AwardDef] = &[
    // ------------------------------------------------- the fighting life
    a("DEEDS OF WAR", "firstblood", "First Blood", "Slay your first foe.", |s| s.kills, |_| 1.0),
    a("DEEDS OF WAR", "slayer", "Slayer", "Fell 100 foes.", |s| s.kills, |_| 100.0),
    h("DEEDS OF WAR", "reaper", "Reaper", "Fell 1,000 foes.", |s| s.kills, |_| 1000.0),
    h("DEEDS OF WAR", "scourge", "Scourge of the Wrift", "Fell 5,000 foes.", |s| s.kills, |_| 5000.0),
    a("DEEDS OF WAR", "hunter", "Hunter", "Discover 10 kinds of foe.", |s| s.mobs, |_| 10.0),
    a("DEEDS OF WAR", "exterminator", "Exterminator", "Discover 25 kinds of foe.", |s| s.mobs, |_| 25.0),
    h("DEEDS OF WAR", "naturalist", "Naturalist", "Fill the bestiary.", |s| s.mobs, |s| s.mobs_total),
    a("DEEDS OF WAR", "championsend", "A Champions End", "Put down 25 champions or elites.", |s| s.elites, |_| 25.0),
    h("DEEDS OF WAR", "giantfall", "Giantfall", "Fell 10 bosses.", |s| s.bosses, |_| 10.0),
    h("DEEDS OF WAR", "crownbreaker", "Crownbreaker", "Fell 25 bosses.", |s| s.bosses, |_| 25.0),
    h("DEEDS OF WAR", "gatecrasher", "Gatecrasher", "Break the Black Castle gate guard.", |s| s.gate, |_| 1.0),
    a("DEEDS OF WAR", "punchingbag", "Punching Bag", "Lose 1,000 HP, lifetime.", |s| s.dmg, |_| 1000.0),
    h("DEEDS OF WAR", "thehardway", "The Hard Way", "Watch it all go dark 10 times.", |s| s.deaths, |_| 10.0),
    // ------------------------------------------------- the road and the map
    a("THE WIDE WORLD", "veteran", "Veteran", "Reach level 10.", |s| s.level, |_| 10.0),
    a("THE WIDE WORLD", "hero", "Hero", "Reach level 25.", |s| s.level, |_| 25.0),
    h("THE WIDE WORLD", "legend", "Legend", "Reach level 50.", |s| s.level, |_| 50.0),
    a("THE WIDE WORLD", "explorer", "Explorer", "Explore 50 rooms.", |s| s.rooms, |_| 50.0),
    a("THE WIDE WORLD", "cartographer", "Cartographer", "Explore 250 rooms.", |s| s.rooms, |_| 250.0),
    h("THE WIDE WORLD", "pathfinder", "Pathfinder", "Explore 1,000 rooms.", |s| s.rooms, |_| 1000.0),
    a("THE WIDE WORLD", "worldwalker", "World-Walker", "Walk 10,000 tiles.", |s| s.walk, |_| 10000.0),
    a("THE WIDE WORLD", "wanderer", "Wanderer", "Discover 5 towns.", |s| s.towns, |_| 5.0),
    h("THE WIDE WORLD", "metropolitan", "Metropolitan", "Discover 10 towns.", |s| s.towns, |_| 10.0),
    a("THE WIDE WORLD", "delver", "Delver", "Discover a dungeon.", |s| s.dungeons, |_| 1.0),
    h("THE WIDE WORLD", "spelunker", "Spelunker", "Discover 5 dungeons.", |s| s.dungeons, |_| 5.0),
    a("THE WIDE WORLD", "battletested", "Battle-Tested", "Clear 5 encounters.", |s| s.encounters, |_| 5.0),
    h("THE WIDE WORLD", "warlord", "Warlord", "Clear 20 encounters.", |s| s.encounters, |_| 20.0),
    h("THE WIDE WORLD", "farfromhome", "Far From Home", "Ride 25 warps.", |s| s.warps, |_| 25.0),
    // ------------------------------------------------- the tear in the world
    h("THE RIFT", "shardseeker", "Shard Seeker", "Recover a Wriftheart shard.", |s| s.relics, |_| 1.0),
    h("THE RIFT", "riftbound", "Riftbound", "Recover all ten shards.", |s| s.relics, |_| 10.0),
    h("THE RIFT", "worldmender", "World-Mender", "Mend the Wriftheart.", |s| s.won, |_| 1.0),
    h("THE RIFT", "kingsplitter", "The Kingsplitter", "Claim the sword that split a king.", |s| s.kingsplitter, |_| 1.0),
    h("THE RIFT", "riftwalker", "Riftwalker", "Reach floor 5 of a Rift Spire.", |s| s.rift_best, |_| 5.0),
    h("THE RIFT", "deepdelver", "Deepdelver", "Reach floor 10 of a Rift Spire.", |s| s.rift_best, |_| 10.0),
    h("THE RIFT", "thelongfall", "The Long Fall", "Reach floor 20 of a Rift Spire.", |s| s.rift_best, |_| 20.0),
    h("THE RIFT", "downanddown", "Down and Down", "Take 25 rift gates, lifetime.", |s| s.riftfloors, |_| 25.0),
    // ------------------------------------------------- home, field, and water
    a("HEARTH AND HARVEST", "homeowner", "Homeowner", "Build a home of your own.", |s| s.home, |_| 1.0),
    a("HEARTH AND HARVEST", "wellrested", "Well Rested", "Sleep 10 nights.", |s| s.sleeps, |_| 10.0),
    a("HEARTH AND HARVEST", "greenthumb", "Green Thumb", "Harvest 10 crops.", |s| s.crops, |_| 10.0),
    h("HEARTH AND HARVEST", "harvesthome", "Harvest Home", "Harvest 100 crops.", |s| s.crops, |_| 100.0),
    a("HEARTH AND HARVEST", "henfriend", "Hen Friend", "Gather 10 eggs.", |s| s.eggs, |_| 10.0),
    h("HEARTH AND HARVEST", "eggsfordays", "Eggs For Days", "Gather 100 eggs.", |s| s.eggs, |_| 100.0),
    a("HEARTH AND HARVEST", "dairyfarmer", "Dairy Farmer", "Fill 10 pails of milk.", |s| s.milk, |_| 10.0),
    a("HEARTH AND HARVEST", "gentlesoul", "Gentle Soul", "Pet your animals 25 times.", |s| s.pets, |_| 25.0),
    h("HEARTH AND HARVEST", "menagerie", "Menagerie", "Keep 5 animals at once.", |s| s.animals, |_| 5.0),
    a("HEARTH AND HARVEST", "angler", "Angler", "Land 10 fish.", |s| s.fish, |_| 10.0),
    h("HEARTH AND HARVEST", "masterangler", "Master Angler", "Land 100 fish.", |s| s.fish, |_| 100.0),
    h("HEARTH AND HARVEST", "braggingrights", "Bragging Rights", "Land a fish of 10 lb or more.", |s| s.bigfish, |_| 10.0),
    a("HEARTH AND HARVEST", "bootfisher", "Boot Fisher", "Reel in 10 pieces of junk.", |s| s.junk, |_| 10.0),
    // ------------------------------------------------- the makers trade
    a("THE MAKERS TRADE", "maker", "Maker", "Craft 10 things.", |s| s.crafts, |_| 10.0),
    h("THE MAKERS TRADE", "artisan", "Artisan", "Craft 100 things.", |s| s.crafts, |_| 100.0),
    a("THE MAKERS TRADE", "draughtsman", "Draughtsman", "Learn 10 blueprints.", |s| s.blueprints, |_| 10.0),
    a("THE MAKERS TRADE", "homesteader", "Homesteader", "Place 5 crafting stations.", |s| s.tables, |_| 5.0),
    a("THE MAKERS TRADE", "timber", "Timber!", "Fell 100 trees.", |s| s.trees, |_| 100.0),
    h("THE MAKERS TRADE", "stonebreaker", "Stonebreaker", "Break 100 stones and veins.", |s| s.rocks, |_| 100.0),
    a("THE MAKERS TRADE", "scavenger", "Scavenger", "Discover 25 items.", |s| s.items, |_| 25.0),
    a("THE MAKERS TRADE", "collector", "Collector", "Discover 50 items.", |s| s.items, |_| 50.0),
    h("THE MAKERS TRADE", "completionist", "Completionist", "Discover every item.", |s| s.items, |s| s.items_total),
    a("THE MAKERS TRADE", "wealthy", "Wealthy", "Amass 1,000 coin.", |s| s.money, |_| 1000.0),
    h("THE MAKERS TRADE", "tycoon", "Tycoon", "Amass 10,000 coin.", |s| s.money, |_| 10000.0),
    h("THE MAKERS TRADE", "coincounter", "Coin Counter", "Pick 25,000 coin off the ground.", |s| s.coins_lifetime, |_| 25000.0),
    // ------------------------------------------------- neighbors and guilds
    a("THE FOLK", "wellmet", "Well Met", "Learn 10 folks names.", |s| s.met, |_| 10.0),
    a("THE FOLK", "familiarface", "A Familiar Face", "Say 100 hellos.", |s| s.hellos, |_| 100.0),
    a("THE FOLK", "confidant", "Confidant", "Win 7 hearts with one soul.", |s| s.best_hearts, |_| 7.0),
    a("THE FOLK", "generous", "Generous", "Give 25 gifts.", |s| s.gifts, |_| 25.0),
    a("THE FOLK", "adventurer", "Adventurer", "Complete 5 quests.", |s| s.quests, |_| 5.0),
    h("THE FOLK", "renowned", "Renowned", "Complete 20 quests.", |s| s.quests, |_| 20.0),
    a("THE FOLK", "fairgoer", "Fairgoer", "Attend 4 festivals.", |s| s.festivals, |_| 4.0),
    a("THE FOLK", "guildfriend", "Guildfriend", "Restore a guildhall wing.", |s| s.wings, |_| 1.0),
    h("THE FOLK", "citybuilder", "City Builder", "Restore 5 guildhall wings.", |s| s.wings, |_| 5.0),
    h("THE FOLK", "hallrestored", "The Hall Restored", "Fully restore a citys guildhall.", |s| s.full_halls, |_| 1.0),
    // ------------------------------------------------- song and story
    a("SONG AND STORY", "atunefortheroad", "A Tune For The Road", "Learn your first flute song.", |s| s.songs, |_| 1.0),
    h("SONG AND STORY", "songmaster", "Songmaster", "Learn every flute song.", |s| s.songs, |s| s.songs_total),
    h("SONG AND STORY", "stonesinger", "Stonesinger", "Sing open a singing stone.", |s| s.songstones, |_| 1.0),
    h("SONG AND STORY", "olddoors", "Old Doors", "Sing open 5 singing stones.", |s| s.songstones, |_| 5.0),
    a("SONG AND STORY", "reader", "Reader", "Recover 10 lost tomes.", |s| s.books, |_| 10.0),
    h("SONG AND STORY", "scholar", "Scholar", "Recover 50 lost tomes.", |s| s.books, |_| 50.0),
    h("SONG AND STORY", "loremaster", "Loremaster", "Recover every lost tome.", |s| s.books, |s| s.books_total),
    a("SONG AND STORY", "groundskeeper", "Groundskeeper", "Dig 25 holes.", |s| s.digs, |_| 25.0),
    a("SONG AND STORY", "deeppockets", "Deep Pockets", "Crack open 25 chests.", |s| s.chests, |_| 25.0),
];

pub fn get(id: &str) -> Option<&'static AwardDef> {
    LIST.iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hall_is_complete_and_unique() {
        assert_eq!(LIST.len(), 79, "the js hall holds 79 deeds");
        for (i, a) in LIST.iter().enumerate() {
            for b in &LIST[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate award id {}", a.id);
            }
        }
        // Categories stay contiguous (the panel groups in list order).
        let mut seen: Vec<&str> = Vec::new();
        for a in LIST {
            match seen.last() {
                Some(&last) if last == a.cat => {}
                _ => {
                    assert!(!seen.contains(&a.cat), "category {} split apart", a.cat);
                    seen.push(a.cat);
                }
            }
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn goals_resolve() {
        let mut s = AchStats { kills: 1.0, ..Default::default() };
        assert!((get("firstblood").unwrap().cur)(&s) >= (get("firstblood").unwrap().goal)(&s));
        s.mobs_total = 41.0;
        assert_eq!((get("naturalist").unwrap().goal)(&s), 41.0);
    }
}
