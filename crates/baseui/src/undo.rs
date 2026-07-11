//! Undo/redo — the [`History`] seam and a built-in [`UndoStack`].
//!
//! Editing widgets do not own an undo implementation; they own a
//! `Option<Box<dyn History>>`. That means undo is **opt-in** (a read-only log
//! view has no use for it) and **replaceable**:
//!
//! - [`TextArea::undo_history`](crate::widget::TextArea::undo_history) enables the
//!   built-in [`UndoStack`], which is the right answer for scripts, config files,
//!   and source files;
//! - [`TextArea::history`](crate::widget::TextArea::history) injects *your*
//!   implementation, which is the right answer when the built-in's model does not
//!   fit — see below.
//!
//! # What the built-in costs
//!
//! [`UndoStack`] stores whole-document [`Snapshot`]s. That is simple, obviously
//! correct, and completely fine up to files of a few hundred KB. It is *not* fine
//! for a 50 MB log or a document you are streaming into: every recorded edit
//! copies the whole buffer.
//!
//! An app in that position implements [`History`] over a piece table, a rope, or
//! an operation log, and injects it. The widget does not care: it hands you a
//! before-state and asks for a state back.
//!
//! # Grouping
//!
//! Undo that steps one character at a time is useless, so edits **coalesce**: a
//! run of [`EditKind::Typing`] becomes one entry, as does a run of
//! [`EditKind::Delete`]. A [`EditKind::Break`] (space, tab, newline) closes the
//! group, so undo walks back word by word. Moving the caret also closes it via
//! [`History::close_group`] — otherwise typing, clicking elsewhere, and typing
//! again would undo as a single confusing lump.
//!
//! Grouping is decided by *what was edited*, not by a timer. Same input, same
//! result — which is also what makes it testable.

/// A document state worth restoring: the text plus where the caret and selection
/// were when it was current.
///
/// Restoring the caret matters more than it sounds: an undo that fixes the text
/// but leaves the caret elsewhere makes the user hunt for what just changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The whole document, not a diff — see the module note on what that costs.
    pub text: String,
    /// Caret as (line, column-in-chars).
    pub caret: (usize, usize),
    /// Selection anchor, if a selection was active.
    pub anchor: Option<(usize, usize)>,
}

/// What kind of edit is being recorded. Implementations use this to decide what
/// coalesces with what.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// An ordinary character was typed.
    Typing,
    /// Whitespace — a space, tab, or newline. Ends a word, and so ends a group.
    Break,
    /// Backspace or delete.
    Delete,
    /// A paste, or any other bulk replacement. Never coalesces: a paste is one
    /// user action and must undo as one.
    Paste,
}

/// Undo/redo for an editing widget.
///
/// The widget calls [`record`](History::record) with the state *before* an edit,
/// then [`undo`](History::undo)/[`redo`](History::redo) with the state that is
/// *current* — so an implementation never needs to track the live document, only
/// the states it was handed.
pub trait History {
    /// Record the document state as it was **before** an edit of `kind`.
    ///
    /// This is also where a new edit invalidates the redo branch.
    fn record(&mut self, before: Snapshot, kind: EditKind);

    /// Step back. `current` is the live state, to be made redoable. Returns the
    /// state to restore, or `None` when there is nothing to undo.
    fn undo(&mut self, current: Snapshot) -> Option<Snapshot>;

    /// Step forward again. Returns `None` when there is nothing to redo.
    fn redo(&mut self, current: Snapshot) -> Option<Snapshot>;

    /// End the current coalescing group — the caret moved, focus changed, or the
    /// app hit a save point. The next edit starts a fresh entry.
    fn close_group(&mut self);

    /// Whether [`undo`](History::undo) would return a state — what an Edit menu
    /// greys its entry on.
    fn can_undo(&self) -> bool;

    /// Whether [`redo`](History::redo) would return a state.
    fn can_redo(&self) -> bool;
}

/// How many undo entries the built-in stack keeps by default.
pub const DEFAULT_LIMIT: usize = 256;

/// One recorded step: the state before it, and whether it can still absorb more
/// edits of the same kind.
#[derive(Clone, Debug)]
struct Entry {
    before: Snapshot,
    kind: EditKind,
    /// Still accepting coalescing edits of the same kind.
    open: bool,
}

/// The built-in snapshot-based [`History`].
///
/// Bounded: past `limit` entries the oldest are dropped, so a long editing
/// session cannot grow without limit.
pub struct UndoStack {
    entries: Vec<Entry>,
    redo: Vec<Snapshot>,
    limit: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        UndoStack::with_limit(DEFAULT_LIMIT)
    }
}

impl UndoStack {
    /// An empty stack holding up to [`DEFAULT_LIMIT`] steps.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep at most `limit` undo steps.
    pub fn with_limit(limit: usize) -> Self {
        UndoStack {
            entries: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Whether the newest entry can absorb another edit of `kind`.
    fn absorbs(&self, kind: EditKind) -> bool {
        match self.entries.last() {
            Some(last) => last.open && last.kind == kind,
            None => false,
        }
    }
}

impl History for UndoStack {
    fn record(&mut self, before: Snapshot, kind: EditKind) {
        // Editing after an undo abandons the redo branch — the usual model, and
        // the one users expect.
        self.redo.clear();

        match kind {
            // Coalesce a run: the group's before-state is already recorded, so
            // this keystroke adds nothing new to remember.
            EditKind::Typing | EditKind::Delete if self.absorbs(kind) => {}

            // Whitespace joins the word being typed, then closes it: undo takes
            // back "hello " in one step, not " " and then "hello".
            EditKind::Break if self.absorbs(EditKind::Typing) => {
                if let Some(last) = self.entries.last_mut() {
                    last.open = false;
                }
            }

            _ => {
                self.entries.push(Entry {
                    before,
                    kind,
                    // A paste is one user action; a break is already complete.
                    open: matches!(kind, EditKind::Typing | EditKind::Delete),
                });
                if self.entries.len() > self.limit {
                    self.entries.remove(0);
                }
            }
        }
    }

    fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let entry = self.entries.pop()?;
        self.redo.push(current);
        Some(entry.before)
    }

    fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        // Redoing must be undoable again, and the restored step is complete —
        // pushing it closed stops the next keystroke from merging into it.
        self.entries.push(Entry {
            before: current,
            kind: EditKind::Paste,
            open: false,
        });
        Some(next)
    }

    fn close_group(&mut self) {
        if let Some(last) = self.entries.last_mut() {
            last.open = false;
        }
    }

    fn can_undo(&self) -> bool {
        !self.entries.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(text: &str) -> Snapshot {
        Snapshot {
            text: text.into(),
            caret: (0, text.chars().count()),
            anchor: None,
        }
    }

    /// Typing a word then undoing must take back the *word*, not one letter.
    #[test]
    fn a_run_of_typing_undoes_as_one_step() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing); // -> "h"
        h.record(snap("h"), EditKind::Typing); // -> "hi"
        h.record(snap("hi"), EditKind::Typing); // -> "hit"

        assert_eq!(h.undo(snap("hit")).unwrap(), snap(""));
        assert!(!h.can_undo(), "the three keystrokes were one step");
    }

    /// Whitespace ends a word: undo walks back word by word.
    #[test]
    fn a_break_closes_the_group() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing); // "hello"
        h.record(snap("hello"), EditKind::Break); // "hello "
        h.record(snap("hello "), EditKind::Typing); // "hello world"

        assert_eq!(h.undo(snap("hello world")).unwrap(), snap("hello "));
        assert_eq!(h.undo(snap("hello ")).unwrap(), snap(""));
        assert!(!h.can_undo());
    }

    /// A paste is one user action and never merges with the typing around it.
    #[test]
    fn a_paste_is_its_own_step() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing); // "a"
        h.record(snap("a"), EditKind::Paste); // "a<lots>"
        h.record(snap("a<lots>"), EditKind::Typing); // "a<lots>b"

        assert_eq!(h.undo(snap("a<lots>b")).unwrap(), snap("a<lots>"));
        assert_eq!(h.undo(snap("a<lots>")).unwrap(), snap("a"));
        assert_eq!(h.undo(snap("a")).unwrap(), snap(""));
    }

    /// Moving the caret splits the undo history — otherwise typing here, clicking
    /// there, and typing again would undo as one lump across both places.
    #[test]
    fn moving_the_caret_closes_the_group() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing); // "ab"
        h.close_group();
        h.record(snap("ab"), EditKind::Typing); // "abcd"

        assert_eq!(h.undo(snap("abcd")).unwrap(), snap("ab"));
        assert_eq!(h.undo(snap("ab")).unwrap(), snap(""));
    }

    #[test]
    fn redo_replays_what_undo_took_back() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing);
        h.close_group();
        h.record(snap("one"), EditKind::Typing);

        let after_first_undo = h.undo(snap("one two")).unwrap();
        assert_eq!(after_first_undo, snap("one"));
        assert!(h.can_redo());

        assert_eq!(h.redo(snap("one")).unwrap(), snap("one two"));
        assert!(!h.can_redo());
        // ...and the redone state is undoable again.
        assert_eq!(h.undo(snap("one two")).unwrap(), snap("one"));
    }

    /// The classic trap: undo, then type. The redo branch must be gone, not
    /// resurrectable into a document the user never had.
    #[test]
    fn editing_after_an_undo_discards_the_redo_branch() {
        let mut h = UndoStack::new();
        h.record(snap(""), EditKind::Typing);

        h.undo(snap("original")).unwrap();
        assert!(h.can_redo());

        h.record(snap(""), EditKind::Typing); // a new edit
        assert!(!h.can_redo(), "the redo branch must not survive a new edit");
    }

    /// Undo is bounded: a long session cannot grow the stack without limit.
    #[test]
    fn the_stack_is_bounded_and_drops_the_oldest() {
        let mut h = UndoStack::with_limit(2);
        for i in 0..5 {
            h.record(snap(&i.to_string()), EditKind::Paste);
        }
        assert_eq!(h.entries.len(), 2);
        // The two most recent survive; the oldest were dropped.
        assert_eq!(h.entries[0].before, snap("3"));
        assert_eq!(h.entries[1].before, snap("4"));
    }

    #[test]
    fn nothing_to_undo_is_not_an_error() {
        let mut h = UndoStack::new();
        assert!(!h.can_undo());
        assert!(h.undo(snap("x")).is_none());
        assert!(h.redo(snap("x")).is_none());
    }
}
