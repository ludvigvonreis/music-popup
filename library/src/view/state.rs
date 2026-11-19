use std::{cell::RefCell, rc::Rc};

use crate::{style::Style, view::view_id::ViewId};

pub type StyleLayer = Rc<RefCell<dyn FnMut(Style) -> Style>>;

pub struct ViewState {
    pub(crate) view_id: ViewId,
    pub(crate) style: Style,
    pub(crate) style_layers: Vec<StyleLayer>,
}

impl ViewState {
    pub fn new(id: ViewId) -> Self {
        Self {
            view_id: id,
            style: Style::new(),
            style_layers: Vec::new(),
        }
    }
}
