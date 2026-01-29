use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use slotmap::new_key_type;

use crate::view::{IntoView, View, state::ViewState, storage::VIEW_STORAGE};

new_key_type! {
    pub struct ViewId;
}

impl ViewId {
    pub fn new() -> Self {
        VIEW_STORAGE.with_borrow_mut(|s| s.view_ids.insert(()))
    }

    pub fn state(&self) -> Rc<RefCell<ViewState>> {
        VIEW_STORAGE.with_borrow_mut(|s| {
            s.states
                .entry(*self)
                .unwrap()
                .or_insert_with(|| Rc::new(RefCell::new(ViewState::new(*self))))
                .clone()
        })
    }

    pub fn view(&self) -> Rc<RefCell<Box<dyn View>>> {
        VIEW_STORAGE.with_borrow(|s| {
            s.views
                .get(*self)
                .cloned()
                .unwrap_or_else(|| s.stale_view.clone())
        })
    }

    pub fn set_parent(&self, parent: ViewId) {
        VIEW_STORAGE.with_borrow_mut(|s| {
            if s.view_ids.contains_key(*self) {
                s.parent.insert(*self, Some(parent));
            }
        });
    }

    pub fn append_children(&self, children: Vec<Box<dyn View>>) {
        VIEW_STORAGE.with_borrow_mut(|s| {
            let children_list = s.children.entry(*self).unwrap().or_default();
            for child in children {
                let child_id = child.id();
                children_list.push(child_id);
                s.parent.insert(child_id, Some(*self));
                s.views.insert(child_id, Rc::new(RefCell::new(child)));
            }
        });
    }

	pub fn set_children_vec(&self, children: Vec<impl IntoView>) {
        self.set_children_iter(children.into_iter().map(|c| c.into_any()));
    }

	pub fn set_children_iter(&self, children: impl Iterator<Item = Box<dyn View>>) {
       	VIEW_STORAGE.with_borrow_mut(|s| {
            let mut children_ids = Vec::new();
            for child_view in children {
                let child_view_id = child_view.id();
                children_ids.push(child_view_id);
                s.parent.insert(child_view_id, Some(*self));
                s.views
                    .insert(child_view_id, Rc::new(RefCell::new(child_view)));
            }
            s.children.insert(*self, children_ids.clone());
        });
    }

	pub fn children(&self) -> Vec<ViewId> {
        VIEW_STORAGE.with_borrow(|s| s.children.get(*self).cloned().unwrap_or_default())
    }
}
