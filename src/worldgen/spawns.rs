//! spawns.rs — mob difficulty ranks + wild-room checks. The roster itself moved to
//! worldgen/entities.rs (the full getEntities port, prop-aware and byte-parity).

use super::world::World;

/// Each enemy's difficulty rank — the distance tier must reach it before it can spawn.
/// Port of MOB_TIER (js/world.js); unknown kinds rank 0, exactly like `MOB_TIER[k] || 0`.
pub fn mob_tier(kind: &str) -> i32 {
    match kind {
        "wolf" | "spider" | "scorpion" | "burrower" | "frog" | "skeleton" | "hurler"
        | "cinderhound" | "chaoswisp" | "vinesnare" | "mirefly" | "tidecrab" => 1,
        "bear" | "zombie" | "ghoul" | "leech" | "lurker" | "archer" | "redgoblin" | "icetroll"
        | "charbrute" | "myconid" | "voidling" | "sandmaw" | "prismshard" | "boglight" => 2,
        "golem" | "revenant" | "frostwyrm" | "pyrewraith" | "sporemother" | "riftlord"
        | "stormcaller" | "deepcrawler" | "gravewarden" | "palehowler" => 3,
        "saltstatue" | "emberling" | "ashgeyser" => 4,
        "witherheart" | "switchshade" => 5,
        _ => 0,
    }
}


impl World {
    /// A plain, un-authored overworld room — port of `isWild`.
    pub fn is_wild_room(&self, ax: i32, ay: i32) -> bool {
        !self.is_town(ax, ay)
            && !World::is_castle(ax, ay)
            && self.shard_dungeon_at(ax, ay).is_none()
            && !self.saltmaze_at(ax, ay)
            && !self.rift_at(ax, ay)
            && (ax.abs() > 1 || ay.abs() > 1)
    }
}
