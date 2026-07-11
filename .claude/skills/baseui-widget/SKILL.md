---
name: baseui-widget
description: Author a BaseUI widget — the Widget trait's three passes, the event-consumption rule, text measurement and carets, reactive state, persistence, and the traps that produce bugs you will not see in the diff. Use when writing or reviewing any widget in this repo, or when a widget misbehaves (click-through, caret drift, stale repaint).
---

# Authoring a BaseUI widget

A widget is a struct implementing three methods. There is no macro, no VDOM, no
hidden lifecycle. The framework will not stop you from breaking the rules below,
so know them.

## The trait

```rust
impl Widget for MyWidget {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
        // Ask for a size WITHIN the constraints. Measure text via cx.fonts.
        c.constrain(Size::new(120.0, 24.0))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        // Emit primitives. `bounds` is where layout put you, in logical pixels.
        scene.rounded_rect(bounds, cx.theme.palette.surface, cx.theme.radius.sm);
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // React. Consume what you handle.
    }
}
```

Builders are chainable and consume `self` (`.font_size(13.0).line_numbers()`).
All coordinates are logical pixels, origin top-left, y down.

## Rule 1 — children before chrome

**A container forwards events to its children BEFORE interpreting them as its own
chrome, and touches its chrome only if nothing consumed the event.**

```rust
fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
    for (child, rect) in self.children_mut() {
        child.event(cx, rect, cx.effective(event));   // children first
    }
    if cx.is_consumed() {
        return;                                        // someone else owns this
    }
    // ...now, and only now, my own header/tab/gutter hit-testing
}
```

Get this backwards and clicks fall *through* an open dropdown onto whatever is
behind it. That is not a hypothetical: a ComboBox selection used to toggle the
collapse header underneath it.

Consume with `cx.consume()` when you act on an event. `cx.effective(event)` hands
later siblings a synthetic `PointerLeft` once an event is consumed, so their hover
state clears instead of sticking.

## Rule 2 — measure with `Fonts`, never by hand

`Fonts::char_advance` is the **single definition of how far the pen moves** — the
renderer steps by exactly it. Anything you compute by summing it lands precisely
on the drawn glyphs, at any DPI and any text scale.

For anything interactive, lay the line out once and ask it questions:

```rust
let line = cx.fonts.layout_line(text, self.font_size, FontId::Mono);

let col = line.col_at(mouse_x);            // click -> caret column (nearest boundary)
let x = line.x_of(col);                    // column -> caret x
let (x0, x1) = line.span(sel_a, sel_b);    // selection / squiggle / run background
let hovered = line.char_at(mouse_x);       // which glyph is UNDER the pointer (or None)
```

`col_at` and `char_at` are different questions: a caret goes *between* characters,
a tooltip is *on* one.

**Never** re-derive advances from a rasterized pixel size. It rounds, and the
error accumulates along the line — the caret drifts further from the text the
longer the line gets, only on fractional-DPI displays, only at some font sizes.

Vertical placement comes from `cx.fonts.metrics(size, font)` and is stated
relative to the **baseline** (`ascent`, and `descent` as a positive depth) — never
as an offset from the bottom of the line box, which sits a different distance
below the baseline for every face.

Full API: `docs/text.md`.

## Rule 3 — global state must mark dirty

Signal writes repaint every window for free. A plain `static` does not:

```rust
pub fn set_thing(v: Thing) {
    THING.with(|t| t.set(v));
    crate::window::mark_dirty();   // <- or only the dispatching window repaints
}
```

Symptom when you forget: a second window updates "late", whenever something else
happens to repaint it.

## Reactive state

```rust
let count = create_signal(0);
Label::reactive(move || format!("count: {}", count.get()));  // re-reads on change
```

`Signal<T>` is cheap to clone and `Copy`-ish to capture. Read in a closure and the
label re-renders when it changes. Widgets that own a large buffer (a document, a
byte slice) should own it directly instead — a signal clones on every read, which
is fine for a line and wasteful for a file.

## Persistence

Opt in with `.persist("some.key")`, then implement:

```rust
fn persist_save(&self) -> Option<Value>;
fn persist_restore(&mut self, value: &Value);
```

Save only *UI* state — collapse, scroll, split sizes, active tab. Not the user's
data.

## Animation

Call `anim::request_frame()` during `paint` while you want to keep animating; the
app schedules ~60Hz frames and drops back to waiting when nobody asks. Do not spin
a timer.

## Painting

The `Scene` is a flat display list: `rect`, `rounded_rect`, `stroke_rect`,
`text_font`, `squiggle`, `underline`, plus `push_clip`/`pop_clip`. For popups and
dropdowns, wrap emission in `scene.begin_overlay()` / `end_overlay()` — the
overlay layer draws above everything and escapes enclosing clips. Take keyboard
modality too, via `popup::set_open(true)`, or a focused text field will keep
eating keys behind your menu.

## Before you call it done

```sh
cargo fmt --all
cargo clippy --workspace --all-targets    # must be clean
cargo test --workspace
timeout 6 cargo run -p editor             # a window must still open
```

Write tests that pin *behavior* and name them as the claim they make. Drive the
widget through its real `event()` path with synthesized `InputEvent`s rather than
calling private helpers — that is the layer where the wiring is actually wrong.
Start font-dependent tests with
`let Some(fonts) = Fonts::load() else { return };` so headless machines skip.

## Worked examples in the tree

- `widget/button.rs` — the smallest complete widget.
- `widget/textarea.rs` — carets, hit-testing, colored runs, squiggles, undo.
- `widget/property.rs` — a container that gets the event-ordering rule right.
- `widget/dock.rs` — id-tree vs content-registry; moving tabs moves *ids*.
