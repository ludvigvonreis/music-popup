//! The bridge between the reactive runtime and the view tree.
//!
//! Effects never touch views directly. They call `push_update`, which queues a
//! message and marks the frame dirty. `apply_updates` drains the queue at a
//! known point in the frame (before layout), so a signal `set` can never
//! re-enter layout or paint.

use std::{
    any::Any,
    cell::{Cell, RefCell},
};

use crate::{style::Style, view::view_id::ViewId};

pub const DIRTY_LAYOUT: u8 = 0b01;
pub const DIRTY_PAINT: u8 = 0b10;

/// Sent to a view when its resolved style was replaced.
pub struct StyleChanged;

/// Internal: carries a freshly resolved style to `apply_updates`.
pub struct SetStyle(pub Style);

thread_local! {
    static UPDATES: RefCell<Vec<(ViewId, Box<dyn Any>)>> = RefCell::new(Vec::new());
    // Start dirty so the very first frame is drawn.
    static DIRTY: Cell<u8> = Cell::new(DIRTY_LAYOUT | DIRTY_PAINT);
}

pub fn mark_dirty(flags: u8) {
    DIRTY.with(|d| d.set(d.get() | flags));
}

pub fn is_dirty() -> bool {
    DIRTY.with(|d| d.get() != 0)
}

/// Returns the pending flags and clears them.
pub fn take_dirty() -> u8 {
    DIRTY.with(|d| d.replace(0))
}

/// Queue a message for `id`. Safe to call from inside effects.
pub fn push_update(id: ViewId, msg: Box<dyn Any>) {
    UPDATES.with_borrow_mut(|u| u.push((id, msg)));
    // Text and style changes can change size, so assume both for now.
    mark_dirty(DIRTY_LAYOUT | DIRTY_PAINT);
}

/// Drain the queue and deliver messages. Call once per frame before layout.
pub fn apply_updates() {
    loop {
        let batch = UPDATES.with_borrow_mut(std::mem::take);
        if batch.is_empty() {
            break;
        }
        for (id, msg) in batch {
            let msg: Box<dyn Any> = match msg.downcast::<SetStyle>() {
                Ok(set) => {
                    id.state().borrow_mut().style = set.0;
                    Box::new(StyleChanged)
                }
                Err(other) => other,
            };
            // Views removed from the tree resolve to the stale view, whose
            // default `update` is a no-op.
            id.view().borrow_mut().update(msg);
        }
    }
}
