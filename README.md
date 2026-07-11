# BaseUI

A modern, extensible **desktop UI framework** for engineering applications —
IDEs, debuggers, visualization tools, editors, CAD software, and custom internal
apps. Written in Rust.

> Status: **early foundation.** The core primitives, reactive runtime, theme
> engine, and a wgpu window shell are in place. The widget system, layout
> engine, and flagship widgets (TreeView, PropertyView) are next.

## Architecture decisions

| Concern            | Choice                     | Why |
| ------------------ | -------------------------- | --- |
| Windowing          | **winit 0.30**             | Cross-platform, standard in the Rust GUI ecosystem. |
| Rendering          | **wgpu 26**                | GPU-accelerated (Vulkan/Metal/DX12); handles heavy viewports/plots/scenes. |
| UI model           | **Retained + reactive**    | State lives in signals; only changed subtrees re-render. |
| Core split         | **`baseui-core`** is dep-free | Geometry, color, ids, and the reactive runtime are reusable by every optional crate without pulling in wgpu. |

## Workspace layout

```
baseui/
├── crates/
│   ├── baseui-core/     # dependency-free primitives
│   │   ├── geometry     # Point, Size, Rect, Vec2, Insets
│   │   ├── color        # RGBA Color + hex parsing + sRGB->linear
│   │   ├── id           # process-unique Ids
│   │   └── reactive     # signals, memos, effects (the reactive runtime)
│   └── baseui/          # the framework crate (winit + wgpu)
│       ├── theme        # design tokens: palette, spacing, radius, type, motion
│       ├── render       # wgpu backend (currently a themed clear)
│       └── app          # App shell + winit event loop
└── examples/
    └── hello/           # opens a themed window
```

Large optional systems will live in their own crates: `baseui-dock`,
`baseui-graph`, `baseui-terminal`, `baseui-code`, `baseui-plot`, etc.

## Running

```bash
cargo run -p hello                 # dark theme
BASEUI_THEME=light cargo run -p hello
RUST_LOG=info cargo run -p hello   # backend logging
```

Opens a 1000×700 window cleared to the active theme's background color.

## Testing

```bash
cargo test -p baseui-core   # geometry, color, ids, and the reactive runtime
cargo clippy --workspace
```

## Roadmap

- [x] **M1 — Foundation:** workspace, core primitives, reactive runtime, theme
      engine, wgpu window shell.
- [ ] **M2 — 2D painter:** batched rounded-rects, borders, clipping, text/glyph
      atlas — a display list the widget tree can emit.
- [ ] **M3 — Widget tree + layout:** `Widget` trait, retained node tree wired to
      signals, layout solver (Row/Column/Grid/Stack/Scroll), input routing.
- [ ] **M4 — Core widgets:** Label, Button, Toggle, Checkbox, TextBox, Slider,
      DragValue, ComboBox.
- [ ] **M5 — Flagship widgets:** TreeView and PropertyView (Blender-Outliner /
      Properties-editor grade).
- [ ] **M6 — App frame:** menu bar, toolbar, panels, status bar, tabs.
- [ ] **M7 — Systems:** command system, event bus, shortcut manager, persistence.
- [ ] **M8 — Extensibility:** plugin registration, icon packs, optional docking
      (`baseui-dock`), optional Lua (mlua) scripting.

## License

MIT OR Apache-2.0
