use crate::core::serde_derive::{Deserialize, Serialize};
use crate::math::*;
pub use hsl;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct PixelRGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelRGBA {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> PixelRGBA {
        PixelRGBA { r, g, b, a }
    }

    #[inline]
    pub const fn white() -> PixelRGBA {
        PixelRGBA {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
    #[inline]
    pub const fn black() -> PixelRGBA {
        PixelRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
    #[inline]
    pub const fn red() -> PixelRGBA {
        PixelRGBA {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }
    #[inline]
    pub const fn green() -> PixelRGBA {
        PixelRGBA {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        }
    }
    #[inline]
    pub const fn blue() -> PixelRGBA {
        PixelRGBA {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        }
    }
    #[inline]
    pub const fn yellow() -> PixelRGBA {
        PixelRGBA {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        }
    }
    #[inline]
    pub const fn magenta() -> PixelRGBA {
        PixelRGBA {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        }
    }
    #[inline]
    pub const fn cyan() -> PixelRGBA {
        PixelRGBA {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        }
    }
    #[inline]
    pub const fn transparent() -> PixelRGBA {
        PixelRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
    #[inline]
    pub const fn greyscale(value: u8) -> PixelRGBA {
        PixelRGBA {
            r: value,
            g: value,
            b: value,
            a: 255,
        }
    }

    #[inline]
    pub fn new_random(random: &mut Random) -> PixelRGBA {
        PixelRGBA::from_hex_rgba(random.u32())
    }

    #[inline]
    pub fn new_random_non_translucent(random: &mut Random) -> PixelRGBA {
        PixelRGBA {
            r: random.u32_bounded(255) as u8,
            g: random.u32_bounded(255) as u8,
            b: random.u32_bounded(255) as u8,
            a: 255,
        }
    }

    #[inline]
    pub fn from_color(input: Color) -> PixelRGBA {
        PixelRGBA {
            r: clampf(input.r * 255.0, 0.0, 255.0) as u8,
            g: clampf(input.g * 255.0, 0.0, 255.0) as u8,
            b: clampf(input.b * 255.0, 0.0, 255.0) as u8,
            a: clampf(input.a * 255.0, 0.0, 255.0) as u8,
        }
    }

    #[inline]
    pub fn to_color(self) -> Color {
        Color::from_pixelrgba(self)
    }

    #[inline]
    pub const fn from_hex_rgba(hex_rgba: u32) -> PixelRGBA {
        const RGBA_MASK_R: u32 = 0xff000000;
        const RGBA_MASK_G: u32 = 0x00ff0000;
        const RGBA_MASK_B: u32 = 0x0000ff00;
        const RGBA_MASK_A: u32 = 0x000000ff;

        const RGBA_SHIFT_R: u32 = 24;
        const RGBA_SHIFT_G: u32 = 16;
        const RGBA_SHIFT_B: u32 = 8;
        const RGBA_SHIFT_A: u32 = 0;

        PixelRGBA {
            r: ((hex_rgba & RGBA_MASK_R) >> RGBA_SHIFT_R) as u8,
            g: ((hex_rgba & RGBA_MASK_G) >> RGBA_SHIFT_G) as u8,
            b: ((hex_rgba & RGBA_MASK_B) >> RGBA_SHIFT_B) as u8,
            a: ((hex_rgba & RGBA_MASK_A) >> RGBA_SHIFT_A) as u8,
        }
    }

    /// Can be used to somewhat order colors by hue -> luminosity -> saturation
    #[inline]
    pub fn compare_by_hue_luminosity_saturation(
        color_a: &PixelRGBA,
        color_b: &PixelRGBA,
    ) -> core::cmp::Ordering {
        let color_a_hsl = hsl::HSL::from_rgb(&[color_a.r, color_a.g, color_a.b]);
        let color_b_hsl = hsl::HSL::from_rgb(&[color_b.r, color_b.g, color_b.b]);
        if color_a_hsl.h < color_b_hsl.h {
            core::cmp::Ordering::Less
        } else if color_a_hsl.h > color_b_hsl.h {
            core::cmp::Ordering::Greater
        } else {
            if color_a_hsl.l < color_b_hsl.l {
                core::cmp::Ordering::Less
            } else if color_a_hsl.l > color_b_hsl.l {
                core::cmp::Ordering::Greater
            } else {
                if color_a_hsl.s < color_b_hsl.s {
                    core::cmp::Ordering::Less
                } else if color_a_hsl.s > color_b_hsl.s {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Equal
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// Premultiplied RGBA
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn to_slice(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
    #[inline]
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }
    #[inline]
    pub fn from_rgb_bytes(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    #[inline]
    pub fn from_rgba_bytes(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    #[inline]
    pub fn from_pixelrgba(input: PixelRGBA) -> Color {
        Color {
            r: input.r as f32 / 255.0,
            g: input.g as f32 / 255.0,
            b: input.b as f32 / 255.0,
            a: input.a as f32 / 255.0,
        }
    }

    #[inline]
    pub fn to_pixelrgba(self) -> PixelRGBA {
        PixelRGBA::from_color(self)
    }

    #[inline]
    pub fn from_hex_rgba(hex_rgba: u32) -> Color {
        Color::from_pixelrgba(PixelRGBA::from_hex_rgba(hex_rgba))
    }

    #[inline]
    pub const fn with_same_channels(value: f32) -> Color {
        Color {
            r: value,
            g: value,
            b: value,
            a: value,
        }
    }
    #[inline]
    pub const fn greyscale(value: f32) -> Color {
        Color {
            r: value,
            g: value,
            b: value,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn transparent() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }
    #[inline]
    pub const fn black() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn white() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn red() -> Color {
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn green() -> Color {
        Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn blue() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn magenta() -> Color {
        Color {
            r: 1.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn yellow() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        }
    }
    #[inline]
    pub const fn cyan() -> Color {
        Color {
            r: 0.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }

    #[inline]
    pub fn mix(start: Color, end: Color, percent: f32) -> Color {
        Color::lerp(start, end, percent)
    }

    #[inline]
    #[must_use = "This does not change the original color"]
    pub fn with_color_multiplied_by(self, factor: f32) -> Color {
        Color {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a,
        }
    }

    #[inline]
    #[must_use = "This does not change the original color"]
    pub fn with_translucency(self, alpha: f32) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a * alpha,
        }
    }

    #[inline]
    #[must_use = "This does not change the original color"]
    pub fn made_opaque(self) -> Color {
        Color { a: 1.0, ..self }
    }

    pub fn to_premultiplied_alpha(self) -> Color {
        Color {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    pub fn to_unpremultipled_alpha(self) -> Color {
        if self.a == 0.0 {
            Color::transparent()
        } else {
            Color {
                r: self.r / self.a,
                g: self.g / self.a,
                b: self.b / self.a,
                a: self.a,
            }
        }
    }

    /// Based on https://en.wikipedia.org/wiki/SRGB#The_reverse_transformation
    #[inline]
    pub fn convert_to_srgba(self) -> Color {
        fn rgb_component_to_srgb_component(component: f32) -> f32 {
            if component < 0.04045 {
                (25.0 / 323.0) * component
            } else {
                ((200.0 * component + 11.0) / 211.0).powf(12.0 / 5.0)
            }
        }

        Color {
            r: rgb_component_to_srgb_component(self.r),
            g: rgb_component_to_srgb_component(self.g),
            b: rgb_component_to_srgb_component(self.b),
            a: self.a,
        }
    }

    /// Based on https://en.wikipedia.org/wiki/Relative_luminance
    #[inline]
    pub fn to_relative_luminance(self) -> f32 {
        let color_srgba = self.convert_to_srgba();
        0.2126 * color_srgba.r + 0.7152 * color_srgba.g + 0.0722 * color_srgba.b
    }

    /// From https://cairographics.org/operators/
    #[inline]
    pub fn luminosity(self) -> f32 {
        0.3 * self.r + 0.59 * self.g + 0.11 * self.b
    }

    /// From https://cairographics.org/operators/
    #[inline]
    pub fn saturation(self) -> f32 {
        f32::max(f32::max(self.r, self.g), self.b) - f32::min(f32::max(self.r, self.g), self.b)
    }

    /// From https://cairographics.org/operators/
    #[inline]
    #[must_use = "This does not change the original color"]
    pub fn with_replaced_luminosity(self, new_luminosity: f32) -> Color {
        let d = new_luminosity - self.luminosity();
        let mut result = Color {
            r: self.r + d,
            g: self.g + d,
            b: self.b + d,
            a: self.a,
        };

        // Clip Color
        let l = result.luminosity();
        let n = f32::min(f32::min(result.r, result.g), result.b);
        let x = f32::max(f32::max(result.r, result.g), result.b);
        if n < 0.0 {
            result.r = l + (((result.r - l) * l) / (l - n));
            result.g = l + (((result.g - l) * l) / (l - n));
            result.b = l + (((result.b - l) * l) / (l - n));
        }
        if x > 1.0 {
            result.r = l + (((result.r - l) * (1.0 - l)) / (x - l));
            result.g = l + (((result.g - l) * (1.0 - l)) / (x - l));
            result.b = l + (((result.b - l) * (1.0 - l)) / (x - l));
        }

        result
    }

    #[inline]
    pub fn premultiplied_alpha_blend_normal(source: Color, dest: Color) -> Color {
        return Color::premultiplied_alpha_blend_with_function(
            source,
            dest,
            |source_x, _source_a, _dest_x, dest_a| source_x * dest_a,
        );
    }

    #[inline]
    pub fn premultiplied_alpha_blend_multiply(source: Color, dest: Color) -> Color {
        return Color::premultiplied_alpha_blend_with_function(
            source,
            dest,
            |source_x, _source_a, dest_x, _dest_a| source_x * dest_x,
        );
    }

    #[inline]
    pub fn premultiplied_alpha_blend_screen(source: Color, dest: Color) -> Color {
        return Color::premultiplied_alpha_blend_with_function(
            source,
            dest,
            |source_x, source_a, dest_x, dest_a| {
                dest_a * source_x + source_a * dest_x - source_x * dest_x
            },
        );
    }

    #[inline]
    pub fn premultiplied_alpha_blend_luminosity(source: Color, dest: Color) -> Color {
        Color::premultiplied_alpha_blend_with_non_separable_function(
            source,
            dest,
            |source, dest| {
                let source = source.to_unpremultipled_alpha();
                let dest = dest.to_unpremultipled_alpha();
                dest.with_replaced_luminosity(source.luminosity())
            },
        )
    }

    #[inline]
    fn premultiplied_alpha_blend_with_function<F: Fn(f32, f32, f32, f32) -> f32>(
        source: Color,
        dest: Color,
        blend_function: F,
    ) -> Color {
        // From https://cairographics.org/operators/
        // result.a = source.a + dest.a·(1−source.a)
        // result.x = (1/result.a)*[(1-dest.a)*source.a*source.x + (1-source.a)*dest.a*dest.x + source.a*dest.a*blend_func(source.x, dest.x)]

        let a = source.a + dest.a * (1.0 - source.a);
        if a == 0.0 {
            Color::transparent()
        } else {
            let mul1 = 1.0 - dest.a;
            let mul2 = 1.0 - source.a;
            // NOTE: The `blend_function` needs to already contain the `source.a * dest.a` term to
            //       work correctly
            let r = mul1 * source.r
                + mul2 * dest.r
                + blend_function(source.r, source.a, dest.r, dest.a);
            let g = mul1 * source.g
                + mul2 * dest.g
                + blend_function(source.g, source.a, dest.g, dest.a);
            let b = mul1 * source.b
                + mul2 * dest.b
                + blend_function(source.b, source.a, dest.b, dest.a);

            Color { r, g, b, a }
        }
    }

    #[inline]
    fn premultiplied_alpha_blend_with_non_separable_function<F: Fn(Color, Color) -> Color>(
        source: Color,
        dest: Color,
        blend_function: F,
    ) -> Color {
        // From https://cairographics.org/operators/
        // result.a = source.a + dest.a·(1−source.a)
        // result.x = (1/result.a)*[(1-dest.a)*source.a*source.x + (1-source.a)*dest.a*dest.x + source.a*dest.a*blend_func(source.x, dest.x)]

        let a = source.a + dest.a * (1.0 - source.a);
        if a == 0.0 {
            Color::transparent()
        } else {
            let mul1 = 1.0 - dest.a;
            let mul2 = 1.0 - source.a;
            let mul3 = source.a * dest.a * blend_function(source, dest);
            let r = mul1 * source.r + mul2 * dest.r + mul3.r;
            let g = mul1 * source.g + mul2 * dest.g + mul3.g;
            let b = mul1 * source.b + mul2 * dest.b + mul3.b;

            Color { r, g, b, a }
        }
    }
}

impl Lerp for Color {
    #[inline]
    fn lerp_value(start: Color, end: Color, percent: f32) -> Color {
        Color::lerp(start, end, percent)
    }
}

impl Color {
    #[inline]
    pub fn dot(v: Color, u: Color) -> f32 {
        v.r * u.r + v.g * u.g + v.b * u.b + v.a * u.a
    }

    #[must_use]
    #[inline]
    pub fn normalized(self) -> Color {
        self / self.magnitude()
    }

    #[inline]
    pub fn is_normalized(self) -> bool {
        let normalized = Color::normalized(self);
        (normalized - self).is_effectively_zero()
    }

    #[inline]
    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn magnitude_squared(self) -> f32 {
        Color::dot(self, self)
    }

    #[inline]
    pub fn distance(v: Color, u: Color) -> f32 {
        (v - u).magnitude()
    }

    #[inline]
    pub fn distance_squared(v: Color, u: Color) -> f32 {
        (v - u).magnitude_squared()
    }

    #[inline]
    /// Liner interpolation
    pub fn lerp(start: Color, end: Color, percent: f32) -> Color {
        Color {
            r: start.r + percent * (end.r - start.r),
            g: start.g + percent * (end.g - start.g),
            b: start.b + percent * (end.b - start.b),
            a: start.a + percent * (end.a - start.a),
        }
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        Color::dot(self, self) == 0.0
    }

    #[inline]
    pub fn is_effectively_zero(self) -> bool {
        Color::dot(self, self) < EPSILON
    }
}

use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;

impl Neg for Color {
    type Output = Color;
    #[inline]
    fn neg(self) -> Color {
        Color {
            r: -self.r,
            g: -self.g,
            b: -self.b,
            a: -self.a,
        }
    }
}

impl Add<Color> for Color {
    type Output = Color;
    #[inline]
    fn add(self, rhs: Color) -> Color {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}
impl AddAssign<Color> for Color {
    #[inline]
    fn add_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}
impl Add<f32> for Color {
    type Output = Color;
    #[inline]
    fn add(self, rhs: f32) -> Color {
        Color {
            r: self.r + rhs,
            g: self.g + rhs,
            b: self.b + rhs,
            a: self.a + rhs,
        }
    }
}
impl Add<Color> for f32 {
    type Output = Color;
    #[inline]
    fn add(self, rhs: Color) -> Color {
        Color {
            r: self + rhs.r,
            g: self + rhs.g,
            b: self + rhs.b,
            a: self + rhs.a,
        }
    }
}
impl AddAssign<f32> for Color {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Color {
            r: self.r + rhs,
            g: self.g + rhs,
            b: self.b + rhs,
            a: self.a + rhs,
        }
    }
}
impl Add<i32> for Color {
    type Output = Color;
    #[inline]
    fn add(self, rhs: i32) -> Color {
        Color {
            r: self.r + rhs as f32,
            g: self.g + rhs as f32,
            b: self.b + rhs as f32,
            a: self.a + rhs as f32,
        }
    }
}
impl Add<Color> for i32 {
    type Output = Color;
    #[inline]
    fn add(self, rhs: Color) -> Color {
        Color {
            r: self as f32 + rhs.r,
            g: self as f32 + rhs.g,
            b: self as f32 + rhs.b,
            a: self as f32 + rhs.a,
        }
    }
}
impl AddAssign<i32> for Color {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        *self = Color {
            r: self.r + rhs as f32,
            g: self.g + rhs as f32,
            b: self.b + rhs as f32,
            a: self.a + rhs as f32,
        }
    }
}

impl Sub<Color> for Color {
    type Output = Color;
    #[inline]
    fn sub(self, rhs: Color) -> Color {
        Color {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
            a: self.a - rhs.a,
        }
    }
}
impl SubAssign<Color> for Color {
    #[inline]
    fn sub_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
            a: self.a - rhs.a,
        }
    }
}
impl Sub<f32> for Color {
    type Output = Color;
    #[inline]
    fn sub(self, rhs: f32) -> Color {
        Color {
            r: self.r - rhs,
            g: self.g - rhs,
            b: self.b - rhs,
            a: self.a - rhs,
        }
    }
}
impl SubAssign<f32> for Color {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Color {
            r: self.r - rhs,
            g: self.g - rhs,
            b: self.b - rhs,
            a: self.a - rhs,
        }
    }
}
impl Sub<i32> for Color {
    type Output = Color;
    #[inline]
    fn sub(self, rhs: i32) -> Color {
        Color {
            r: self.r - rhs as f32,
            g: self.g - rhs as f32,
            b: self.b - rhs as f32,
            a: self.a - rhs as f32,
        }
    }
}
impl SubAssign<i32> for Color {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        *self = Color {
            r: self.r - rhs as f32,
            g: self.g - rhs as f32,
            b: self.b - rhs as f32,
            a: self.a - rhs as f32,
        }
    }
}

impl Mul<Color> for Color {
    type Output = Color;
    #[inline]
    fn mul(self, rhs: Color) -> Color {
        Color {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
            a: self.a * rhs.a,
        }
    }
}
impl MulAssign<Color> for Color {
    #[inline]
    fn mul_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
            a: self.a * rhs.a,
        }
    }
}
impl Mul<f32> for Color {
    type Output = Color;
    #[inline]
    fn mul(self, rhs: f32) -> Color {
        Color {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}
impl Mul<Color> for f32 {
    type Output = Color;
    #[inline]
    fn mul(self, rhs: Color) -> Color {
        Color {
            r: self * rhs.r,
            g: self * rhs.g,
            b: self * rhs.b,
            a: self * rhs.a,
        }
    }
}
impl MulAssign<f32> for Color {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Color {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}
impl Mul<i32> for Color {
    type Output = Color;
    #[inline]
    fn mul(self, rhs: i32) -> Color {
        Color {
            r: self.r * rhs as f32,
            g: self.g * rhs as f32,
            b: self.b * rhs as f32,
            a: self.a * rhs as f32,
        }
    }
}
impl Mul<Color> for i32 {
    type Output = Color;
    #[inline]
    fn mul(self, rhs: Color) -> Color {
        Color {
            r: self as f32 * rhs.r,
            g: self as f32 * rhs.g,
            b: self as f32 * rhs.b,
            a: self as f32 * rhs.a,
        }
    }
}
impl MulAssign<i32> for Color {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = Color {
            r: self.r * rhs as f32,
            g: self.g * rhs as f32,
            b: self.b * rhs as f32,
            a: self.a * rhs as f32,
        }
    }
}

impl Div<Color> for Color {
    type Output = Color;
    #[inline]
    fn div(self, rhs: Color) -> Color {
        Color {
            r: self.r / rhs.r,
            g: self.g / rhs.g,
            b: self.b / rhs.b,
            a: self.a / rhs.a,
        }
    }
}
impl DivAssign<Color> for Color {
    #[inline]
    fn div_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r / rhs.r,
            g: self.g / rhs.g,
            b: self.b / rhs.b,
            a: self.a / rhs.a,
        }
    }
}
impl Div<f32> for Color {
    type Output = Color;
    #[inline]
    fn div(self, rhs: f32) -> Color {
        Color {
            r: self.r / rhs,
            g: self.g / rhs,
            b: self.b / rhs,
            a: self.a / rhs,
        }
    }
}
impl DivAssign<f32> for Color {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Color {
            r: self.r / rhs,
            g: self.g / rhs,
            b: self.b / rhs,
            a: self.a / rhs,
        }
    }
}
impl Div<i32> for Color {
    type Output = Color;
    #[inline]
    fn div(self, rhs: i32) -> Color {
        Color {
            r: self.r / rhs as f32,
            g: self.g / rhs as f32,
            b: self.b / rhs as f32,
            a: self.a / rhs as f32,
        }
    }
}
impl DivAssign<i32> for Color {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = Color {
            r: self.r / rhs as f32,
            g: self.g / rhs as f32,
            b: self.b / rhs as f32,
            a: self.a / rhs as f32,
        }
    }
}
