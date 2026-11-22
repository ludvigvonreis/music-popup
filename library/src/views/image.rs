use std::rc::Rc;
use std::{any::Any, sync::Arc};

use crate::backend::layout_context::LayoutContext;
use crate::{
    backend::render_context::RenderContext,
    reactive::create_effect,
    types::{ImageData, ImageInstance, RawImage},
    view::{View, update::push_update, view_id::ViewId},
};

/// Dynamic image. `f` runs in an effect, so signals read inside it keep the
/// image up to date: `remote_image(move || p.art.get())`.
pub fn remote_image(f: impl Fn() -> Option<Arc<RawImage>> + 'static) -> RemoteImage {
    let img = RemoteImage {
        id: ViewId::new(),
        data: None,
    };
    let id = img.id;
    create_effect(move || {
        push_update(id, Box::new(f()));
    });
    img
}

pub struct RemoteImage {
    id: ViewId,
    data: Option<Rc<ImageData>>,
}

impl View for RemoteImage {
    fn id(&self) -> ViewId {
        self.id
    }

    fn update(&mut self, state: Box<dyn Any>) {
        if let Ok(raw) = state.downcast::<Option<Arc<RawImage>>>() {
            // Built once per new image, so the texture id is stable across frames.
            self.data = raw.as_ref().as_ref().map(|r| ImageData::from_raw(r));
        }
    }

    fn measure(
        &mut self,
        _ctx: &mut LayoutContext,
        known: taffy::Size<Option<f32>>,
        _available: taffy::Size<taffy::AvailableSpace>,
    ) -> Option<taffy::Size<f32>> {
        let d = self.data.as_ref()?;
        let (iw, ih) = (d.width as f32, d.height as f32);
        Some(match (known.width, known.height) {
            (Some(w), Some(h)) => taffy::Size {
                width: w,
                height: h,
            },
            (Some(w), None) => taffy::Size {
                width: w,
                height: w * ih / iw,
            },
            (None, Some(h)) => taffy::Size {
                width: h * iw / ih,
                height: h,
            },
            (None, None) => taffy::Size {
                width: iw,
                height: ih,
            },
        })
    }

    fn paint(&self, ctx: &mut RenderContext) {
        let Some(data) = &self.data else { return };
        let style = self.id.state().borrow().style.clone();
        let r = self.id.rect();
        ctx.image(
            data,
            ImageInstance {
                position: [r.x, r.y],
                size: [r.w, r.h],
                uv_rect: [0.0, 0.0, 1.0, 1.0],
                tint: [1.0; 4],
                radius: style.border_radius,
                _padding: [0.0; 3],
            },
            None,
        );
    }
}
