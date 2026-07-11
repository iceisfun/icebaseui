# Lua scripting — scope and rationale

> Status: **implemented** as `baseui-lua` (optional crate). This document records
> *why* the binding surface is drawn where it is, so the boundary doesn't erode.

## The decision

**Lua is a composition + command/glue layer. It is not a widget-authoring
language.**

- **Rust** writes the app and any custom widgets (`impl Widget`).
- **Lua** registers commands, binds shortcuts, publishes/handles events, and
  contributes status items — i.e. it changes what the app *does*, without
  recompiling it.

`baseui-lua` deliberately **does not expose the `Widget` trait**.

## Why not let scripts implement widgets?

A `Widget` implements `layout` / `paint` / `event`. Exposing that to Lua means:

- crossing the FFI boundary for **every widget, every frame** (layout + paint),
- handing out `&mut Scene` and the whole geometry/paint API,
- Lua errors and panics on the render path,
- borrow/lifetime gymnastics for `&mut self` widgets held by the retained tree.

The payoff is small: nearly every "custom widget" people actually want is a
*composition* of existing widgets, which the declarative layer covers. And the
premise of this framework is that **consumers integrate their own controls in
Rust** — so scripting widget internals would be a second, weaker path to
something Rust already does well.

If real custom drawing is ever needed, the intended escape hatch is a single
`Canvas` widget whose `paint` calls a Lua function with a small immediate API
(`draw.rect`, `draw.text`). One widget, opt-in, with the per-frame cost visible
at the call site — rather than opening the whole trait.

## Why it fits so cleanly

The M7 systems layer was already shaped for this:

| System | Why it binds well |
| --- | --- |
| `command` registry | string-keyed ids + metadata; menus, toolbar, shortcuts, and the palette **all** invoke by id |
| `bus` (named channel) | string-addressed events with JSON payloads |
| `persist` store | string-keyed JSON |
| retained tree | built **once** — Lua runs at construction and at event time, **never in layout/paint** |

That last row is the load-bearing one. Because the tree is retained, a script can
run when a command fires and then get out of the way; reactivity still works
because handlers mutate signals and signals drive the repaint.

The payoff: a Lua-registered command is **indistinguishable from a Rust one** —
same registry, so it appears in the Command Palette, can be bound to a key, and
can be invoked from a menu. One source of truth, no parallel API.

## Two things the framework had to grow

1. **Named events.** The typed bus keys on Rust `TypeId`, which scripts cannot
   name. `bus::publish_named` / `bus::on_named` add a string-addressed channel
   (JSON payload) riding on the same bus — useful to Rust too.
2. **Icon-by-name.** `icon::parse("gis:compass")` / `"glyph:star"`, backed by a
   generated `gis::by_name`, so config and scripts can reference icons.

Plus a status-item registry (`widget::statusbar::contribute`) so plugins can add
status entries, per the SOW.

## API surface

```lua
baseui.commands.register{ id, title, category?, icon?, color?, shortcut?, run }
baseui.commands.run(id)
baseui.shortcuts.bind(chord, command_id)
baseui.bus.on(name, function(payload) end)
baseui.bus.emit(name, table)
baseui.status.add{ text = string | function, icon?, color?, right? }
baseui.log.info/warn/error(msg)

-- Text measurement (see docs/text.md). font is "ui" (default) | "mono" | "icon:N".
baseui.text.measure(s, size, font?)        -- {width=, height=}
baseui.text.width(s, size, font?)          -- single line
baseui.text.metrics(size, font?)           -- {ascent=, descent=, line_gap=, height=}
baseui.text.char_advance(ch, size, font?)
baseui.text.x_of(s, col, size, font?)      -- caret x for a column (1-based)
baseui.text.col_at(s, x, size, font?)      -- column nearest an x  (1-based)
baseui.text.truncate(s, max_w, size, font?)-- fits the budget, adds an ellipsis
baseui.text.wrap(s, max_w, size, font?)    -- list of lines
baseui.text.set_scale(n) ; baseui.text.scale()
```

Errors in a Lua handler are **caught and logged**, never unwound into the
renderer.

**Why scripts get the full measurement API and not just the zoom knob:** a plugin
that contributes a status item or a generated label has to know how wide its text
is, or it can only hard-code pixel widths — which break the moment the user
changes the text scale or the theme's font. These are the *same* numbers the
renderer positions glyphs by, so scripted layout is exactly as correct as Rust
layout. An unknown font name is an error rather than a silent fallback, because
measuring in the wrong font gives layout that is subtly, not obviously, wrong.
Measurement needs the app's loaded fonts, so call it from a command or event
handler, not at script top level.

## Deliberately not done (yet)

- **Declarative panels** (`ui.column{ ui.label(...), ui.textbox{...} }`
  interpreted **once** into a `LuaPanel: Widget`). This is the natural next tier
  and stays within the "Lua composes, Rust implements" rule. It needs a
  dynamic-signal handle registry, because `Signal<T>` is generic and Lua is not:
  `state.number()/text()/bool()` would create the correctly-typed signal Rust-side
  and hand back an opaque handle.
- **Menu contribution** from scripts (needs a menu registry + a defined build
  order: load scripts, then assemble the menu bar). Less urgent, since script
  commands already reach users through the Command Palette and shortcuts.
- **Sandboxing / resource limits** for untrusted scripts.

## Cost

`mlua` vendors and compiles Lua from C. It lives in a **separate optional crate**
so the core stays lean (per the SOW's "large optional systems remain separate
crates"). Applications that don't want scripting simply don't depend on it.
