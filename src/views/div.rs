use slotmap::Key;

use crate::view::{View, view_id::ViewId, view_iter::IntoViewIter};

pub fn div(children: impl IntoViewIter) -> Div {
    Div::new(children)
}

pub struct Div {
    id: ViewId,
}

impl Div {
    pub fn new(children: impl IntoViewIter) -> Self {
        let id = ViewId::new();
		id.set_children_iter(children.into_view_iter());
        Div {
            id,
        }
    }
}

impl View for Div {
    fn id(&self) -> ViewId {
        self.id
    }

    fn print(&self) {
		let children = self.id.children();
		print!("\n");
		println!("{:?}", self.id);
		print!("    ");
		for child in children {
			print!("'");
			child.view().borrow().print();
			print!("' ");
		}
		print!("\n");
    }
}
