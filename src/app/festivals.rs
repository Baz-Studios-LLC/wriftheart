//! festivals.rs — the year's four fairs (js festivals.js data + the game.js hooks).
//! One per 28-day season, all on DAY 12 (marked on the codex calendar): towns hang
//! BUNTING, the first town you enter that day greets you (the Seed Fair gifts
//! seeds), Cast Day doubles what fish fetch and the Harvest Fair doubles crops
//! (shop sell hooks), and on BELLNIGHT standing in any town at dusk rings the
//! blessing over you (the status system's `blessing` — +luck, slow mending) until
//! the day turns. Attendance and the blessing are once-per-day, saved.
//! NOT YET (flagged): festival folk with their own lines (extra villagers).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::codex::calendar_tab::{day_of_season, season_index};
use super::gather::DAY_LEN;
use super::play::{CurRoom, SlideActive};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};

pub struct FestDef {
    pub id: &'static str,
    pub season: usize, // index into calendar_tab::SEASONS
    pub day: i64,
    pub name: &'static str,
    pub desc: &'static str,
    pub color: u32,
}

pub static LIST: [FestDef; 4] = [
    FestDef { id: "seedfair", season: 0, day: 12, name: "THE SEED FAIR", desc: "VISIT A TOWN FOR A GIFT OF SEEDS", color: 0x7ee08a },
    FestDef { id: "castday", season: 1, day: 12, name: "THE GREAT CAST", desc: "FISH SELL FOR DOUBLE ALL DAY", color: 0xffd34d },
    FestDef { id: "harvestfair", season: 2, day: 12, name: "THE HARVEST FAIR", desc: "CROPS SELL FOR DOUBLE ALL DAY", color: 0xe0903a },
    FestDef { id: "bellnight", season: 3, day: 12, name: "BELLNIGHT", desc: "STAND IN TOWN AT DUSK FOR THE BLESSING", color: 0xbfe0ff },
];

/// The festival happening RIGHT NOW, if today is its day (js Festivals.onDay).
pub fn today(clock: i64) -> Option<&'static FestDef> {
    LIST.iter().find(|f| f.season == season_index(clock) && f.day == day_of_season(clock))
}

/// js festSellMult: Cast Day doubles fish, the Harvest Fair doubles crops.
pub fn sell_mult(id: &str, clock: i64) -> i32 {
    let Some(f) = today(clock) else { return 1 };
    let Some(d) = crate::items::get(id) else { return 1 };
    match (f.id, d.kind) {
        ("castday", "FISH") | ("harvestfair", "CROP") => 2,
        _ => 1,
    }
}

/// Once-per-day markers (js festivalSeenDay / blessedDay) — ride the save.
#[derive(Resource)]
pub struct FestivalLedger {
    pub seen_day: i64,
    pub blessed_day: i64,
}
impl Default for FestivalLedger {
    fn default() -> Self {
        FestivalLedger { seen_day: -1, blessed_day: -1 }
    }
}

/// The sagging flag-line strung over the square (js bunting, static v1).
const BUNTING: [&str; 14] = [
    "P................................................P",
    "PLLL...........................................LLP",
    "P..LLLLL...................................LLLLL.P",
    "P.FFF..LLLLLLL.....................LLLLLLL..FFF..P",
    "P.FFF.WWW....LLLLLLLLLLLLLLLLLLLLLL....WWW..FFF..P",
    "P..F..WWW..FFF......WWW......FFF...FFF.WWW...F...P",
    "P..F..WWW..FFF.WWW..WWW.FFF..FFF...FFF.WWW...F...P",
    "P......W...FFF.WWW...W..FFF...F....FFF..W........P",
    "P...........F..WWW......FFF...F.....F............P",
    "P...........F...W........F.......................P",
    "P................................................P",
    "P................................................P",
    "P................................................P",
    "P................................................P",
];

#[derive(Component)]
struct Bunting;

/// Town rooms dress for the fair + the first town of the day greets (and gifts).
#[allow(clippy::too_many_arguments)]
fn fest_town_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<FrameClock>,
    cur: Res<CurRoom>,
    sliding: Res<SlideActive>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    world: Res<super::play::GameWorld>,
    mut ledger: ResMut<FestivalLedger>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut rng: ResMut<super::battle::GameRng>,
    mut stats: ResMut<super::stats::Stats>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut dressed: Local<Option<(i32, i32, i64)>>,
    bunting: Query<Entity, With<Bunting>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        return;
    }
    let day = clock.0 / DAY_LEN;
    let Some(f) = today(clock.0) else {
        *dressed = None;
        return;
    };
    if !world.0.is_town(cur.rx, cur.ry) {
        return;
    }
    // Dress the square once per room-entry (RoomActor: the sweep undresses on leave).
    if *dressed != Some((cur.rx, cur.ry, day)) {
        *dressed = Some((cur.rx, cur.ry, day));
        if bunting.is_empty() {
            let img = images.add(crate::gfx::bake(&BUNTING, &[('F', f.color), ('P', 0x6a4a2a), ('L', 0xd8ccb0)]));
            for x in [72.0, 184.0] {
                commands.spawn((
                    Sprite::from_image(img.clone()),
                    at(PLAY_X + x, PLAY_Y + 46.0, 50.0, 14.0, 2.2),
                    PIXEL_LAYER,
                    RoomActor,
                    Bunting,
                ));
            }
        }
        // The first town of the day: the greeting toast (+ the Seed Fair's packet).
        if ledger.seen_day != day {
            ledger.seen_day = day;
            stats.bump("festivals", 1.0);
            log.add("fest", &format!("{} - {}", f.name, f.desc), 1, f.color, false, true);
            if f.id == "seedfair" {
                let pool: Vec<&crate::items::CropDef> =
                    crate::items::CROPS.iter().filter(|c| c.seasons.contains(&"SPRING")).collect();
                for _ in 0..3 {
                    if pool.is_empty() {
                        break;
                    }
                    let c = pool[(rng.0.next_f64() * pool.len() as f64) as usize % pool.len()];
                    if let Some(def) = crate::items::get(&format!("{}seed", c.id)) {
                        inv.add_item(def.id, 3);
                    }
                }
                log.add("fest", "THE TOWN GIFTS YOU SEEDS", 1, 0x7ee08a, false, true);
                sfx.write(super::sfx::Sfx("itemget"));
            } else {
                sfx.write(super::sfx::Sfx("levelup"));
            }
        }
    }
}

/// Bellnight: stand in any town at dusk and the bells bless you (once per Bellnight).
#[allow(clippy::too_many_arguments)]
fn bellnight_tick(
    clock: Res<FrameClock>,
    cur: Res<CurRoom>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    world: Res<super::play::GameWorld>,
    mut ledger: ResMut<FestivalLedger>,
    mut statuses: ResMut<super::status::Statuses>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        return;
    }
    let day = clock.0 / DAY_LEN;
    if ledger.blessed_day == day
        || super::lighting::day_darkness(clock.0) <= 0.35
        || !world.0.is_town(cur.rx, cur.ry)
        || today(clock.0).map(|f| f.id) != Some("bellnight")
    {
        return;
    }
    ledger.blessed_day = day;
    statuses.add("blessing", DAY_LEN as i32); // rings until the day turns
    log.add("fest", "THE BELLS BLESS YOU", 1, 0xbfe0ff, false, true);
    sfx.write(super::sfx::Sfx("bellring"));
}

pub struct FestivalsPlugin;
impl Plugin for FestivalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FestivalLedger>().add_systems(
            bevy::app::FixedUpdate,
            (fest_town_tick, bellnight_tick).before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bunting_is_rectangular_and_seasons_covered() {
        for r in BUNTING {
            assert_eq!(r.chars().count(), 50, "bunting row width");
        }
        let mut seasons: Vec<usize> = LIST.iter().map(|f| f.season).collect();
        seasons.sort_unstable();
        assert_eq!(seasons, vec![0, 1, 2, 3], "one fair per season");
    }
}
