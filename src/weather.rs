//! weather.rs — the deterministic weather SIMULATION (js/weather.js fronts, verbatim):
//! each third-of-a-day PERIOD rolls a season-weighted FRONT off the world seed, each
//! biome's CLIMATE decides how it lands (cold PRECIP = snow, arid = sandstorm...), and
//! thematically-named lands lean on the scales (Stormreach brews storms, Hollowwood
//! breeds fog). Pure data + math — presentation lives in app/weather.rs + the shader.

/// One weather's presentation facts (js DEFS): the sky darkness it adds, the mood tint,
/// which particle pass it runs, and whether it strikes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WeatherDef {
    pub id: &'static str,
    pub label: &'static str,
    pub sky: f32,
    pub tint: Option<[u8; 3]>,
    pub kind: Kind,
    pub heavy: bool,
    pub lightning: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    None,
    Wind,
    Fog,
    Rain,
    Snow,
    Dust,
}

const fn def(id: &'static str, label: &'static str, sky: f32, tint: Option<[u8; 3]>, kind: Kind) -> WeatherDef {
    WeatherDef { id, label, sky, tint, kind, heavy: false, lightning: false }
}

pub static DEFS: &[WeatherDef] = &[
    def("clear", "CLEAR", 0.00, None, Kind::None),
    def("overcast", "OVERCAST", 0.14, Some([64, 66, 74]), Kind::None),
    def("windy", "WINDY", 0.04, Some([72, 76, 82]), Kind::Wind),
    def("fog", "FOG", 0.07, Some([122, 128, 136]), Kind::Fog),
    def("rain", "RAIN", 0.18, Some([40, 50, 72]), Kind::Rain),
    WeatherDef { heavy: true, lightning: true, ..def("thunderstorm", "THUNDERSTORM", 0.28, Some([26, 32, 54]), Kind::Rain) },
    def("snow", "SNOW", 0.04, Some([120, 132, 150]), Kind::Snow),
    WeatherDef { heavy: true, ..def("blizzard", "BLIZZARD", 0.13, Some([140, 150, 172]), Kind::Snow) },
    WeatherDef { heavy: true, ..def("sandstorm", "SANDSTORM", 0.16, Some([150, 120, 70]), Kind::Dust) },
];

pub fn get(id: &str) -> &'static WeatherDef {
    DEFS.iter().find(|d| d.id == id).unwrap_or(&DEFS[0])
}

/// js CLIMATE — how a front lands here ("cave" = no sky, no weather).
pub static CLIMATE: &[(&str, &str)] = &[
    ("grassland", "temperate"),
    ("forest", "temperate"),
    ("petalwood", "temperate"),
    ("honeyglade", "temperate"),
    ("bluebell", "temperate"),
    ("graveyard", "temperate"),
    ("chaos", "temperate"),
    ("stormreach", "temperate"),
    ("galewind", "temperate"),
    ("witherlands", "temperate"),
    ("desert", "arid"),
    ("suncoast", "arid"),
    ("prismwastes", "arid"),
    ("saltwastes", "arid"),
    ("mountains", "cold"),
    ("arctic", "cold"),
    ("swamp", "humid"),
    ("tarmire", "humid"),
    ("mushroom", "humid"),
    ("greenmaw", "humid"),
    ("hollowwood", "humid"),
    ("burnt", "volcanic"),
    ("embermaw", "volcanic"),
    ("emberscar", "volcanic"),
    ("blackdeep", "cave"),
    ("starhollow", "cave"),
    ("wriftscar", "temperate"),
    ("gloammoor", "humid"),
];

/// js BIAS — (biome, front, weight multiplier). Iteration order matters for parity? No:
/// biases multiply independently.
pub static BIAS: &[(&str, &[(&str, f64)])] = &[
    ("stormreach", &[("STORM", 3.5), ("CLEAR", 0.5)]),
    ("galewind", &[("WINDY", 4.0)]),
    ("chaos", &[("STORM", 2.5)]),
    ("hollowwood", &[("FOG", 3.5)]),
    ("swamp", &[("FOG", 2.5)]),
    ("tarmire", &[("FOG", 2.5)]),
    ("witherlands", &[("FOG", 2.0)]),
    ("suncoast", &[("CLEAR", 2.5), ("FOG", 0.4)]),
    ("honeyglade", &[("CLEAR", 2.0)]),
    ("bluebell", &[("CLEAR", 1.6)]),
    ("desert", &[("CLEAR", 1.8), ("FOG", 0.3)]),
    ("arctic", &[("PRECIP", 1.8)]),
    ("wriftscar", &[("STORM", 3.0), ("CLEAR", 0.5)]),
    ("gloammoor", &[("FOG", 3.0)]),
];

/// js SEASON_FRONTS — front weights in JS OBJECT-KEY ORDER (the roll walks them in
/// order, so this sequence is parity-load-bearing).
const FRONTS: [&str; 6] = ["CLEAR", "CLOUDY", "PRECIP", "STORM", "FOG", "WINDY"];
pub static SEASON_FRONTS: &[(&str, [f64; 6])] = &[
    ("SPRING", [7.0, 2.0, 3.0, 1.0, 1.5, 1.5]),
    ("SUMMER", [9.0, 1.5, 1.5, 2.2, 0.5, 1.0]),
    ("FALL", [6.0, 2.5, 2.0, 1.0, 2.5, 2.5]),
    ("WINTER", [6.0, 2.5, 3.0, 1.2, 1.5, 1.5]),
];

fn climate(biome: &str) -> &'static str {
    CLIMATE.iter().find(|(b, _)| *b == biome).map(|(_, c)| *c).unwrap_or("temperate")
}

fn bias_for(biome: &str, front: &str) -> f64 {
    BIAS.iter()
        .find(|(b, _)| *b == biome)
        .and_then(|(_, rows)| rows.iter().find(|(f, _)| *f == front))
        .map(|(_, w)| *w)
        .unwrap_or(1.0)
}

/// js hash — the period roll's uint32 mixer.
fn hash(mut a: u32) -> u32 {
    a = (a ^ 61) ^ (a >> 16);
    a = a.wrapping_add(a << 3);
    a ^= a >> 4;
    a = a.wrapping_mul(0x27d4eb2d);
    a ^= a >> 15;
    a
}

/// js manifest — a front, reinterpreted by the local climate.
fn manifest(front: &str, climate: &str) -> &'static str {
    match front {
        "CLEAR" => "clear",
        "CLOUDY" => "overcast",
        "WINDY" => "windy",
        "FOG" => "fog",
        "PRECIP" => match climate {
            "cold" => "snow",
            "arid" => "sandstorm",
            "volcanic" => "overcast",
            _ => "rain",
        },
        "STORM" => match climate {
            "cold" => "blizzard",
            "arid" => "sandstorm",
            _ => "thunderstorm",
        },
        _ => "clear",
    }
}

/// js rollFront — the period's front, stable across reloads.
fn roll_front(season: &str, period: i64, world_seed: u32, biome: &str) -> &'static str {
    let base = SEASON_FRONTS
        .iter()
        .find(|(s, _)| *s == season)
        .map(|(_, w)| *w)
        .unwrap_or(SEASON_FRONTS[0].1);
    let w: Vec<f64> = FRONTS.iter().zip(base).map(|(f, b)| b * bias_for(biome, f)).collect();
    let total: f64 = w.iter().sum();
    let h = hash(world_seed ^ ((period as i32 + 1) as u32).wrapping_mul(2654435761));
    let mut r = (h as f64 / 4294967296.0) * total;
    for (f, wt) in FRONTS.iter().zip(&w) {
        r -= wt;
        if r <= 0.0 {
            return f;
        }
    }
    "CLEAR"
}

/// js weatherFor — the weather standing over a biome this period.
/// js precipFor — what falls from a PRECIP front in this biome's climate (Stormcall).
pub fn precip_for(biome: &str) -> &'static str {
    let climate = CLIMATE.iter().find(|(b, _)| *b == biome).map_or("temperate", |(_, c)| *c);
    manifest("PRECIP", climate)
}

pub fn weather_for(biome: &str, season: &str, period: i64, world_seed: u32) -> &'static str {
    let cl = climate(biome);
    if cl == "cave" {
        return "clear"; // underground: no sky, no weather
    }
    manifest(roll_front(season, period, world_seed, biome), cl)
}
