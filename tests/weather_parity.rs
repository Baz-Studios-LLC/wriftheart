//! weather_parity.rs — pins src/weather's front rolls bit-exact against the live js
//! (every biome x season x period across two seeds).

use wriftheart::weather::weather_for;

include!("data/weather_golden.rs");

#[test]
fn fronts_match_js() {
    const BIOMES: [&str; 20] = [
        "grassland", "forest", "desert", "mountains", "arctic", "swamp", "graveyard", "burnt", "mushroom", "chaos",
        "stormreach", "galewind", "hollowwood", "suncoast", "saltwastes", "embermaw", "blackdeep", "wriftscar",
        "gloammoor", "tarmire",
    ];
    let mut i = 0;
    for seed in [1337u32, 42] {
        for season in ["SPRING", "SUMMER", "FALL", "WINTER"] {
            for period in 0..9i64 {
                let row: Vec<&str> = BIOMES.iter().map(|b| weather_for(b, season, period, seed)).collect();
                let ours = format!("{seed} {season} p{period} {}", row.join(","));
                assert_eq!(ours, WEATHER_GOLDEN[i], "line {i}");
                i += 1;
            }
        }
    }
    assert_eq!(i, WEATHER_GOLDEN.len());
}
