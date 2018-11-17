use super::rle::RLEImage;
use crate::{Color, Display};

pub struct Font {
    pub ascender: u16,
    pub descender: u16,
    pub glyphs: &'static [Glyph],
    pub get_glyph_index: fn(c: char) -> Option<usize>,
}

impl Font {
    pub fn get_text_size(&self, text: &str) -> (u32, u32) {
        let mut width = 0;
        for c in text.chars() {
            let index = (self.get_glyph_index)(c);
            if index.is_none() {
                continue;
            }
            let glyph = &self.glyphs[index.unwrap()];
            width += glyph.advance;
        }
        (width, (self.ascender + self.descender) as u32)
    }

    pub fn render_line<DisplayType>(
        &self,
        display: &mut DisplayType,
        text: &str,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        if right < left {
            return;
        }
        if y > (self.ascender + self.descender) as u32 {
            display.fill(right - left, Color::White);
        }
        let mut pos = 0;
        for c in text.chars() {
            // Discard glyphs early if they are not shown at all.
            if pos >= right {
                break;
            }
            let index = (self.get_glyph_index)(c);
            if index.is_none() {
                continue;
            }
            let glyph = &self.glyphs[index.unwrap()];
            if pos + glyph.advance <= left {
                pos += glyph.advance;
                continue;
            }

            // Here, the glyph is at least partially shown. Transform the limits
            // into local coordinates.
            let glyph_left = if left < pos { 0 } else { left - pos };
            let glyph_right = if right >= pos + glyph.advance {
                glyph.advance
            } else {
                right - pos
            };
            self.render_glyph_line(display, glyph, y, glyph_left, glyph_right);
            pos += glyph.advance;
        }
    }

    fn render_glyph_line<DisplayType>(
        &self,
        display: &mut DisplayType,
        glyph: &Glyph,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        let min_y = self.ascender - glyph.image_top;
        let max_y = min_y + glyph.image.height;
        let min_x = glyph.image_left;
        let max_x = min_x + glyph.image.width;

        // Check whether the image is visible at all - if not, draw only the
        // background.
        if (y as u16) < min_y || y as u16 >= max_y {
            display.fill(right - left, Color::White);
            return;
        }
        if left as u16 >= max_x || right as u16 <= min_x {
            display.fill(right - left, Color::White);
            return;
        }

        // Left padding.
        if min_x as u32 > left {
            display.fill(min_x as u32 - left, Color::White);
        }
        // Image.
        let image_left = if left < min_x as u32 {
            0
        } else {
            left - min_x as u32
        };
        let image_right = if right > max_x as u32 {
            max_x as u32 - min_x as u32
        } else {
            right - min_x as u32
        };
        glyph
            .image
            .render_line(display, y - min_y as u32, image_left, image_right);
        // Right padding.
        if right > max_x as u32 {
            display.fill(right - max_x as u32, Color::White);
        }
    }
}

pub struct Glyph {
    pub image: RLEImage,
    pub image_left: u16,
    pub image_top: u16,
    pub advance: u32,
}
