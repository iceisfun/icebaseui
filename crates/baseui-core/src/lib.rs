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
//! - [`geometry`]: `Point`, `Size`, `Rect`, `Vec2`, and `Insets`.
//! - [`color`]: an RGBA [`Color`](color::Color) type with hex parsing.
//! - [`id`]: process-unique, monotonically increasing [`Id`](id::Id)s.
//! - [`reactive`]: a small single-threaded reactive runtime (signals, memos,
//!   and effects) that powers BaseUI's retained + reactive widget tree.

pub mod color;
pub mod geometry;
pub mod id;
pub mod paint;
pub mod reactive;

pub use color::Color;
pub use geometry::{Insets, Point, Rect, Size, Vec2};
pub use id::Id;
pub use reactive::{Memo, Signal, create_effect, create_memo, create_signal};
