# Docking — plan and architecture

Goal: main-area tabs that hold user-provided widgets, can be **drag-reordered**,
**detached into a floating window**, and **redocked** — the multi-monitor story
for complex engineering apps. (SOW: `baseui-dock` — drag, dock, undock, float,
tab groups, persistent layouts.)

## The decision that makes it tractable

**Separate layout from content.**

```rust
// Layout tree holds only IDs — cheap to move, reorder, split, and serialize.
enum DockNode {
    Split { axis: Axis, children: Vec<DockNode>, sizes: Vec<f32> },
    Tabs  { panels: Vec<PanelId>, active: usize },
}

// Content lives in a flat registry, keyed by id.
struct Panel { id: PanelId, title: String, icon: Option<Icon>, dirty: bool,
               widget: Box<dyn Widget> }
```

Moving a tab — reorder, split, or detach to another window — is then just
**editing an id tree**. No re-parenting a `Box<dyn Widget>` out of one owner and
into another, which is exactly where naive docking implementations turn into an
ownership fight.

Persistence falls out for free: serialize the id tree + window geometries, and
rebuild panels through a factory `fn(&PanelId) -> Option<Panel>` (content widgets
can't be serialized, so the app reconstructs them from the id).

## Consolidation: document tabs *are* dock tab-groups

`docs/document-tabs.md` describes a VS Code-style tab strip (closable,
reorderable, overflow, dirty dot, context menu). A dock **tab group** is the same
widget. Build the strip **once**, inside the dock work — don't implement it twice.

Note this is distinct from the existing [`TabView`], which stays what it is: a
*fixed* set of panel tabs (e.g. the inspector's Object/Material rail). Different
widget, different job.

## Phases

| Phase | Work | Status |
| --- | --- | --- |
| **0** | **Multi-window `App`** — split `Renderer` into a shared `GpuContext` (device, queue, quad pipeline, glyph atlas) and a per-window `WindowRenderer` (surface, config, size, DPI). `window::open(WindowSpec)` request queue. Reactive change repaints **all** windows. | ✅ **done** |
| **1** | Reusable `Popup` (extracted from `MenuBar`, with flip/clamp when there's no room) + right-click routing (`PointerButton::Secondary`) | ✅ **done** |
| **2** | Dock model + `DockArea` in a single window: id tree, tab groups, drag-reorder, drag-to-split with drop indicators, persistence | DONE |
| **3** | **Detach → floating window**, redock, and per-window command contexts | DONE |
| **4** | **Live cross-window drag**: the torn-off window follows the cursor; other windows show drop indicators | DONE |

## Notes from Phase 0

- **Windows cannot be created from application code directly** — that code runs
  deep inside an event handler (a command, a button) with no access to the
  `ActiveEventLoop`. So `window::open` **queues** a request, drained in
  `about_to_wait`. A dock tear-off will use exactly this path.
- **Mixed-DPI multi-monitor already works.** The glyph cache keys on the
  rasterized pixel size (`size × text_scale × dpi_scale`), so a window dragged to
  a different-DPI monitor just produces additional atlas entries. No special
  handling.
- **Windows are rendered one at a time, each with its own submit**, so sharing a
  single instance buffer across windows is safe: transfers and draws execute in
  submission order.
- **Known gap:** `popup::is_open()` is a single global, so a popup in window A
  suppresses shortcuts in window B. Fine today (popups are transient); make it
  per-window when it starts to matter.
- **Known gap:** persistence covers the *main* window's tree and geometry.
  Multi-window layout persistence arrives with the dock id-tree (Phase 2/3).

## Notes from Phase 1

`PopupMenu` is now the single popup implementation, shared by the menu bar, combo
boxes, and right-click context menus. It owns the four things each of them
previously got subtly different: on-screen **placement** (`popup::place` — below,
flip above, clamp), **overlay painting**, **event consumption**, and **keyboard
modality**. A context menu is just a popup anchored to a zero-size rect at the
click point.

Dock tabs will reuse it directly for their right-click menu (Close / Close Others
/ Detach) and for the overflow chevron.

## Notes from Phase 2

The id-tree bet paid off. Every mutation is a tree edit:

- **reorder** within a strip: a vector move,
- **regroup**: `remove_tab` + `insert_tab`,
- **split**: `split_with` wraps the target node in a `Split`,
- **close**: `retain_panels` + `prune`.

**Mutation order matters.** Removing a tab can prune empty groups and collapse
single-child splits, which invalidates paths. So a drop does: *remove without
pruning* (paths stay valid) -> *insert/split at the target path* -> *normalize the
whole tree once*. Getting this backwards is the classic docking bug.

The recursive tree is flattened each layout into `GroupLayout`s (strip + content
rect) and gutters; paint/event work on those flat lists and apply mutations back
by path. Only the **active** panel of each group is laid out, painted, and sent
events, so idle tabs cost nothing.

Robustness: a persisted layout naming panels the app no longer registers is
repaired (`drop_unknown`), and a panel registered but absent from the layout is
adopted into the first group (`adopt_orphans`) rather than silently vanishing.

## Notes from Phase 3

Detaching a panel **moves owned content**: `take_panel` pulls the `Panel` out of
the registry (and its id out of the tree), and hands it to a new window whose
root is a `FloatingPanel`. Redocking pushes the same owned `Panel` onto a queue;
the `DockArea` absorbs it on its next layout. No widget is ever shared,
duplicated, or re-parented behind anyone's back — the direct payoff of splitting
layout from content.

### Per-window command palettes

A detached window offers *different* commands than the main window. The right
mechanism is **not** separate palette instances — it is **command contexts**, so
the registry stays the single source of truth (a Rust command, a Lua command, and
a menu item are all the same thing):

- `CommandMeta::context("panel")` scopes a command.
- `WindowSpec::context("panel")` declares what a window activates while focused.
- The palette lists global commands **plus** the focused window's context; a
  context-scoped **shortcut only fires in that context** (the SOW's
  "context-sensitive shortcuts").

So a floating panel window offers *Dock Panel Back* (Ctrl+D) and *Close This
Panel Window*, which simply don't exist in the main window's palette. Commands
that act on "this window" use `window::focused()`.

## Notes from Phase 4 — live cross-window drag

The thing that makes this tractable is a platform detail worth writing down:

> **While a mouse button is held, the window that received the press keeps an
> implicit pointer grab.** It goes on receiving `CursorMoved` even after the
> cursor leaves it — with out-of-bounds (possibly negative) coordinates.

So there is no need to capture the pointer or poll a global cursor API. The
window that started the drag is the **driver** for the whole gesture, and its
local coordinates still describe where the cursor is; `window::to_screen`
converts them to screen space using the window's client-area origin and DPI.

**The gesture:**

1. **Tear-off.** Dragging a tab out of the `DockArea` bounds calls `take_panel`
   and opens a floating **carrier** window under the cursor. The dock keeps the
   pointer grab, so it keeps driving: each move it updates the session's global
   cursor and calls `window::set_position` on the carrier, which follows.
2. **Claim.** Every *other* `DockArea` reads the session while painting. If the
   global cursor is over it, it converts back to its own local coordinates
   (`window::from_screen`), computes a drop target, **claims** it, and draws the
   indicator. Repaints happen because each move marks the UI dirty.
3. **Release.** The driver marks the session finished. The carrier sees this at
   its next layout: if a claim exists it hands the owned `Panel` over *with the
   claim*, and closes; the claiming `DockArea` reproduces exactly the placement
   the indicator promised (`apply_placement`). With no claim, the window simply
   stays where it was let go.

Dragging a floating window's **header** runs the same machinery in reverse
(carrier == driver), so a detached panel can be dragged back into the dock and
dropped on an indicator.

Two details worth keeping:

- The claim is computed against the tree **after** tear-off pruning, so its path
  is never stale by the time it is applied.
- Moving the carrier under the cursor converges rather than running away, because
  the window is positioned so the *grabbed point* stays under the pointer — after
  the move the cursor reports ~the same local position again.

### Docking back: remember where it came from

Docking a panel back **without** an explicit drop target used to dump it in
whatever group happened to be first. `Panel` now records a `Home` when it is
pulled out — and the anchor is the **sibling panel ids, not a tree path**, because
pulling the panel out can prune its group's parent and collapse the split,
invalidating any path. "The group that still contains Viewport" survives that;
`[1, 0]` does not.

Resolution order when a panel comes back:

1. an explicit **claim** (it was dropped on an indicator) — always wins,
2. its **home** (the group it left, at the index it had),
3. failing both, `adopt_orphans` puts it in the first group.

Either way the tab **flashes** for ~0.7s, so it is obvious where it landed — which
matters most in exactly the case where the user did *not* choose the target.

### Not verified interactively

The drag gesture cannot be exercised in this environment (no pointer injection),
so the live feel — tracking latency, indicator flicker, multi-monitor edges — is
untested by me. The pure logic (tree mutation, placement, ownership transfer) is
unit-tested, and the app runs clean.
