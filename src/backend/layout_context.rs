use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use crate::backend::render_storage::RenderStorage;

pub struct LayoutContext {
    render_storage: Rc<RefCell<RenderStorage>>,
}

impl LayoutContext {
    pub fn new(render_storage: Rc<RefCell<RenderStorage>>) -> Self {
        LayoutContext {
            render_storage: render_storage,
        }
    }

    pub fn storage(&self) -> Ref<RenderStorage> {
        self.render_storage.borrow()
    }

    pub fn storage_mut(&self) -> RefMut<RenderStorage> {
        self.render_storage.borrow_mut()
    }
}
