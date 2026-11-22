use std::cell::RefCell;

use slotmap::SecondaryMap;
use taffy::{AvailableSpace, NodeId, Size, TaffyTree, style_helpers::{auto, length}};

use crate::{backend::layout_context::LayoutContext, view::view_id::ViewId};

/// Absolute rect in window pixels.
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

struct LayoutState {
    // The context is the ViewId of leaf nodes, so the measure callback can find the view.
    tree: TaffyTree<ViewId>,
    nodes: SecondaryMap<ViewId, NodeId>,
    rects: SecondaryMap<ViewId, Rect>,
}

thread_local! {
    static LAYOUT: RefCell<LayoutState> = RefCell::new(LayoutState {
        tree: TaffyTree::new(),
        nodes: SecondaryMap::new(),
        rects: SecondaryMap::new(),
    });
}

impl ViewId {
    /// Result of the last layout pass.
    pub fn rect(&self) -> Rect {
        LAYOUT.with_borrow(|s| s.rects.get(*self).copied().unwrap_or_default())
    }
}

/// Sync view styles into taffy, solve, and store absolute rects.
pub fn compute(root: ViewId, viewport: (f32, f32), ctx: &mut LayoutContext) {
    LAYOUT.with_borrow_mut(|st| {
        let root_node = sync(st, root);

        // The root always fills the window.
        let mut root_style = st.tree.style(root_node).unwrap().clone();
        root_style.size = Size { width: length(viewport.0), height: length(viewport.1) };
        st.tree.set_style(root_node, root_style).unwrap();

        st.tree
            .compute_layout_with_measure(
                root_node,
                Size {
                    width: AvailableSpace::Definite(viewport.0),
                    height: AvailableSpace::Definite(viewport.1),
                },
                |known, available, _node, view, _style| match view {
                    Some(id) => id
                        .view()
                        .borrow_mut()
                        .measure(ctx, known, available)
                        .unwrap_or(Size::ZERO),
                    None => Size::ZERO,
                },
            )
            .unwrap();

        collect(st, root, 0.0, 0.0);
    });
}

fn sync(st: &mut LayoutState, id: ViewId) -> NodeId {
    let style = id.state().borrow().style.clone();
    let children = id.children();
    let child_nodes: Vec<NodeId> = children.iter().map(|c| sync(st, *c)).collect();

    let mut ts = style.layout.clone();
    if children.is_empty() && ts.min_size.width == auto() {
        // Leaves (text, images) may shrink below content size; overflow is clipped.
        ts.min_size.width = length(0.0);
    }

    let leaf_ctx = children.is_empty().then_some(id);

    match st.nodes.get(id).copied() {
        Some(node) => {
            st.tree.set_style(node, ts).unwrap();
            st.tree.set_children(node, &child_nodes).unwrap();
            st.tree.set_node_context(node, leaf_ctx).unwrap();
            node
        }
        None => {
            let node = match leaf_ctx {
                Some(vid) => st.tree.new_leaf_with_context(ts, vid).unwrap(),
                None => st.tree.new_with_children(ts, &child_nodes).unwrap(),
            };
            st.nodes.insert(id, node);
            node
        }
    }
}

fn collect(st: &mut LayoutState, id: ViewId, ox: f32, oy: f32) {
    let node = st.nodes[id];
    let l = *st.tree.layout(node).unwrap();
    let (x, y) = (ox + l.location.x, oy + l.location.y);
    st.rects.insert(id, Rect { x, y, w: l.size.width, h: l.size.height });
    for c in id.children() {
        collect(st, c, x, y);
    }
}
