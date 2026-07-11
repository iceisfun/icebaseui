# Document tabs (VS Code style) — design plan

> Status: **planned.** Not implemented. The existing [`TabView`] handles *panel*
> tabs (a fixed set of views, e.g. Object/Material in the inspector). Document
> tabs are a different widget: a dynamic, user-managed set of open documents
> across the top of the **main content area**.

## Why a separate widget

| | `TabView` (exists) | `DocumentTabs` (planned) |
|---|---|---|
| Tab set | fixed, defined at build time | dynamic — opened/closed at runtime |
| Close | no | yes (× per tab, middle-click) |
| Reorder | no | drag to reorder |
| Overflow | tabs always fit | scroll / chevron menu when they don't |
| State | selected index | open list + active + dirty flags + per-doc view state |
| Persistence | active index | the whole open-document list |

Trying to grow `TabView` into this would muddy both. Keep `TabView` simple;
add `DocumentTabs` beside it.

## Model

```rust
pub struct Document {
    pub id: String,          // stable key (e.g. a file path) — used for persistence
    pub title: String,       // display name ("main.rs")
    pub tooltip: String,     // full path
    pub icon: Option<Icon>,  // per-type icon (font-gis / future packs)
    pub dirty: bool,         // unsaved-changes dot
    pub pinned: bool,        // pinned tabs sort first and don't auto-close
    pub content: Box<dyn Widget>,
}

pub struct DocumentTabs {
    docs: Vec<Document>,
    active: usize,
    // interaction state: hovered, close-hovered, drag (index + offset), scroll_x
}
```

API: `open(Document)` (focuses if the id is already open), `close(id)`,
`activate(id)`, `set_dirty(id, bool)`, `on_close(fn(&str) -> bool)` so the app can
veto/prompt on unsaved changes.

## Behavior

- **Tab strip**: icon + title + dirty dot (● replaces × until hovered) + close ×.
- **Middle-click** closes; **double-click empty strip** opens a new document.
- **Drag to reorder** — reuse the pointer-capture pattern from `Split`'s gutter
  drag (press → record index, move → reorder, release → commit).
- **Overflow** — when total tab width exceeds the strip: horizontal scroll on
  wheel plus a chevron button opening a dropdown of all open docs, rendered in
  the **Scene overlay layer** (same mechanism as `MenuBar`/`ComboBox`).
- **Context menu** (right-click): Close, Close Others, Close to the Right,
  Close Saved, Pin. Needs a general context-menu popup — factor the dropdown
  out of `MenuBar` into a reusable `Popup` so all three (menu, combo, context)
  share it.
- **Keyboard**: Ctrl+W close, Ctrl+Tab / Ctrl+Shift+Tab cycle, Ctrl+1..9 jump —
  registered as **commands** so they show up in the Command Palette for free.

## Integration

The content pane of the app frame becomes:

```
Split::vertical().gutter(0.0)
    .fixed_range(tab_h, tab_h, tab_h, DocumentTabs::new(...))
    .flex(/* active document's content */)
```

Same shape as the outliner's search-box-over-tree pane today. Because only the
active document's content is laid out/painted/evented (as `TabView` already
does), having many documents open is cheap.

## Persistence

`persist_save` writes the open-document ids + active id + per-doc scroll/caret
state; `persist_restore` asks the app to re-open them via a
`fn(&str) -> Option<Document>` factory (content widgets can't be serialized, so
the app rebuilds them from the id).

## Prerequisites

1. **Reusable `Popup`** extracted from `MenuBar` (overflow + context menus).
2. **Context-menu (right-click) routing** — `PointerButton::Secondary` already
   exists in `InputEvent`; nothing consumes it yet.
3. Optional: **split editors** (two document groups side by side) — falls out of
   nesting `DocumentTabs` inside the existing `Split`.
