//! A straightforward linear-friendly RGBA color type.
//!
//! Channels are stored as `f32` in the `0.0..=1.0` range. Values are treated as
//! non-premultiplied sRGB by convention; conversion to the linear space needed
//! by a GPU is the renderer's responsibility.

/// An RGBA color with `f32` channels in `0.0..=1.0`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);

    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Construct from 8-bit sRGB channels.
    #[inline]
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    #[inline]
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba8(r, g, b, 255)
    }

    /// Parse a hex color string. Accepts `#RGB`, `#RGBA`, `#RRGGBB`, and
    /// `#RRGGBBAA` (the leading `#` is optional). Returns `None` on any
    /// malformed input.
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let parse = |slice: &str| u8::from_str_radix(slice, 16).ok();
        match s.len() {
            3 | 4 => {
                let dup = |c: char| {
                    let mut buf = [0u8; 2];
                    let one = c.encode_utf8(&mut buf).as_bytes()[0];
                    u8::from_str_radix(
                        std::str::from_utf8(&[one, one]).ok()?,
                        16,
                    )
                    .ok()
                };
                let mut chars = s.chars();
                let r = dup(chars.next()?)?;
                let g = dup(chars.next()?)?;
                let b = dup(chars.next()?)?;
                let a = match chars.next() {
                    Some(c) => dup(c)?,
                    None => 255,
                };
                Some(Self::rgba8(r, g, b, a))
            }
            6 => Some(Self::rgb8(
                parse(&s[0..2])?,
                parse(&s[2..4])?,
                parse(&s[4..6])?,
            )),
            8 => Some(Self::rgba8(
                parse(&s[0..2])?,
                parse(&s[2..4])?,
                parse(&s[4..6])?,
                parse(&s[6..8])?,
            )),
            _ => None,
        }
    }

    /// Returns a copy with the alpha channel replaced.
    #[inline]
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Linearly interpolate between two colors (`t` clamped to `0.0..=1.0`).
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color::rgba(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    /// The channels as a `[f32; 4]`, convenient for GPU uploads.
    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert each sRGB channel to linear space (alpha left unchanged).
    pub fn to_linear(self) -> [f32; 4] {
        fn c(x: f32) -> f32 {
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        }
        [c(self.r), c(self.g), c(self.b), self.a]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_long() {
        assert_eq!(Color::from_hex("#ff8000"), Some(Color::rgb8(255, 128, 0)));
        assert_eq!(
            Color::from_hex("112233ff"),
            Some(Color::rgba8(0x11, 0x22, 0x33, 0xff))
        );
    }

    #[test]
    fn hex_short() {
        assert_eq!(Color::from_hex("#f80"), Some(Color::rgb8(0xff, 0x88, 0x00)));
        assert_eq!(
            Color::from_hex("#f80c"),
            Some(Color::rgba8(0xff, 0x88, 0x00, 0xcc))
        );
    }

    #[test]
    fn hex_invalid() {
        assert_eq!(Color::from_hex("#xyz"), None);
        assert_eq!(Color::from_hex("12345"), None);
        assert_eq!(Color::from_hex(""), None);
    }
}
