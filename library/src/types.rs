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
