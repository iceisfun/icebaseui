//! A small, single-threaded reactive runtime: signals, memos, and effects.
//!
//! This is the backbone of BaseUI's *retained + reactive* architecture. State
//! lives in [`Signal`]s. Reading a signal from inside a reactive computation
//! (an [`effect`](create_effect) or [`memo`](create_memo)) records a
//! dependency; writing the signal later re-runs exactly the computations that
//! depend on it.
//!
//! The runtime is intentionally simple and glitch-tolerant: updates propagate
//! synchronously and depth-first. It is designed for a single UI thread and
//! stored in thread-local storage, so [`Signal`], [`Memo`], and their handles
//! are **not** `Send`/`Sync`.
//!
//! ```
//! use baseui_core::reactive::{create_signal, create_effect, create_memo};
//! use std::cell::Cell;
//! use std::rc::Rc;
//!
//! let count = create_signal(0i32);
//! let doubled = create_memo(move || count.get() * 2);
//!
//! let seen = Rc::new(Cell::new(0));
//! let seen2 = seen.clone();
//! create_effect(move || seen2.set(doubled.get()));
//! assert_eq!(seen.get(), 0);
//!
//! count.set(5);
//! assert_eq!(doubled.get(), 10);
//! assert_eq!(seen.get(), 10); // effect re-ran automatically
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;

/// Index of a node (signal, memo, or effect) in the runtime's arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct NodeId(usize);

type Computation = Rc<dyn Fn()>;

struct Node {
    /// Stored value for signals and memos; `None` for pure effects.
    value: Option<Box<dyn Any>>,
    /// Nodes whose computations read this node and must be re-run when it
    /// changes.
    subscribers: HashSet<NodeId>,
    /// Nodes this node's computation read during its last run. Cleared and
    /// rebuilt on every run so stale dependencies do not linger.
    sources: HashSet<NodeId>,
    /// The reactive computation (effect body / memo recompute), if any.
    computation: Option<Computation>,
}

impl Node {
    fn empty() -> Self {
        Node {
            value: None,
            subscribers: HashSet::new(),
            sources: HashSet::new(),
            computation: None,
        }
    }
}

struct Runtime {
    nodes: Vec<Node>,
    /// Stack of currently-running reactive computations. The top of the stack
    /// is the node that should record dependencies on any signal it reads.
    observers: Vec<NodeId>,
}

impl Runtime {
    fn new() -> Self {
        Runtime {
            nodes: Vec::new(),
            observers: Vec::new(),
        }
    }

    fn create_node(&mut self) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node::empty());
        id
    }

    /// Record that the current observer (if any) depends on `source`.
    fn track(&mut self, source: NodeId) {
        if let Some(&observer) = self.observers.last() {
            self.nodes[source.0].subscribers.insert(observer);
            self.nodes[observer.0].sources.insert(source);
        }
    }

    /// Detach `node` from all of its current sources, so a re-run can rebuild
    /// the dependency set cleanly.
    fn clear_sources(&mut self, node: NodeId) {
        let sources = std::mem::take(&mut self.nodes[node.0].sources);
        for source in sources {
            self.nodes[source.0].subscribers.remove(&node);
        }
    }
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::new());
    /// Optional hook invoked after any signal mutation. The UI layer registers
    /// this to request a repaint whenever reactive state changes — the bridge
    /// that makes the retained widget tree reactive.
    static ON_CHANGE: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

fn with_runtime<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    RUNTIME.with(|rt| f(&mut rt.borrow_mut()))
}

/// Register a callback fired after every [`Signal::set`]/[`Signal::update`].
///
/// Typically the application registers something like
/// `move || window.request_redraw()` so that mutating any signal — from an
/// event handler, a timer, or async work — schedules a repaint. Replaces any
/// previously registered hook.
pub fn set_on_change(f: impl Fn() + 'static) {
    ON_CHANGE.with(|slot| *slot.borrow_mut() = Some(Box::new(f)));
}

/// Invoke the change hook, if one is registered. Called after signal writes.
/// The hook is temporarily moved out so it may itself touch signals without a
/// borrow conflict.
fn mark_dirty() {
    let hook = ON_CHANGE.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
        ON_CHANGE.with(|slot| {
            let mut slot = slot.borrow_mut();
            // Only restore if no new hook was registered during the call.
            if slot.is_none() {
                *slot = Some(hook);
            }
        });
    }
}

/// Run `computation` for `node`, tracking the signals it reads as `node`'s
/// sources. Any previously-recorded sources are cleared first.
fn run_computation(node: NodeId) {
    // Fetch the computation without holding the runtime borrow while it runs;
    // the computation itself will re-enter the runtime to read/write signals.
    let computation = with_runtime(|rt| {
        rt.clear_sources(node);
        rt.observers.push(node);
        rt.nodes[node.0].computation.clone()
    });

    if let Some(computation) = computation {
        computation();
    }

    with_runtime(|rt| {
        debug_assert_eq!(rt.observers.last().copied(), Some(node));
        rt.observers.pop();
    });
}

/// Notify every subscriber of `node` that it must re-run. Subscribers are
/// snapshotted before any run so re-subscription during the run is safe.
fn notify_subscribers(node: NodeId) {
    let subscribers: Vec<NodeId> =
        with_runtime(|rt| rt.nodes[node.0].subscribers.iter().copied().collect());
    for sub in subscribers {
        run_computation(sub);
    }
}

// ---------------------------------------------------------------------------
// Signal
// ---------------------------------------------------------------------------

/// A reactive container for a single value.
///
/// Cheap (`Copy`) handle into the runtime. Reading with [`Signal::get`] or
/// [`Signal::with`] inside an effect or memo subscribes that computation;
/// [`Signal::set`] / [`Signal::update`] re-runs the subscribers.
pub struct Signal<T: 'static> {
    node: NodeId,
    _ty: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Signal<T> {}

/// Create a new signal holding `value`.
pub fn create_signal<T: 'static>(value: T) -> Signal<T> {
    let node = with_runtime(|rt| {
        let id = rt.create_node();
        rt.nodes[id.0].value = Some(Box::new(value));
        id
    });
    Signal {
        node,
        _ty: PhantomData,
    }
}

impl<T: 'static> Signal<T> {
    /// Read the value by reference, tracking a dependency. Returns whatever the
    /// closure returns, avoiding a clone when you only need to inspect the
    /// value.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        with_runtime(|rt| rt.track(self.node));
        with_runtime(|rt| {
            let value = rt.nodes[self.node.0]
                .value
                .as_ref()
                .expect("signal always has a value");
            let value = value.downcast_ref::<T>().expect("signal type mismatch");
            f(value)
        })
    }

    /// Replace the value and re-run subscribers.
    pub fn set(&self, value: T) {
        with_runtime(|rt| {
            rt.nodes[self.node.0].value = Some(Box::new(value));
        });
        notify_subscribers(self.node);
        mark_dirty();
    }

    /// Mutate the value in place and re-run subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        with_runtime(|rt| {
            let value = rt.nodes[self.node.0]
                .value
                .as_mut()
                .expect("signal always has a value");
            let value = value.downcast_mut::<T>().expect("signal type mismatch");
            f(value);
        });
        notify_subscribers(self.node);
        mark_dirty();
    }
}

impl<T: Clone + 'static> Signal<T> {
    /// Read a clone of the value, tracking a dependency.
    pub fn get(&self) -> T {
        self.with(|v| v.clone())
    }
}

// ---------------------------------------------------------------------------
// Memo
// ---------------------------------------------------------------------------

/// A cached, derived value that recomputes when its dependencies change.
///
/// Reading a memo tracks a dependency just like reading a signal, so memos can
/// be composed and read from effects.
pub struct Memo<T: 'static> {
    signal: Signal<T>,
}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Memo<T> {}

/// Create a memo whose value is produced by `compute`. `compute` runs
/// immediately to establish the initial value and its dependencies, then again
/// whenever any dependency changes.
pub fn create_memo<T: 'static>(compute: impl Fn() -> T + 'static) -> Memo<T> {
    // Seed a signal by running the computation once outside any tracking so we
    // have a concrete initial value, then wire an effect that writes future
    // recomputations back into the signal.
    let signal = create_signal(compute());
    create_effect(move || {
        let value = compute();
        signal.set(value);
    });
    Memo { signal }
}

impl<T: 'static> Memo<T> {
    /// Read the memo by reference, tracking a dependency.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }
}

impl<T: Clone + 'static> Memo<T> {
    /// Read a clone of the memo's current value, tracking a dependency.
    pub fn get(&self) -> T {
        self.signal.get()
    }
}

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

/// Register a side effect that runs immediately and re-runs whenever any signal
/// or memo it read changes.
///
/// The effect is retained by the runtime for the life of the process (BaseUI
/// does not yet expose scoped disposal); do not create unbounded numbers of
/// effects in a hot loop.
pub fn create_effect(effect: impl Fn() + 'static) {
    let node = with_runtime(|rt| {
        let id = rt.create_node();
        rt.nodes[id.0].computation = Some(Rc::new(effect));
        id
    });
    run_computation(node);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn signal_get_set() {
        let s = create_signal(1);
        assert_eq!(s.get(), 1);
        s.set(42);
        assert_eq!(s.get(), 42);
        s.update(|v| *v += 1);
        assert_eq!(s.get(), 43);
    }

    #[test]
    fn effect_runs_on_change() {
        let s = create_signal(0);
        let runs = Rc::new(Cell::new(0));
        let last = Rc::new(Cell::new(-1));
        let (r2, l2) = (runs.clone(), last.clone());
        create_effect(move || {
            r2.set(r2.get() + 1);
            l2.set(s.get());
        });
        assert_eq!(runs.get(), 1);
        assert_eq!(last.get(), 0);

        s.set(7);
        assert_eq!(runs.get(), 2);
        assert_eq!(last.get(), 7);
    }

    #[test]
    fn memo_recomputes() {
        let a = create_signal(2);
        let b = create_signal(3);
        let sum = create_memo(move || a.get() + b.get());
        assert_eq!(sum.get(), 5);
        a.set(10);
        assert_eq!(sum.get(), 13);
        b.set(0);
        assert_eq!(sum.get(), 10);
    }

    #[test]
    fn dynamic_dependencies_are_dropped() {
        // When `toggle` is false the effect must not depend on `b`.
        let toggle = create_signal(true);
        let a = create_signal(1);
        let b = create_signal(100);
        let runs = Rc::new(Cell::new(0));
        let r2 = runs.clone();
        create_effect(move || {
            r2.set(r2.get() + 1);
            if toggle.get() { a.get() } else { b.get() };
        });
        assert_eq!(runs.get(), 1);

        // Depends on `a` currently, so changing `a` re-runs.
        a.set(2);
        assert_eq!(runs.get(), 2);

        // Switch to depending on `b`.
        toggle.set(false);
        assert_eq!(runs.get(), 3);

        // Now changing `a` must NOT re-run, but changing `b` must.
        a.set(3);
        assert_eq!(runs.get(), 3);
        b.set(200);
        assert_eq!(runs.get(), 4);
    }
}
