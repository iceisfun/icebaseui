//! # baseui-core
//!
//! Dependency-free foundational primitives for the BaseUI framework.
//!
//! This crate deliberately pulls in **no** rendering, windowing, or platform
//! dependencies so that it can be shared by every downstream crate — the main
//! `baseui` crate as well as optional crates such as `baseui-dock`,
//! `baseui-graph`, and `baseui-plot` — without dragging in wgpu or winit.
//!
//! It provides:
//!
//! - [`geometry`]: [`Point`], [`Size`], [`Rect`], [`Vec2`], and [`Insets`] — all
//!   in logical (DPI-independent) pixels, origin top-left, y down.
//! - [`color`]: an RGBA [`Color`] with hex parsing and sRGB/linear conversion.
//! - [`id`]: process-unique, monotonically increasing [`Id`]s.
//! - [`font`]: [`FontId`] — which face a run of text is drawn in (UI, monospace,
//!   or an icon font).
//! - [`paint`]: the [`Scene`](paint::Scene) display list — a flat, backend-agnostic
//!   list of rects, text, and decorations, with a clip stack and an overlay layer
//!   so popups escape their parents' clips. Widgets emit into it; the renderer in
//!   the `baseui` crate consumes it. This is the seam that keeps the widget tree
//!   from knowing anything about the GPU.
//! - [`reactive`]: a small single-threaded reactive runtime (signals, memos,
//!   and effects) that powers BaseUI's retained + reactive widget tree.

// Every public item carries documentation. This is a lint, not a convention,
// because a convention is what let 213 items go undocumented in the first place.
#![warn(missing_docs)]

pub mod color;
pub mod font;
pub mod geometry;
pub mod id;
pub mod paint;
pub mod reactive;

pub use color::Color;
pub use font::FontId;
pub use geometry::{Insets, Point, Rect, Size, Vec2};
pub use id::Id;
pub use reactive::{Memo, Signal, create_effect, create_memo, create_signal, set_on_change};
