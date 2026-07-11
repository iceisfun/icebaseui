# Text measurement

Everything that draws text needs to know how big it is: a button sizing itself, a
caret finding its x, a table truncating a cell, a tooltip wrapping a sentence.
BaseUI answers all of it through one type — [`Fonts`] — reachable from Rust and
from Lua, with the same numbers.

The rule that makes it trustworthy:

> **`Fonts` is the single definition of how far the pen moves.** The renderer
> steps its pen by exactly `Fonts::char_advance` (see `render::glyph::push_text`).
> Anything you compute by summing advances therefore lands *precisely* on the
> drawn glyphs — at any DPI, at any text scale.

Do not re-derive advances from a rasterized pixel size. That rounds, and the error
accumulates along the line: the caret drifts further from the text the longer the
line gets. (This was a real bug; see the tests in `text.rs`, which pin it shut.)

## Getting a `Fonts`

Inside a widget pass, it is already on the context:

```rust
fn layout(&mut self, cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
    let size = cx.fonts.measure(&self.label, 14.0, FontId::Ui);
    c.constrain(size + Size::new(24.0, 12.0))
}
```

`cx.fonts` exists in all three passes (`LayoutCx`, `PaintCx`, `EventCx`) — you
need it in `event` too, because hit-testing text is a measurement question.

Outside a pass — a command handler, a script, background work — use the global
handle:

```rust
if let Some(fonts) = baseui::text::fonts() {
    let w = fonts.width("Ready", 12.0, FontId::Ui);
}
```

It is the same object; `App` publishes it at startup. It returns `None` before the
app starts.

## Sizes are logical, and already scaled

All sizes and results are **logical pixels**. The [global text scale] is folded in
for you: ask for 14.0 at 125% zoom and you get the metrics of a 17.5px font. You
never multiply by the scale yourself, and you never touch DPI — the renderer
handles physical pixels.

[global text scale]: #

## The simple layer

| Call | Answers |
| --- | --- |
| `measure(text, size, font) -> Size` | Bounding box; honors `\n`. The usual choice in `layout`. |
| `width(text, size, font) -> f32` | Width of one line, without building a `Size`. |
| `line_height(size, font) -> f32` | Top of this line to top of the next. |
| `ascent(size, font) -> f32` | Top of the line box down to the baseline. |
| `char_advance(ch, size, font) -> f32` | How far one character moves the pen. |

`font` is a `FontId`: `Ui`, `Mono`, or `Icon(n)`.

## Vertical metrics

`metrics(size, font)` returns a `LineMetrics` when you need more than a height —
placing an underline, aligning mixed sizes, drawing a box around text:

```text
         ┌─────────────────────────  top of the line box
 ascent  │   ██  ██
         │   ██████   ← baseline, `ascent` below the top
 descent │     ██
line_gap │
         └─────────────────────────  top of the next line, `height` below
```

`descent` is reported as a **positive depth** below the baseline (`ab_glyph`
reports it negative; a sign error there is an easy way to draw an underline in the
wrong place, so the API picks one convention and states it).

`height == ascent + descent + line_gap == line_height(..)`.

## The advanced layer: `Line`

For anything interactive, lay the line out once and ask it questions.
`fonts.layout_line(text, size, font)` returns a [`Line`] — the cumulative x offset
of every character boundary. `n` characters means `n + 1` offsets, and the last is
the width.

```rust
let line = cx.fonts.layout_line(text, 13.0, FontId::Mono);

let col = line.col_at(mouse_x);      // click -> caret column
let x = line.x_of(col);              // column -> caret x
let (x0, x1) = line.span(sel_a, sel_b);  // selection / squiggle / run background
let hovered = line.char_at(mouse_x); // which glyph is under the cursor (or None)
```

| Method | Use it for |
| --- | --- |
| `x_of(col)` | Where to **draw** a caret. Clamped — safe past the end. |
| `col_at(x)` | Where a **click** puts the caret: the *nearest* boundary, so the right half of a glyph selects after it. |
| `char_at(x)` | Which character is **under** the pointer — hover, tooltips, per-glyph hit targets. `None` past either end. |
| `span(a, b)` | The x extent of characters `a..b`: selection rects, squiggles, run backgrounds. |
| `width()`, `len()`, `offsets()` | The raw numbers. |

`col_at` and `char_at` are genuinely different questions, which is why both exist:
a caret goes *between* characters, a tooltip is *on* one.

This is what `TextBox`, `TextArea`, and `HexView` all use — they have no
prefix-sum code of their own. If you are writing a widget that puts a caret in
text, you should not either.

## Fitting text into a box

```rust
let label = fonts.truncate("a long column header", 12.0, FontId::Ui, 80.0);
// -> "a long col…"   guaranteed to fit 80px, ellipsis included

let lines = fonts.wrap(paragraph, 13.0, FontId::Ui, 240.0);
// -> greedy word wrap; hard newlines honored; an over-long word is broken
```

`wrap` is layout, not shaping: no reordering, no per-span fonts. For a paragraph
of *styled*, wrapped rich text, see the galley engine planned in
[`rich-text.md`](rich-text.md). For plain wrapped labels and tooltips, this is the
whole job.

## From Lua

`baseui.text` mirrors the Rust API. The font is a string — `"ui"` (default),
`"mono"`, or `"icon:N"` — and an unknown name is an **error**, not a silent
fallback, because measuring in the wrong font produces layout that is subtly
rather than obviously wrong.

```lua
local m  = baseui.text.measure("Hello", 14.0)          -- {width=, height=}
local w  = baseui.text.width("Hello", 14.0, "mono")
local vm = baseui.text.metrics(14.0)                   -- {ascent=, descent=, line_gap=, height=}
local a  = baseui.text.char_advance("W", 14.0)

-- Caret math. Columns are 1-based on the Lua side, so they pair with string.sub.
local x   = baseui.text.x_of("hello world", 7, 14.0)
local col = baseui.text.col_at("hello world", x, 14.0)   -- 7

local cut   = baseui.text.truncate("a long piece of text", 60.0, 14.0)
local lines = baseui.text.wrap(paragraph, 200.0, 14.0)   -- list of strings

baseui.text.set_scale(1.25)
local s = baseui.text.scale()
```

Measurement needs the app's fonts, so call it from a command or an event handler —
not at script top level, before the app has started. (It raises a clear error if
you do.)

## Global text scale

`baseui.text.set_scale(1.25)` / `text::set_scale(1.25)` multiplies every font size
everywhere, in measurement *and* rasterization. Because widgets size themselves
from measured text, this grows rows, buttons, fields, and headers with it; `App`
scales the theme's spacing and radii to match. Clamped to `MIN_SCALE..=MAX_SCALE`
(0.5–3.0).

You do not need to react to it. It is applied inside `Fonts`, so code that
measures is already correct.
