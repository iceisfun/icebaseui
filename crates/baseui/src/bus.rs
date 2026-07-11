//! The typed event bus.
//!
//! A lightweight publish/subscribe channel for decoupled communication: panels
//! and plugins react to typed events instead of holding references to each other
//! (SOW: "Panels communicate through events rather than direct references").
//!
//! Events are ordinary `'static` types — define whatever payloads you need:
//!
//! ```
//! use baseui::bus;
//!
//! struct SelectionChanged { path: String }
//!
//! let sub = bus::subscribe::<SelectionChanged>(|e| {
//!     println!("selected {}", e.path);
//! });
//! bus::publish(&SelectionChanged { path: "Cube".into() });
//! drop(sub); // unsubscribes
//! ```
//!
//! The bus is thread-local (single UI thread), so `subscribe`/`publish` work
//! from anywhere without threading a context object through the widget tree.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Identifies one subscription, for [`unsubscribe`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SubscriptionId(u64);

type Callback = Rc<dyn Fn(&dyn Any)>;

#[derive(Default)]
struct Bus {
    subscribers: HashMap<TypeId, Vec<(SubscriptionId, Callback)>>,
    next_id: u64,
}

thread_local! {
    static BUS: RefCell<Bus> = RefCell::new(Bus::default());
}

/// Subscribe to events of type `T`. The returned [`Subscription`] unsubscribes
/// when dropped; call [`Subscription::leak`] to keep it alive for the process.
pub fn subscribe<T: 'static>(handler: impl Fn(&T) + 'static) -> Subscription {
    let callback: Callback = Rc::new(move |any: &dyn Any| {
        if let Some(event) = any.downcast_ref::<T>() {
            handler(event);
        }
    });
    let id = BUS.with(|b| {
        let mut b = b.borrow_mut();
        let id = SubscriptionId(b.next_id);
        b.next_id += 1;
        b.subscribers
            .entry(TypeId::of::<T>())
            .or_default()
            .push((id, callback));
        id
    });
    Subscription { id: Some(id) }
}

/// Publish `event` to every current subscriber of its type. Subscribers are
/// snapshotted first, so a handler may itself publish or (un)subscribe safely.
pub fn publish<T: 'static>(event: &T) {
    let callbacks: Vec<Callback> = BUS.with(|b| {
        b.borrow()
            .subscribers
            .get(&TypeId::of::<T>())
            .map(|subs| subs.iter().map(|(_, cb)| cb.clone()).collect())
            .unwrap_or_default()
    });
    for callback in callbacks {
        callback(event as &dyn Any);
    }
}

/// Remove a subscription by id (usually handled by dropping its [`Subscription`]).
pub fn unsubscribe(id: SubscriptionId) {
    BUS.with(|b| {
        for subs in b.borrow_mut().subscribers.values_mut() {
            subs.retain(|(sub_id, _)| *sub_id != id);
        }
    });
}

/// An RAII handle to a subscription; unsubscribes on drop.
#[must_use = "dropping the Subscription immediately unsubscribes; bind it or call leak()"]
pub struct Subscription {
    id: Option<SubscriptionId>,
}

impl Subscription {
    /// This subscription's id.
    pub fn id(&self) -> Option<SubscriptionId> {
        self.id
    }

    /// Keep the subscription alive for the rest of the process (never auto-drop).
    pub fn leak(mut self) {
        self.id = None;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            unsubscribe(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct Ping(i32);

    #[test]
    fn publish_reaches_subscribers() {
        let total = Rc::new(Cell::new(0));
        let t2 = total.clone();
        let sub = subscribe::<Ping>(move |p| t2.set(t2.get() + p.0));
        publish(&Ping(3));
        publish(&Ping(4));
        assert_eq!(total.get(), 7);

        // Dropping unsubscribes.
        drop(sub);
        publish(&Ping(100));
        assert_eq!(total.get(), 7);
    }

    #[test]
    fn types_are_isolated() {
        struct A;
        struct B;
        let hits = Rc::new(Cell::new(0));
        let h2 = hits.clone();
        let _a = subscribe::<A>(move |_| h2.set(h2.get() + 1));
        publish(&B);
        assert_eq!(hits.get(), 0);
        publish(&A);
        assert_eq!(hits.get(), 1);
        _a.leak();
    }
}
