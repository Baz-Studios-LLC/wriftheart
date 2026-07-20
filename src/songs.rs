//! songs.rs — the flute's songbook (port of js/songs.js, pure data + the shared
//! note-arrow art). The flute has FOUR notes, one per direction, tuned to a pentatonic
//! subset (A C D G) so any noodling still sounds like music. A song is a string of
//! U D L R; play-mode (app/flute.rs) keeps the tail of what you've played and asks
//! [`match_tail`] after every note — playing a song's notes casts it, but ONLY if
//! you've been taught it. THE FLUTE ITSELF NEVER SHOWS NOTATION — the SONGS codex page
//! is the only place a song's notes are written down.
//!
//! Add a song: one row here + a cast branch in app/flute.rs. Keep every song EXACTLY
//! FOUR notes (equal lengths can't suffix-shadow each other) and distinct.

/// One of the four notes: its letter, pitch, and display colour.
pub struct Note {
    pub ltr: char,
    pub word: &'static str,
    pub freq: f32,
    pub col: u32,
}

pub const NOTES: [Note; 4] = [
    Note { ltr: 'U', word: "UP", freq: 440.00, col: 0xb48ae8 },
    Note { ltr: 'D', word: "DOWN", freq: 261.63, col: 0xe8a84a },
    Note { ltr: 'L', word: "LEFT", freq: 293.66, col: 0x5ac8e8 },
    Note { ltr: 'R', word: "RIGHT", freq: 392.00, col: 0xe87a9a },
];

pub fn note_idx(ltr: char) -> usize {
    NOTES.iter().position(|n| n.ltr == ltr).unwrap_or(0)
}

pub struct SongDef {
    pub id: &'static str,
    pub name: &'static str,
    pub notes: &'static str,
    pub mana: i32,
    pub col: u32,
    pub desc: &'static str,
    /// What the codex shows while the song is still unlearned.
    pub hint: &'static str,
}

/// The songbook (js LIST, verbatim).
pub static LIST: &[SongDef] = &[
    SongDef { id: "returning", name: "SONG OF RETURNING", notes: "URDL", mana: 10, col: 0x9ad0ff,
        desc: "CARRIES YOU HOME, OR TO ANY TOWN YOU KNOW",
        hint: "THE BARD ON ANY TAVERN STAGE TEACHES IT" },
    SongDef { id: "stormcall", name: "STORMCALL", notes: "DUDU", mana: 6, col: 0x7090d8,
        desc: "CALLS THE RAIN - OR CLEARS THE SKY",
        hint: "WRITTEN DOWN IN SOME TAVERN OR OTHER" },
    SongDef { id: "sunsong", name: "THE SUN SONG", notes: "RULD", mana: 6, col: 0xfcd23b,
        desc: "HURRIES THE SUN TO DAWN OR DUSK",
        hint: "THE DAWN PRIESTS KEEP IT IN THEIR CHAPELS" },
    SongDef { id: "opening", name: "SONG OF OPENING", notes: "LRLR", mana: 4, col: 0xc8a060,
        desc: "OPENS WHAT THE SINGING STONES KEEP SHUT",
        hint: "TOLD IN OLD TALES AROUND WILD CAMPFIRES" },
    SongDef { id: "canticle", name: "THE BELL CANTICLE", notes: "ULRD", mana: 4, col: 0xf0ead0,
        desc: "A TOLLING PEAL THAT THROWS BACK ALL AROUND YOU",
        hint: "BURIED WITH THE CHOIR IN SOME DARK PLACE" },
    SongDef { id: "wardsong", name: "WARDSONG", notes: "LRRL", mana: 6, col: 0x7fb0e0,
        desc: "A HUM THAT TURNS ASIDE BLOWS FOR A WHILE",
        hint: "THE KINGSGUARD SANG IT - WRITTEN IN SOME CASTLE" },
    SongDef { id: "lullaby", name: "THE LULLABY", notes: "URLD", mana: 8, col: 0xa894e0,
        desc: "LULLS THE FOES AROUND YOU TO SUDDEN SLEEP",
        hint: "A CRADLE-RHYME SET DOWN IN SOME TOWN BOOK" },
    SongDef { id: "greensong", name: "GREENSONG", notes: "DLRU", mana: 4, col: 0x7ad86a,
        desc: "COAXES THE SOWN FIELDS TO SUDDEN FRUIT",
        hint: "WILDFOLK SING IT - KEPT IN SOME CAMP TALE" },
];

pub fn get(id: &str) -> Option<&'static SongDef> {
    LIST.iter().find(|s| s.id == id)
}

/// The song (if any) whose full note string the played tail ends with (js match).
pub fn match_tail(seq: &str) -> Option<&'static SongDef> {
    LIST.iter().find(|s| seq.ends_with(s.notes))
}

/// The shared chunky note-arrow on a 7x7 grid (js ARROW + drawArrow's rotation): the
/// cells to fill for a letter, in grid coords — every place notes appear (flute
/// overlay, codex) rasterises exactly these.
const ARROW: [&str; 7] = ["...X...", "..XXX..", ".XXXXX.", "XXXXXXX", "..XXX..", "..XXX..", "..XXX.."];

pub fn arrow_cells(ltr: char) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for (r, row) in ARROW.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch != 'X' {
                continue;
            }
            let (r, c) = (r as i32, c as i32);
            let (cx, cy) = match ltr {
                'D' => (c, 6 - r),
                'L' => (r, 6 - c),
                'R' => (6 - r, c),
                _ => (c, r), // 'U' as authored
            };
            cells.push((cx, cy));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_are_four_notes_and_distinct() {
        for s in LIST {
            assert_eq!(s.notes.len(), 4, "{} must be exactly four notes", s.id);
            assert!(s.notes.chars().all(|c| "UDLR".contains(c)));
        }
        for (i, a) in LIST.iter().enumerate() {
            for b in &LIST[i + 1..] {
                assert_ne!(a.notes, b.notes, "{} and {} share notes", a.id, b.id);
            }
        }
    }

    #[test]
    fn tail_matching() {
        assert_eq!(match_tail("RRDUDU").map(|s| s.id), Some("stormcall"));
        assert_eq!(match_tail("URDL").map(|s| s.id), Some("returning"));
        assert!(match_tail("URD").is_none());
        assert!(match_tail("").is_none());
    }

    #[test]
    fn arrows_rotate() {
        // Every rotation keeps the same 25 filled cells inside the 7x7 box.
        for l in ['U', 'D', 'L', 'R'] {
            let cells = arrow_cells(l);
            assert_eq!(cells.len(), 25);
            assert!(cells.iter().all(|&(x, y)| (0..7).contains(&x) && (0..7).contains(&y)));
        }
    }
}
