# BaseUI — working notes

A desktop UI framework in Rust (winit + wgpu) for engineering apps: IDEs,
debuggers, CAD, visualization. Retained widget tree + reactive signals.

```
crates/baseui-core   dependency-free: geometry, color, id, font, paint (Scene), reactive
crates/baseui        the framework: app, render, widget, text, theme, command, undo, ...
crates/baseui-lua    optional mlua scripting (commands/events/glue -- NOT widgets)
examples/            hello, counter, widgets, inspector, dock, editor
docs/                text.md, rich-text.md, docking.md, scripting.md, document-tabs.md
```

## Invariants

These are load-bearing. Each one is here because breaking it produced a bug that
was *not* obvious from reading the diff.

**1. `Fonts` is the single definition of how far the pen moves.**
The renderer steps its pen by exactly `Fonts::char_advance`. Never re-derive
advances from a rasterized pixel size — that rounds, and the error accumulates
along the line, so a caret drifts further from the text the longer the line gets.
It only shows on fractional-DPI displays, at some font sizes. Use
`Fonts::layout_line` -> `Line` for carets, hit-testing, and run placement; do not
hand-roll a prefix sum. See `docs/text.md`.

**2. Containers forward events to children BEFORE interpreting them as their own
chrome.** Handle your own chrome only if `!cx.is_consumed()`. Violating this makes
clicks fall *through* an open dropdown onto whatever is behind it — which is how a
ComboBox selection ended up toggling a collapse header underneath it.

**3. Global (non-signal) state must call `window::mark_dirty()` when it changes.**
Signal writes repaint every window automatically; a plain `static` does not. Text
scale and command dispatch both had to learn this: only the window that dispatched
the change was repainting, so a second window updated "late".

**4. A chord is not typing.** The platform reports `text` for chords too (winit
hands back `"z"` for Ctrl+Z). `app.rs::suppresses_text` gates this in ONE place.
Alt is deliberately excluded from the test: AltGr arrives as Ctrl+Alt and is how
much of the world types `@`, `#`, `€`.

**5. Vertical text placement is stated in terms of the baseline**, from
`Fonts::metrics` — never as an offset from the bottom of the line box, which sits
a different distance below the baseline for every face and size. `LineMetrics`
reports `descent` as a *positive depth* for exactly this reason.

**6. Docking moves ids, never widgets.** `DockNode` is an id tree (layout);
`Panel` is the content registry. Moving a tab edits the id tree; it never
re-parents a `Box<dyn Widget>`. Mutation order: remove-without-pruning →
insert/split → normalize once.

## Conventions

- **Comments state constraints and reasons**, not what the next line does. If a
  comment could be deleted without losing information, delete it.
- **Tests pin behavior, not implementation.** Name them as the claim they make
  (`the_squiggle_sits_below_the_baseline_and_inside_the_row`), and make the
  failure message say what broke.
- Tests that need fonts start with `let Some(fonts) = Fonts::load() else { return };`
  so a headless box without system fonts skips rather than fails.
- Public items are documented; `missing_docs` is warned on in both crates.
- Logical (DPI-independent) pixels everywhere, origin top-left, y down.
- No backticks in commit messages (bash substitutes them).

## Checks before committing

```sh
cargo fmt --all
cargo clippy --workspace --all-targets    # must be clean
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
timeout 6 cargo run -p editor             # a real window must still open
```

Auto-push after commits is authorized (`git@github.com:iceisfun/icebaseui.git`).
