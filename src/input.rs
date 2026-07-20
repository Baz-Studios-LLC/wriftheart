//! input.rs — semantic actions, keyboard + GAMEPAD bindings, and DERIVED prompts.
//!
//! THE RULE (see PORT.md): gameplay code never names a key or button. It reads
//! [`ActionState`] (the JS `Input.held`/`Input.pressed` pair, polled once per render frame
//! and CONSUMED once per fixed tick — the port of poll()/endFrame()); the [`Bindings`]
//! tables map actions to physical keys/pad buttons; and every on-screen "press X" prompt
//! renders through [`Bindings::prompt`], which switches to pad labels the moment a
//! controller is present (the JS `padPresent` behaviour). Rebind once — every prompt
//! in the game updates itself.

use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

/// Everything the game can ask the player to do. Gameplay speaks ONLY in these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Slot1, // primary action / confirm
    Slot2, // secondary / cancel
    Slot3,
    Slot4,
    Inventory,
    SkillTree,
    Map,
    TabPrev, // previous tab in tabbed screens (codex, later crafting)
    TabNext,
    Pause,
    Trash, // inventory: destroy an item (js 'trash' — T; tap one, hold the stack)
    Sort,  // inventory: tidy the bag (js 'warpHome' — H, the menu-helper button)
    Interact, // talk / enter / harvest / read (js 'interact' — F; pad D-pad up)
    /// Menu SELECT only (js Input.confirm's ENTER half): Enter/NumpadEnter confirm on
    /// the title/slot screens. Hidden from the CONTROLS tab (its ROWS list is curated).
    MenuConfirm,
    DevPanel, // the backquote console (dev.rs)
    God,      // invulnerability toggle (dev.rs) — G
    // Quick-access openers (unbound by default unless noted): each jumps straight to a
    // codex tab or a slide-out page from free roam. All rebindable in CONTROLS.
    Calendar,
    People,
    Guilds,
    Mobs,
    ItemsDex,
    Songs,
    Awards,
    StatsTab,
    Lore,
    Wriftheart,
    Craft,
    StatusTab,
}
pub const ACTIONS: [Action; 32] = [
    Action::MenuConfirm,
    Action::Up,
    Action::Down,
    Action::Left,
    Action::Right,
    Action::Slot1,
    Action::Slot2,
    Action::Slot3,
    Action::Slot4,
    Action::Inventory,
    Action::SkillTree,
    Action::Map,
    Action::TabPrev,
    Action::TabNext,
    Action::Pause,
    Action::Trash,
    Action::Sort,
    Action::Interact,
    Action::DevPanel,
    Action::God,
    Action::Calendar,
    Action::People,
    Action::Guilds,
    Action::Mobs,
    Action::ItemsDex,
    Action::Songs,
    Action::Awards,
    Action::StatsTab,
    Action::Lore,
    Action::Wriftheart,
    Action::Craft,
    Action::StatusTab,
];

fn idx(a: Action) -> usize {
    ACTIONS.iter().position(|x| *x == a).unwrap()
}

/// Stable serialization slug per action (the js action-name strings — `warpHome` is our
/// Sort). Settings.json stores bindings under these, so the Action enum can reorder freely.
pub fn action_slug(a: Action) -> &'static str {
    use Action as A;
    match a {
        A::Up => "up",
        A::Down => "down",
        A::Left => "left",
        A::Right => "right",
        A::Slot1 => "slot1",
        A::Slot2 => "slot2",
        A::Slot3 => "slot3",
        A::Slot4 => "slot4",
        A::Inventory => "inventory",
        A::SkillTree => "skilltree",
        A::Map => "map",
        A::TabPrev => "tabPrev",
        A::TabNext => "tabNext",
        A::Pause => "pause",
        A::Trash => "trash",
        A::Sort => "warpHome",
        A::Interact => "interact",
        A::MenuConfirm => "menuConfirm",
        A::DevPanel => "dev",
        A::God => "god",
        A::Calendar => "calendar",
        A::People => "people",
        A::Guilds => "guilds",
        A::Mobs => "mobs",
        A::ItemsDex => "itemsdex",
        A::Songs => "songs",
        A::Awards => "awards",
        A::StatsTab => "statstab",
        A::Lore => "lore",
        A::Wriftheart => "wriftheart",
        A::Craft => "craft",
        A::StatusTab => "statustab",
    }
}

pub fn action_from_slug(s: &str) -> Option<Action> {
    ACTIONS.into_iter().find(|a| action_slug(*a) == s)
}

/// One serialized binding row for settings.json: (action slug, bound input labels).
pub type BindRow = (String, Vec<String>);

/// Action -> physical inputs. Key defaults port DEF_KEYS, pad defaults port DEF_PAD
/// (js/input.js): face buttons = the 4 slots, LB = map/codex, RB = inventory, Start = pause.
#[derive(Resource)]
pub struct Bindings {
    keys: Vec<(Action, Vec<KeyCode>)>,
    pads: Vec<(Action, Vec<GamepadButton>)>,
}

impl Default for Bindings {
    fn default() -> Self {
        use Action as A;
        use GamepadButton as G;
        use KeyCode::*;
        Self {
            // (Action:: qualified — KeyCode also has `Pause`/`Tab`, and glob-importing both
            // enums lets the wrong one win silently.)
            keys: vec![
                (A::Up, vec![ArrowUp, KeyW]),
                (A::Down, vec![ArrowDown, KeyS]),
                (A::Left, vec![ArrowLeft, KeyA]),
                (A::Right, vec![ArrowRight, KeyD]),
                (A::Calendar, vec![]),
                (A::People, vec![]),
                (A::Guilds, vec![]),
                (A::Mobs, vec![]),
                (A::ItemsDex, vec![]),
                (A::Songs, vec![]),
                (A::Awards, vec![]),
                (A::StatsTab, vec![]),
                (A::Lore, vec![]),
                (A::Wriftheart, vec![]),
                (A::Craft, vec![]),
                (A::StatusTab, vec![]),
                (A::Slot1, vec![KeyE]), // SPACE removed — it silently attacked (a hidden 2nd bind); rebind it here if wanted
                (A::Slot2, vec![KeyX]),
                (A::Slot3, vec![KeyC]),
                (A::Slot4, vec![KeyV]),
                (A::Inventory, vec![KeyI]),
                (A::SkillTree, vec![KeyK]),
                (A::Map, vec![KeyM, Tab]),
                (A::TabPrev, vec![KeyQ]),
                (A::TabNext, vec![KeyR]), // js: moved off E (E is select/slot1)
                (A::Pause, vec![Escape]),
                (A::Trash, vec![KeyT]),
                (A::Sort, vec![KeyH]),
                (A::Interact, vec![KeyF]),
                (A::MenuConfirm, vec![Enter, NumpadEnter]),
                (A::DevPanel, vec![Backquote]),
                (A::God, vec![KeyG]),
            ],
            // The D-PAD is the shortcut cluster (movement rides the stick in free roam;
            // menus reroute the d-pad back to directions — see DpadDirs in poll_input).
            pads: vec![
                (A::Up, vec![]),
                (A::Down, vec![]),
                (A::Left, vec![]),
                (A::Right, vec![]),
                (A::Calendar, vec![]),
                (A::People, vec![]),
                (A::Guilds, vec![]),
                (A::Mobs, vec![]),
                (A::ItemsDex, vec![]),
                (A::Songs, vec![]),
                (A::Awards, vec![]),
                (A::StatsTab, vec![]),
                (A::Lore, vec![]),
                (A::Wriftheart, vec![]),
                (A::Craft, vec![]),
                (A::StatusTab, vec![]),
                (A::Slot1, vec![G::South]), // A
                (A::Slot2, vec![G::East]),  // B
                (A::Slot3, vec![G::West]),  // X
                (A::Slot4, vec![G::North]), // Y
                (A::Inventory, vec![G::RightTrigger, G::DPadDown]), // RB; ▼ shortcut
                (A::SkillTree, vec![G::DPadRight]),                 // ▶ shortcut
                (A::Map, vec![G::LeftTrigger, G::DPadLeft]),        // LB; ◀ shortcut
                (A::TabPrev, vec![G::LeftTrigger2]),  // LT
                (A::TabNext, vec![G::RightTrigger2]), // RT
                (A::Pause, vec![G::Start]),
                // DEVIATION: the js pad binds are CHORDS (trash = R3+RIGHT, sort =
                // SELECT+LEFT) — chords aren't in our binding model yet, so the thumb
                // clicks carry them solo for now.
                (A::Trash, vec![G::RightThumb]), // R3
                (A::Sort, vec![G::LeftThumb]),   // L3
                (A::Interact, vec![G::DPadUp]), // js pad 12: pushing UP at a door enters it
            ],
        }
    }
}

impl Bindings {
    fn key_binds(&self, action: Action) -> &[KeyCode] {
        self.keys.iter().find(|(a, _)| *a == action).map(|(_, k)| k.as_slice()).unwrap_or(&[])
    }
    fn pad_binds(&self, action: Action) -> &[GamepadButton] {
        self.pads.iter().find(|(a, _)| *a == action).map(|(_, b)| b.as_slice()).unwrap_or(&[])
    }

    /// The label for a "press X" prompt — DERIVED from the live binding at draw time, never
    /// typed at a call site. Shows the pad glyph whenever a controller is connected.
    pub fn prompt(&self, action: Action, pad_present: bool) -> &'static str {
        if pad_present
            && let Some(b) = self.pad_binds(action).first()
        {
            return pad_label(*b);
        }
        self.key_binds(action).first().map(|k| key_label(*k)).unwrap_or("--")
    }

    // --- Rebinding API (js Input.rebind / resetBindings / keyName / padNames) ---

    /// Bind `key` as the action's ONLY key, stripping it from every other action first
    /// (the js rule: a key binds exactly one action).
    pub fn rebind_key(&mut self, action: Action, key: KeyCode) {
        for (_, ks) in &mut self.keys {
            ks.retain(|k| *k != key);
        }
        if let Some((_, ks)) = self.keys.iter_mut().find(|(a, _)| *a == action) {
            *ks = vec![key];
        }
    }

    /// Same, on the pad table.
    pub fn rebind_pad(&mut self, action: Action, b: GamepadButton) {
        for (_, bs) in &mut self.pads {
            bs.retain(|x| *x != b);
        }
        if let Some((_, bs)) = self.pads.iter_mut().find(|(a, _)| *a == action) {
            *bs = vec![b];
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The CONTROLS row's KEY column: the first bound key's name.
    /// EVERY key bound to the action, joined (js keyName). Showing only the first hid
    /// second bindings — e.g. SPACE on ABILITY 1 attacked but never appeared, so it read
    /// as an un-settable control (Baz). Now the CONTROLS table tells the whole truth.
    pub fn key_name(&self, action: Action) -> String {
        let ks = self.key_binds(action);
        if ks.is_empty() {
            return "--".into();
        }
        ks.iter().map(|k| key_label(*k)).collect::<Vec<_>>().join("/")
    }

    /// The CONTROLS row's PAD column: EVERY bound button, '/'-joined (js padNames).
    pub fn pad_names(&self, action: Action) -> String {
        let bs = self.pad_binds(action);
        if bs.is_empty() {
            return "--".into();
        }
        bs.iter().map(|b| pad_label(*b)).collect::<Vec<_>>().join("/")
    }

    /// Snapshot as (slug, labels) rows for settings.json.
    pub fn export(&self) -> (Vec<BindRow>, Vec<BindRow>) {
        let keys = self
            .keys
            .iter()
            .map(|(a, ks)| (action_slug(*a).into(), ks.iter().map(|k| key_label(*k).into()).collect()))
            .collect();
        let pads = self
            .pads
            .iter()
            .map(|(a, bs)| (action_slug(*a).into(), bs.iter().map(|b| pad_label(*b).into()).collect()))
            .collect();
        (keys, pads)
    }

    /// Restore from settings.json rows. Unknown slugs/labels drop quietly (cross-build
    /// safety, same rule as the save file).
    pub fn import(&mut self, keys: &[BindRow], pads: &[BindRow]) {
        for (slug, labels) in keys {
            if let Some(a) = action_from_slug(slug)
                && let Some((_, ks)) = self.keys.iter_mut().find(|(x, _)| *x == a)
            {
                *ks = labels.iter().filter_map(|l| key_from_label(l)).collect();
            }
        }
        for (slug, labels) in pads {
            if let Some(a) = action_from_slug(slug)
                && let Some((_, bs)) = self.pads.iter_mut().find(|(x, _)| *x == a)
            {
                *bs = labels.iter().filter_map(|l| pad_from_label(l)).collect();
            }
        }
    }
}

/// The polled action state — gameplay's ONLY input surface. `held` is level-triggered;
/// `pressed` is edge-triggered and consumed once per fixed tick (`clear_pressed`), so a
/// press is seen by exactly one game tick, exactly like the JS endFrame() contract.
#[derive(Resource, Default)]
pub struct ActionState {
    held: [bool; ACTIONS.len()],
    pressed: [bool; ACTIONS.len()],
    latched: [bool; ACTIONS.len()],
    /// Injected holds (WRIFT_SHOT scenes) — poll_input ORs these in for one frame each
    /// (`held` itself is rewritten from the device every poll; `pressed` accumulates).
    test_held: [bool; ACTIONS.len()],
    pub pad_present: bool,
}

impl ActionState {
    pub fn held(&self, a: Action) -> bool {
        self.held[idx(a)]
    }
    pub fn pressed(&self, a: Action) -> bool {
        self.pressed[idx(a)]
    }
    /// Held AND not latched — the trigger for hold-to-repeat weapons (js:
    /// `Input.held(a) && !p.heldLatch[a]`).
    pub fn held_unlatched(&self, a: Action) -> bool {
        let i = idx(a);
        self.held[i] && !self.latched[i]
    }
    /// Inject a press (WRIFT_SHOT debug scenes only — real input comes from poll_input).
    pub fn press_for_test(&mut self, a: Action) {
        self.pressed[idx(a)] = true;
    }
    /// Inject a hold for ONE polled frame (WRIFT_SHOT debug scenes — walking the hero).
    pub fn hold_for_test(&mut self, a: Action) {
        self.test_held[idx(a)] = true;
    }
    /// Swallow this action while it stays held (a UI consumed it — port of heldLatch: the
    /// leftover hold can't swing a weapon the instant the menu closes). Poll clears it on
    /// release.
    pub fn latch(&mut self, a: Action) {
        let i = idx(a);
        if self.held[i] {
            self.latched[i] = true;
        }
    }
    /// Consume this tick's press — the js priority ladder (door > book > counter > npc)
    /// as explicit ordering: whoever acts on a press eats it so later systems stay quiet.
    pub fn consume(&mut self, a: Action) {
        self.pressed[idx(a)] = false;
    }
}

const STICK_DEAD: f32 = 0.5; // left stick past this reads as a held direction (JS threshold)

/// While ON (any non-free-roam screen — menus, codex, title, death), the pad D-PAD feeds
/// the four DIRECTION actions and its shortcut bindings go quiet; in free roam it fires
/// the shortcuts and the stick carries movement (port of the js dpadDirs switch).
#[derive(Resource, Default)]
pub struct DpadDirs(pub bool);

const DPAD: [GamepadButton; 4] =
    [GamepadButton::DPadUp, GamepadButton::DPadDown, GamepadButton::DPadLeft, GamepadButton::DPadRight];

/// Poll keyboard + every connected gamepad into [`ActionState`] (runs each render frame,
/// before the fixed ticks). The left stick maps onto the four move actions; its edge
/// crossings count as presses so menus and facing taps work from the stick too.
pub fn poll_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    bindings: Res<Bindings>,
    dpad_dirs: Res<DpadDirs>,
    mut state: ResMut<ActionState>,
    mut prev_stick: Local<[bool; 4]>,
) {
    state.pad_present = !pads.is_empty();
    let stick: Vec2 = pads.iter().map(|g| g.left_stick()).fold(Vec2::ZERO, |a, b| a + b);
    let stick_dir = [
        stick.y > STICK_DEAD,  // up (stick y-up)
        stick.y < -STICK_DEAD, // down
        stick.x < -STICK_DEAD, // left
        stick.x > STICK_DEAD,  // right
    ];
    for (i, a) in ACTIONS.into_iter().enumerate() {
        let mut held = bindings.key_binds(a).iter().any(|k| keys.pressed(*k));
        let mut pressed = bindings.key_binds(a).iter().any(|k| keys.just_pressed(*k));
        for g in &pads {
            for b in bindings.pad_binds(a) {
                // Menus own the D-pad as arrows — its shortcut bindings go quiet there.
                if dpad_dirs.0 && DPAD.contains(b) {
                    continue;
                }
                held |= g.pressed(*b);
                pressed |= g.just_pressed(*b);
            }
        }
        // The left stick + D-pad carry the four MOVE actions. Map each to its stick_dir /
        // DPAD slot by ACTION, not by array position: MenuConfirm was prepended to ACTIONS
        // (title Enter), so Up/Down/Left/Right no longer sit at indices 0..3 — a bare
        // `i < 4` fed the stick to the WRONG actions (Baz: "the analog stick walks me the
        // wrong directions"; WASD, bound per-action, was unaffected).
        if let Some(d) = match a {
            Action::Up => Some(0),
            Action::Down => Some(1),
            Action::Left => Some(2),
            Action::Right => Some(3),
            _ => None,
        } {
            held |= stick_dir[d];
            pressed |= stick_dir[d] && !prev_stick[d];
            if dpad_dirs.0 {
                for g in &pads {
                    held |= g.pressed(DPAD[d]);
                    pressed |= g.just_pressed(DPAD[d]);
                }
            }
        }
        held |= state.test_held[i];
        state.test_held[i] = false;
        state.held[i] = held;
        state.pressed[i] |= pressed; // accumulate until a fixed tick consumes it
        if !held {
            state.latched[i] = false; // a latch only survives while the button stays down
        }
    }
    *prev_stick = stick_dir;
}

/// Consume the edge-triggered presses — the LAST system of every fixed tick (js endFrame()).
pub fn clear_pressed(mut state: ResMut<ActionState>) {
    state.pressed = [false; ACTIONS.len()];
}

/// Pad button <-> on-screen name (port of PAD_NAMES: D-pad renders as the triangle glyphs —
/// font.rs bakes ▲▼◀▶). ONE table drives labels, capture whitelisting, and settings serde.
const PAD_LABELS: &[(GamepadButton, &str)] = {
    use GamepadButton as G;
    &[
        (G::South, "A"),
        (G::East, "B"),
        (G::West, "X"),
        (G::North, "Y"),
        (G::LeftTrigger, "LB"),
        (G::RightTrigger, "RB"),
        (G::LeftTrigger2, "LT"),
        (G::RightTrigger2, "RT"),
        (G::Select, "SEL"),
        (G::Start, "START"),
        (G::LeftThumb, "L3"),
        (G::RightThumb, "R3"),
        (G::DPadUp, "▲"),
        (G::DPadDown, "▼"),
        (G::DPadLeft, "◀"),
        (G::DPadRight, "▶"),
    ]
};

/// Key <-> on-screen name (font charset only: A-Z 0-9 + named specials). Keys outside this
/// table can't be rebound to (the font couldn't print their prompt) — the capture flow
/// ignores them.
const KEY_LABELS: &[(KeyCode, &str)] = {
    use KeyCode::*;
    &[
        (ArrowUp, "UP"), (ArrowDown, "DOWN"), (ArrowLeft, "LEFT"), (ArrowRight, "RIGHT"),
        (Space, "SPACE"), (Escape, "ESC"), (Tab, "TAB"), (Enter, "ENTER"), (Backquote, "TILDE"),
        (ShiftLeft, "LSHIFT"), (ShiftRight, "RSHIFT"), (ControlLeft, "LCTRL"), (ControlRight, "RCTRL"),
        (KeyA, "A"), (KeyB, "B"), (KeyC, "C"), (KeyD, "D"), (KeyE, "E"), (KeyF, "F"),
        (KeyG, "G"), (KeyH, "H"), (KeyI, "I"), (KeyJ, "J"), (KeyK, "K"), (KeyL, "L"),
        (KeyM, "M"), (KeyN, "N"), (KeyO, "O"), (KeyP, "P"), (KeyQ, "Q"), (KeyR, "R"),
        (KeyS, "S"), (KeyT, "T"), (KeyU, "U"), (KeyV, "V"), (KeyW, "W"), (KeyX, "X"),
        (KeyY, "Y"), (KeyZ, "Z"),
        (Digit0, "0"), (Digit1, "1"), (Digit2, "2"), (Digit3, "3"), (Digit4, "4"),
        (Digit5, "5"), (Digit6, "6"), (Digit7, "7"), (Digit8, "8"), (Digit9, "9"),
    ]
};

/// Short on-screen name for a pad button.
pub fn pad_label(b: GamepadButton) -> &'static str {
    PAD_LABELS.iter().find(|(x, _)| *x == b).map(|(_, l)| *l).unwrap_or("?")
}

/// Short on-screen name for a key.
pub fn key_label(k: KeyCode) -> &'static str {
    KEY_LABELS.iter().find(|(x, _)| *x == k).map(|(_, l)| *l).unwrap_or("?")
}

pub fn key_from_label(l: &str) -> Option<KeyCode> {
    KEY_LABELS.iter().find(|(_, x)| *x == l).map(|(k, _)| *k)
}

pub fn pad_from_label(l: &str) -> Option<GamepadButton> {
    PAD_LABELS.iter().find(|(_, x)| *x == l).map(|(b, _)| *b)
}

/// Is this key allowed as a binding? (= the font can print its name.)
pub fn key_bindable(k: KeyCode) -> bool {
    KEY_LABELS.iter().any(|(x, _)| *x == k)
}
