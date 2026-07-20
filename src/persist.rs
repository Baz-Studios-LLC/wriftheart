//! persist.rs — WHERE saved state lives, and WHETHER it may be touched. The one source of
//! truth for on-disk persistence shared by the save file (app/save.rs) and the settings
//! file (settings.rs). The JS used localStorage keys; here it's JSON files in the platform
//! data dir (~/Library/Application Support/wriftheart on macOS).

/// Screenshot-harness runs must never read or write real player state — shots stay
/// deterministic and can't clobber a real save.
pub fn enabled() -> bool {
    std::env::var("WRIFT_SHOT").is_err()
}

/// The path for a named persistence file, creating the data dir on the way.
pub fn data_file(name: &str) -> Option<std::path::PathBuf> {
    let dir = dirs::data_dir()?.join("wriftheart");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(name))
}
