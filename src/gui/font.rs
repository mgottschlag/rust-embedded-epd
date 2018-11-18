use super::rle::RLEImage;
use crate::{ClipRow, Color, RowRenderer};

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

    pub fn render_row(
        &self,
        row: &mut RowRenderer,
        clip: &ClipRow,
        text: &str,
        y: i32,
        offset: i32,
    ) {
        // TODO: Do we need to fill the background?
        row.fill(clip, 0, core::i32::MAX, Color::White);
        if y < 1 || y > (self.ascender + self.descender) as i32 {
            return;
        }
        let mut pos = offset;
        // TODO: Discard glyphs if they are definitely not shown.
        for c in text.chars() {
            let index = (self.get_glyph_index)(c);
            if index.is_none() {
                continue;
            }
            let glyph = &self.glyphs[index.unwrap()];
            self.render_glyph_row(row, clip, glyph, y, pos);
            pos += glyph.advance as i32;
        }
    }

    fn render_glyph_row(
        &self,
        row: &mut RowRenderer,
        clip: &ClipRow,
        glyph: &Glyph,
        y: i32,
        offset: i32,
    ) {
        let image_offset = offset + glyph.image_left as i32;
        let image_y = y - self.ascender as i32 + glyph.image_top as i32;
        glyph
            .image
            .render_row_transparent(row, clip, image_y, image_offset);
    }
}

pub struct Glyph {
    pub image: RLEImage,
    pub image_left: i16,
    pub image_top: i16,
    pub advance: u32,
}
