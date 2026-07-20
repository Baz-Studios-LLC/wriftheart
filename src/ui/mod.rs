//! ui — the shared interface primitives (panels, labels, bars, list navigation).
//! Screens compose these; none draw their own chrome (see the reuse rule in PORT.md).

pub mod widgets;

pub use widgets::{
    bar, border_strips, cell, frame_rect, label, list_window, panel, set_bar, BarSpec, ListNav,
    ListWindow, Pen,
};
