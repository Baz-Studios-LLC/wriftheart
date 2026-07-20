//! settings.rs — persisted player preferences (port of js/settings.js, plus the sound flag
//! from audio.js and the autosave flag from game.js, which the JS kept in separate
//! localStorage keys — ONE file owns them all here, settings.json, custom keybindings
//! included). Toggled from the pause menu; read live by the consuming systems
//! (lighting brightness, the canvas scaler, the autosave heartbeat).

use crate::input::Bindings;
use crate::persist;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Resource, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pixel: bool,  // pixel-perfect (integer) canvas scaling
    pub shake: u8,    // screen shake: 0 OFF / 1 LOW / 2 FULL
    pub bright: u8,   // brightness lift 0..4 (0 = default darkness)
    pub flash: bool,  // REDUCE FLASHING (dampen lightning / white-outs / hit flashes)
    #[serde(default = "default_true")]
    pub sound: bool, // audio master switch (honoured once the synth lands)
    #[serde(default = "default_true")]
    pub autosave: bool, // the ~10s heartbeat + pause checkpoint (explicit SAVE always writes)
    pub fullscreen: bool, // DEVIATION: a real toggle — the js could only print a key hint
    keys: Vec<crate::input::BindRow>, // custom keyboard bindings, (action slug, key labels)
    pads: Vec<crate::input::BindRow>, // custom pad bindings
}

impl Default for Settings {
    fn default() -> Self {
        // js DEF { pixel: false, shake: 2, bright: 0, flash: false } — shake FULL out of the box.
        Self {
            pixel: false,
            shake: 2,
            bright: 0,
            flash: false,
            sound: true,
            autosave: true,
            fullscreen: true, // default to fullscreen on a fresh install (the menu toggle persists a choice)
            keys: vec![],
            pads: vec![],
        }
    }
}

impl Settings {
    /// Scales every screen shake (js shakeMul).
    pub fn shake_mul(&self) -> f32 {
        [0.0, 0.5, 1.0][self.shake.min(2) as usize]
    }
    /// Subtracted from the ambient darkness alpha, 0..0.36 (js brightLift).
    pub fn bright_lift(&self) -> f32 {
        self.bright.min(4) as f32 * 0.09
    }
    // --- menu display labels (js shakeLabel / brightLabel) ---
    pub fn shake_label(&self) -> &'static str {
        ["OFF", "LOW", "FULL"][self.shake.min(2) as usize]
    }
    pub fn bright_label(&self) -> &'static str {
        ["DEFAULT", "+1", "+2", "+3", "+4"][self.bright.min(4) as usize]
    }
}

/// Write settings + the current bindings to disk — call after every menu change (the js
/// wrote localStorage on each set(); the file is tiny).
pub fn store(settings: &mut Settings, bindings: &Bindings) {
    if !persist::enabled() {
        return;
    }
    let Some(path) = persist::data_file("settings.json") else { return };
    (settings.keys, settings.pads) = bindings.export();
    if let Ok(json) = serde_json::to_string_pretty(&*settings) {
        let _ = std::fs::write(path, json);
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bindings>().add_systems(PreStartup, load_settings);
    }
}

/// Load settings.json (defaults on a fresh install or under WRIFT_SHOT) and overlay any
/// saved custom bindings onto the defaults.
fn load_settings(mut commands: Commands, mut bindings: ResMut<Bindings>) {
    let settings = persist::enabled()
        .then(|| persist::data_file("settings.json"))
        .flatten()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default();
    if !settings.keys.is_empty() || !settings.pads.is_empty() {
        bindings.import(&settings.keys, &settings.pads);
    }
    commands.insert_resource(settings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Action;
    use bevy::input::keyboard::KeyCode;

    /// Rebind -> export -> import round trip: the custom key survives, the stolen key is
    /// gone from its old action, unknown rows drop quietly.
    #[test]
    fn bindings_round_trip() {
        let mut b = Bindings::default();
        b.rebind_key(Action::Trash, KeyCode::KeyM); // steal M from Map
        let (keys, pads) = b.export();
        let mut fresh = Bindings::default();
        fresh.import(&keys, &pads);
        assert_eq!(fresh.key_name(Action::Trash), "M");
        assert_eq!(fresh.key_name(Action::Map), "TAB"); // M was stripped; Tab remains
        let bogus = vec![("nosuch".to_string(), vec!["Z".to_string()])];
        fresh.import(&bogus, &[]); // must not panic or change anything
        assert_eq!(fresh.key_name(Action::Trash), "M");
    }

    /// Settings JSON round trip + defaults for missing fields (an old file gains new
    /// fields without corruption).
    #[test]
    fn settings_round_trip() {
        let s: Settings = serde_json::from_str(r#"{"bright":3,"pixel":true}"#).unwrap();
        assert!(s.pixel);
        assert_eq!(s.bright_label(), "+3");
        assert!(s.autosave); // defaulted
        assert!((s.shake_mul() - 1.0).abs() < f32::EPSILON); // shake defaulted to FULL
    }
}
