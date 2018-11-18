use crate::{ClipRow, Color, RowRenderer};

pub struct RLEImage {
    pub data: &'static [u16],
    pub width: u16,
    pub height: u16,
}

impl RLEImage {
    pub fn render_row_transparent(
        &self,
        row: &mut RowRenderer,
        clip: &ClipRow,
        y: i32,
        offset: i32,
    ) {
        if y < 0 {
            return;
        }
        if y >= self.height as i32 {
            return;
        }
        let line_start = self.data[y as usize] as usize;
        let line_end = self.data[y as usize + 1] as usize;
        let line = &self.data[line_start..line_end];

        let mut pos = 0;
        for run in line {
            let length = (run & 0x7fff) as u32;
            if (run >> 15) != 0u16 {
                row.fill(
                    clip,
                    offset + pos,
                    offset + pos + length as i32,
                    Color::Black,
                );
            }
            pos += length as i32;
        }
    }
}
