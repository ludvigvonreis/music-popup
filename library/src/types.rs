use crate::color::Color;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub viewport: [f32; 2], // width, height (physical pixels)
    pub _padding: [f32; 2], // required padding
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    pub position: [f32; 2], // 8 bytes
    pub size: [f32; 2],     // 8 bytes
    pub color: [f32; 4],    // 16 bytes
    //pub texture_index: u32, // 4 bytes
    pub radius: f32,        // 4 bytes
    pub _padding: [f32; 3], // pad to 16-byte alignment
}

#[derive(Debug, Clone)]
pub struct TextEntry {
    pub buffer: glyphon::Buffer,
    pub x: f32,
    pub y: f32,
    pub color: Color,
}

use std::{
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_rect: [f32; 4], // x, y, w, h in 0..1
    pub tint: [f32; 4],
    pub radius: f32,
    pub _padding: [f32; 3],
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ImageId(u64);

pub struct ImageData {
    pub id: ImageId,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct RawImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ImageData {
    pub fn from_bytes(bytes: &[u8]) -> Result<Rc<Self>, image::ImageError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let img = image::load_from_memory(bytes)?.to_rgba8();
        Ok(Rc::new(Self {
            id: ImageId(NEXT.fetch_add(1, Ordering::Relaxed)),
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        }))
    }

    pub fn from_raw(raw: &RawImage) -> Rc<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1 << 32); // avoid clashing with from_bytes ids
        Rc::new(Self {
            id: ImageId(NEXT.fetch_add(1, Ordering::Relaxed)),
            width: raw.width,
            height: raw.height,
            rgba: raw.rgba.clone(),
        })
    }
}
