# BaseUI

A modern, extensible **desktop UI framework** for engineering applications —
IDEs, debuggers, visualization tools, editors, CAD software, and custom internal
apps. Written in Rust.

> Status: **M1–M7 complete.** Rendering (wgpu SDF painter + glyph/icon atlas),
> the retained + reactive widget tree, layout, pointer & keyboard input, the full
> widget set (TreeView, PropertyView, HexView, TextBox, **TextArea**, ComboBox,
> tabs, resizable splits), the app frame (menu/toolbar/status), the systems layer
> (commands + Command Palette, shortcuts, event bus, persistence), Lua scripting,
> and **docking** (drag/split/detach to floating windows). Next: M8 extensibility.

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
│   ├── baseui-lua/      # OPTIONAL Lua scripting (commands/events/status glue)
│   └── baseui/          # the framework crate (winit + wgpu)
│       ├── theme        # design tokens: palette, spacing, radius, type, motion
│       ├── text         # fonts (UI + mono), measurement, global text scale
│       ├── render       # wgpu: shared GpuContext + per-window WindowRenderer
│       ├── layout       # box Constraints
│       ├── event        # normalized input events
│       ├── icon         # Icon type + glyph icons + embedded font-gis pack
│       ├── command      # command registry + shortcuts + Command Palette (F1)
│       ├── bus          # typed event bus (subscribe/publish)
│       ├── persist      # JSON UI-state store (layout persistence)
│       ├── focus        # keyboard focus (thread-local)
│       ├── clipboard    # system clipboard (text)
│       ├── window       # multi-window: open/close request queue
│       ├── popup        # popup modality (keyboard capture)
│       ├── widget       # Widget trait + Label, Button, Column, Row, Checkbox,
│       │                #   Slider, DragValue, TextBox, TextArea, ComboBox,
│       │                #   HexView, ScrollArea, TreeView, PropertyView, TabView,
│       │                #   Split, DockArea, MenuBar, Toolbar, StatusBar
│       └── app          # App shell + winit loop + pointer/keyboard routing
├── docs/
│   ├── rich-text.md     # styled runs, squiggles (built); wrapping galley (planned)
│   ├── document-tabs.md # planned VS Code-style tabs for the content area
│   ├── docking.md       # dock plan: id-tree design + phases (Phase 0 done)
│   └── scripting.md     # Lua scope: composition/glue, NOT widget authoring
└── examples/
    ├── hello/           # painter demo (raw Scene: rects, text, clipping)
    ├── counter/         # widget + reactive-signal demo
    ├── widgets/         # control gallery starring HexView (colored bytes)
    ├── inspector/       # TreeView + PropertyView (Blender-style)
    ├── dock/            # dockable panels: drag, split, detach, redock
    └── editor/          # TextArea as a code area: syntax colors + squiggles
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
- [x] **M2 — 2D painter:** `Scene` display list + a single instanced-quad wgpu
      pipeline drawing SDF rounded-rects, borders, per-quad clipping, and text
      (ab_glyph glyph atlas, system font via fontdb).
- [x] **M3 — Widget tree + layout:** `Widget` trait (layout/paint/event passes),
      Flutter-style box `Constraints`, `Column`/`Row` containers, `Label`
      (static + reactive) and `Button`, pointer input routing, and the
      reactive→repaint bridge (`set_on_change`). Monospace font + text
      measurement landed as rich-text prerequisites (see `docs/rich-text.md`).
- [x] **M4 — Core widgets:** Checkbox, Slider, Blender-style DragValue, a
      **HexView** (monospace grid, per-byte class coloring, hover, wheel scroll,
      reactive ASCII toggle), and — once keyboard/focus landed in M7 — an
      editable **TextBox** (caret, selection, clipboard, password) and
      **ComboBox**. `ScrollArea` landed in M5; a `Grid` container is still open.
      **`TextArea`** followed: multi-line editing with a pluggable syntax
      highlighter and squiggle-underlined diagnostics (see the `editor` example).
- [x] **M5 — Flagship widgets:** `ScrollArea`, `TreeView` (expand/collapse,
      colored type icons, selection, hover), and `PropertyView` (collapsible
      groups with colored section icons; rows embed real editor widgets). See
      the `inspector` example (Blender Outliner + Properties layout).
- [x] **M6 — App frame:** `MenuBar` (dropdowns via a Scene overlay layer),
      `Toolbar` (icon/text buttons, toggles, spacers), `StatusBar` (reactive
      left/right items), `TabView`, resizable `Split` (horizontal + vertical,
      fixed + flexible panes with drag gutters), and an `icon` system with the
      embedded **font-gis** pack (367 icons) rendered through the glyph atlas.
      See the `inspector` example (full app shell).
- [x] **M7 — Systems:** command system (registry with id/title/category/icon/
      color/shortcut + handler; run/all/search), shortcut manager
      (chord→command), a searchable **Command Palette** (F1), keyboard input +
      focus, a typed **event bus** (subscribe/publish/unsubscribe), and
      **persistence** (JSON store: split sizes, active tab, group collapse, tree
      expansion, scroll offset, window geometry — restored on launch, saved on
      close). Plus event-capture, menu-item icons/options, and tree glyph icons.
- [~] **M8 — Extensibility:** ✅ optional **Lua scripting** (`baseui-lua`):
      scripts register commands (icon/color/category/shortcut), bind shortcuts,
      publish/handle named events, and contribute status items — appearing in the
      Command Palette like native commands. The `Widget` trait is deliberately
      **not** exposed; see `docs/scripting.md`. Remaining: declarative Lua panels,
      a Rust plugin-registration API, and optional docking (`baseui-dock`).

## License

MIT OR Apache-2.0
