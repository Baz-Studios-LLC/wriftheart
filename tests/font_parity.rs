//! Font metrics parity: `font::measure` must match the JS `Font.measure` (layout math all
//! over the UI depends on it — a 1px drift misaligns every right-aligned/centred label).

use wriftheart::gfx::font::measure;

mod golden {
    include!("data/font_golden.rs");
}

#[test]
fn measure_matches_js() {
    for (text, want) in golden::MEASURE {
        assert_eq!(measure(text), *want, "measure({text:?}) drifted from JS");
    }
}
