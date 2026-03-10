#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub b: f32,
    pub g: f32,
    pub a: f32,
}

impl Color {
    const TRANSPARENT: Color = Color {
        r: 0.0,
        b: 0.0,
        g: 0.0,
        a: 0.0,
    };

    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, b, g, a }
    }

    /// Parses colors in the format:
    /// #RRGGBB
    /// #RRGGBBAA
    pub fn from_hex(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');

        let parse_byte = |s: &str| u8::from_str_radix(s, 16).ok();

        let (r, g, b, a) = match hex.len() {
            6 => {
                let r = parse_byte(&hex[0..2]);
                let g = parse_byte(&hex[2..4]);
                let b = parse_byte(&hex[4..6]);
                match (r, g, b) {
                    (Some(r), Some(g), Some(b)) => (r, g, b, 255),
                    _ => return Color::TRANSPARENT,
                }
            }
            8 => {
                let r = parse_byte(&hex[0..2]);
                let g = parse_byte(&hex[2..4]);
                let b = parse_byte(&hex[4..6]);
                let a = parse_byte(&hex[6..8]);
                match (r, g, b, a) {
                    (Some(r), Some(g), Some(b), Some(a)) => (r, g, b, a),
                    _ => return Color::TRANSPARENT,
                }
            }
            _ => return Color::TRANSPARENT,
        };

        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

impl Into<glyphon::Color> for Color {
    fn into(self) -> glyphon::Color {
        glyphon::Color::rgba(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}
