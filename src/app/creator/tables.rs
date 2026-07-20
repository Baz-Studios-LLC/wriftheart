//! tables.rs — the creator's pick tables + starter-name pools, verbatim from
//! js/creator.js (data only; the interaction lives in mod.rs).

// --- The js pick tables, verbatim -----------------------------------------------------

pub(super) const OUTFITS: [(&str, u32, u32); 10] = [
    ("BLUE", 0x2f6fe0, 0x163f9c),
    ("RED", 0xe23a2a, 0x8c1810),
    ("GREEN", 0x3cba4a, 0x1c6e28),
    ("PURPLE", 0xa64fe0, 0x5e2496),
    ("TEAL", 0x2fc0b0, 0x147068),
    ("ORANGE", 0xfc8a30, 0xa8480f),
    ("PINK", 0xfc7ac0, 0xb83c84),
    ("GOLD", 0xf0c030, 0x9a7410),
    ("SLATE", 0x8890a0, 0x4a5260),
    ("WHITE", 0xe8e8f0, 0x9a9aa8),
];
pub(super) const HAIRS: [(&str, u32, u32); 8] = [
    ("BROWN", 0x8a5a2a, 0x5a3a18),
    ("BLACK", 0x4a4450, 0x26222c),
    ("BLONDE", 0xf0d070, 0xb89030),
    ("AUBURN", 0xc85a2a, 0x8c3416),
    ("SILVER", 0xe0e0e8, 0xa0a0ac),
    ("BLUE", 0x4a90d0, 0x2c5e96),
    ("PINK", 0xf08ac0, 0xb85088),
    ("GREEN", 0x5ab04a, 0x327028),
];
pub(super) const SKINS: [(&str, u32); 7] = [
    ("LIGHT", 0xfcd0a0),
    ("FAIR", 0xf0b890),
    ("TAN", 0xd89860),
    ("BROWN", 0xa06838),
    ("DEEP", 0x6a4428),
    ("OLIVE", 0x9ab070),
    ("ASHEN", 0xb8c0d0),
];
pub(super) const EYES: [(&str, u32); 7] = [
    ("BROWN", 0x3a2a18),
    ("BLUE", 0x3a6ad0),
    ("GREEN", 0x2a8a4a),
    ("GRAY", 0x6a6a78),
    ("AMBER", 0xc88a2a),
    ("RED", 0xb02020),
    ("VIOLET", 0x8a4ad0),
];
pub(super) const STYLES: [(&str, &str); 8] = [
    ("SHORT", "short"),
    ("BANGS", "bangs"),
    ("PARTED", "parted"),
    ("MOHAWK", "mohawk"),
    ("LONG", "long"),
    ("PONYTAIL", "ponytail"),
    ("SPIKY", "spiky"),
    ("TOPKNOT", "topknot"),
];

pub(super) const N_FIELDS: usize = 9; // name gender hair style eyes skin outfit reroll start
pub(super) const KB: [&[&str]; 4] = [
    &["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"],
    &["K", "L", "M", "N", "O", "P", "Q", "R", "S", "T"],
    &["U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3"],
    &["4", "5", "6", "7", "8", "9", "_", "DEL", "OK"],
];

// Starter names, each tagged M/F/N (js NAMES) — a NEW GAME rolls one and defaults the
// gender to match.
pub(super) const MALE: &[&str] = &[
    "ALARIC", "ALDEN", "AMBROSE", "ARNO", "ASHTON", "BALDR", "BORIS", "BRAM", "BRENNAN", "BROM",
    "CALEB", "CASPER", "CEDRIC", "CONRAD", "DARIUS", "DENHOLM", "DORAN", "DRAVEN", "EAMON", "EDGAR",
    "EDRIC", "ELDON", "ELRIC", "FARLEY", "FELIX", "FENN", "GALEN", "GARETH", "GODWIN", "GUNNAR",
    "HALDOR", "HARROW", "HENDRIK", "HOLT", "IVOR", "JASPER", "JORAH", "JULIAN", "KELDAN", "KENRIC",
    "KORR", "LOMAR", "LUCIAN", "MAGNUS", "MARCUS", "MORDEN", "NESTOR", "NIALL", "ORRIN", "OSRIC",
    "OSWALD", "PERRIN", "PIERS", "QUILLON", "RODRIC", "ROLAND", "RURIK", "SILAS", "SORIN", "THANE",
    "TOBIAS", "TORIN", "ULRIC", "VANCE", "VICTOR", "WULF", "YORICK", "ZARIN", "ZEKE",
];
pub(super) const FEMALE: &[&str] = &[
    "ADELE", "ALYS", "ASHA", "AVANI", "BRIAR", "BRYN", "CARA", "CLOVE", "CORINNE", "DAHLIA",
    "DELIA", "EDA", "ELARA", "ELSA", "ELYRA", "ESME", "FREYA", "GRETA", "GWEN", "HAZEL",
    "IDA", "INGRID", "IRIS", "ISOLDE", "JUNE", "KIRA", "LARK", "LENORE", "LIRA", "LYRA",
    "MAEVE", "MARA", "MERA", "MINA", "MIRA", "NESSA", "NORA", "NOVA", "NYX", "ODESSA",
    "ORLA", "PRIYA", "RHEA", "ROSALIE", "ROWENA", "SABLE", "SADIE", "SELENE", "SERA", "SIBYL",
    "SORA", "TAMSIN", "TESSA", "THEA", "THORA", "VERA", "VESPER", "WILLA", "YARA", "YVAINE",
    "ZARA", "ZINNIA",
];
pub(super) const NEUTRAL: &[&str] = &[
    "ASH", "AVERY", "BLAIR", "CASS", "DALE", "ELLIS", "EMERY", "FINLEY", "GRAY", "HARLOW",
    "JADEN", "KAI", "LANE", "MARLOW", "MORGAN", "OAKLEY", "PAYTON", "QUINN", "REESE", "REN",
    "RILEY", "ROBIN", "ROWAN", "SAGE", "SAWYER", "SHAY", "SKYLAR", "TATUM", "WREN", "ZEPHYR",
];

