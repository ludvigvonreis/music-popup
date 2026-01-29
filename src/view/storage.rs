use crate::view::{AnyView, IntoView, state::ViewState, view_id::ViewId};
use slotmap::{SecondaryMap, SlotMap};
use std::{cell::RefCell, rc::Rc};

thread_local! {
    pub static VIEW_STORAGE: RefCell<ViewStorage> = Default::default();
}

pub struct ViewStorage {
    pub(crate) view_ids: SlotMap<ViewId, ()>,
    pub(crate) views: SecondaryMap<ViewId, Rc<RefCell<AnyView>>>,
	pub(crate) children: SecondaryMap<ViewId, Vec<ViewId>>,
	pub(crate) parent: SecondaryMap<ViewId, Option<ViewId>>,

    pub(crate) states: SecondaryMap<ViewId, Rc<RefCell<ViewState>>>,
	pub(crate) stale_view: Rc<RefCell<AnyView>>,
}

impl Default for ViewStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStorage {
    pub fn new() -> Self {
        let view_ids = SlotMap::<ViewId, ()>::default();

        Self {
            view_ids,
            views: Default::default(),
			children: Default::default(),
			parent: Default::default(),
            states: Default::default(),
            stale_view: Rc::new(RefCell::new(
                crate::views::Empty {
                    id: ViewId::default(),
                }
                .into_any(),
            )),
        }
    }
}
