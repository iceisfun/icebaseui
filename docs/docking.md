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
| **2** | Dock model + `DockArea` in a single window: id tree, tab groups, drag-reorder, drag-to-split with drop indicators, persistence | todo |
| **3** | **Detach → floating window** (tear-off creates the window immediately), redock via indicators | todo |
| **4** | **Live cross-window drag**: the torn-off window follows the cursor; other windows show drop indicators | todo |

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

## Phase 4 is the hard 20%

Live cross-window drag needs pointer capture across windows, a global cursor
position, and hit-testing another window's drop zones — the platform-specific
part. Phases 0–3 deliver the entire multi-monitor workflow via tear-off +
indicator redock; Phase 4 is feel, not capability.
