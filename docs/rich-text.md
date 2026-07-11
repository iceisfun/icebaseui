# Rich / marked-up text — design plan

> Status: **partly built.**
>
> - **Decorations are done** — `paint::Decoration` (`Underline` / `Squiggle` /
>   `Strikethrough`) and the `MODE_SQUIGGLE` shader path ship today. Use-cases 1,
>   2 and 3 below are all buildable.
> - **`TextArea`** ([`widget/textarea.rs`]) is the first consumer: per-line
>   colored spans from a pluggable highlighter, plus squiggled diagnostics.
> - **Still planning:** the `StyledText`/`TextLayout` galley engine — i.e.
>   use-case 4 with **soft wrapping in a proportional font**. That is the one
>   piece that genuinely needs it, and nothing below it is blocked on it.
>
> The reason the split falls here: without wrapping, every x-position on a line
> is a *prefix sum of advances*, so caret, hit-testing, and run placement are all
> one cheap pass. Wrapping is what forces a real layout cache.

## Motivating use cases

1. **Hex editor** — a fixed-width grid: `HH HH HH …` byte columns beside an ASCII
   pane. Needs the monospace font, exact cell alignment, and independent color
   per byte/nibble and per ASCII cell.
2. **Colored byte/ASCII markup** — bytes tinted by class (zero, printable,
   control, high), selection ranges, diff add/remove, search hits.
3. **Squiggle underlines** — editor-style wavy underlines under a text range in a
   color (error = red, warning = amber, info = blue), plus straight underline,
   strikethrough, and background highlight.
4. **Marked-up text views** — log/console/markdown output where spans within a
   line carry foreground color, optional background, and bold/italic.

## Layered architecture

```
StyledText            (data: spans with style)
   │  shape + wrap
   ▼
TextLayout / "Galley" (positioned glyphs + decorations, cached)
   │  emit
   ▼
Scene primitives      (glyph quads + decoration quads)  ← painter, already exists
```

Keeping *styling*, *layout*, and *painting* as three layers means the hex editor,
a log view, and a markdown view all feed the same layout+paint path; only the
span construction differs.

### 1. Styling data — `StyledText`

```rust
pub struct TextStyle {
    pub color: Color,
    pub font: FontId,        // Ui | Mono | Custom(u32)
    pub weight: Weight,      // Normal | Bold   (synthetic bold until real faces)
    pub italic: bool,
    pub background: Option<Color>,
    pub decoration: Option<Decoration>,
}

pub struct Span { pub text: Range<usize> /* into the string */, pub style: TextStyle }
pub struct StyledText { pub text: String, pub spans: Vec<Span> }
```

A builder (`StyledText::builder().push("2A", red).push(" ", none)…`) covers the
hex/log cases without hand-managing ranges.

### 2. Layout — `TextLayout` (a.k.a. Galley)

Produced by a shaper from `StyledText` + wrap width + font metrics; **cached** by
content hash (this is the expensive step). Contains:

- positioned glyphs: `{ glyph_id, font, x, y, size, color }`
- decoration runs: `{ kind, color, x0, x1, y }`
- per-line metrics + total size, plus a cursor/hit-test index (byte ⇄ x/line).

Hit-testing (x,y → byte offset) and caret geometry live here — needed by any text
selection UI, the hex editor cursor, and future TextBox/TextArea widgets.

Shaping starts as today's advance-width layout over runs; it can later be swapped
for a real shaper (cosmic-text / rustybuzz) behind this same type without
touching callers.

### 3. Painting — new decoration primitive

The glyph path already exists (`mode = glyph`). Add **one** primitive for
decorations so squiggles/underlines batch through the same pipeline:

```rust
pub enum Decoration { Underline, Squiggle, Strikethrough }        // in baseui-core::paint
Primitive::Decoration { rect: Rect, color: Color, kind: Decoration, thickness: f32 }
```

Implementation in `quad.wgsl` as new `mode` values, drawn analytically (no atlas):

- **Underline / strikethrough** — a 1–2px SDF rounded rect (already expressible;
  the primitive is mostly sugar + correct baseline offset).
- **Squiggle** — a wavy line via an SDF in the fragment shader: for a quad of
  height `2*amp`, `dist = |local.y - amp*sin(local.x * freq)| - thickness`,
  anti-aliased with `fwidth`. Amplitude/'frequency scale with font size. Color is
  independent of the underlined text (red/amber/blue).
- **Background highlight** — already just a `RectShape` drawn behind the run.

No new pipeline, no new bind group — only additional `mode` branches and instance
`params`, so it stays one batched draw call.

## Hex editor specifics

- Uses `FontId::Mono`; column x-positions come from `Fonts::measure("0", size,
  Mono)` (monospace ⇒ every advance equal) so bytes align exactly.
- Byte color from a classifier fn `u8 -> Color` (theme-driven palette: zero /
  printable / control / high). Selection & diff overlay as background rects;
  annotations as squiggles under a byte range.
- ASCII pane is a second `StyledText` sharing the same row layout.
- Large buffers: only the visible rows are turned into a `TextLayout` each frame
  (virtualized), so cost is bound by viewport, not file size.

## What exists today

- **Monospace font** loaded alongside the UI font (`Fonts { ui, mono }`),
  selectable via `TextShape.font`.
- **`Fonts::measure()` + `char_advance()` + line metrics** — the measurement
  primitives every layer above depends on.
- **Run-based drawing**: colored runs on one line = several `TextShape`s
  positioned by summing advances. `HexView` colors bytes this way; `TextArea`
  colors syntax this way.
- **The decoration primitive**: `Scene::squiggle()` / `Scene::underline()` /
  `push_decoration()`. Underline and strikethrough flatten to plain rects;
  squiggle gets its own shader mode, drawn analytically from
  `abs(y - sin(x)) - thickness` with `fwidth()` anti-aliasing — no atlas, one
  instance per run.
- **`TextArea`**: multi-line editing, line numbers, selection, clipboard,
  virtualized paint, `highlighter(|line| -> Vec<Span>)`, and
  `checker(|doc| -> Vec<Diagnostic>)`.

Still deferred: `StyledText`/`TextStyle`, the cached `TextLayout` galley, and
soft wrapping with per-span font styles (bold/italic runs). A `TextArea` that
wraps is the concrete thing that unlocks.
