use std::fmt::Display;
use crate::view::{View, view_id::ViewId};

pub fn label<S: Display + 'static>(label: impl Fn() -> S + 'static) -> Label {
    Label {
        id: ViewId::new(),
        value: label().to_string(),
    }
}

pub fn text<S: Display>(label: S) -> Label {
    Label {
        id: ViewId::new(),
        value: label.to_string(),
    }
}

pub struct Label {
    value: String,
    id: ViewId,
}

impl View for Label {
    fn id(&self) -> ViewId {
        self.id
    }

    fn print(&self) {
        print!("{:}", self.value);
    }
}