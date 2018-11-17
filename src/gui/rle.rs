use crate::{Color, Display};

use core::cmp::max;
use core::cmp::min;

pub struct RLEImage {
    pub data: &'static [u16],
    pub width: u16,
    pub height: u16,
}

impl RLEImage {
    pub fn render_line<DisplayType>(&self, display: &mut DisplayType, y: u32, left: u32, right: u32)
    where
        DisplayType: Display,
    {
        let line_start = self.data[y as usize] as usize;
        let line_end = self.data[y as usize + 1] as usize;
        let line = &self.data[line_start..line_end];

        let mut pos = 0;
        for run in line {
            if pos >= right {
                break;
            }
            let length = (run & 0x7fff) as u32;
            if pos + length < left {
                pos += length;
                continue;
            }
            let color = if (run >> 15) != 0u16 {
                Color::Black
            } else {
                Color::White
            };
            let run_left = max(pos, left);
            let run_right = min(pos + length, right);
            display.fill(run_right - run_left, color);
            pos += length;
        }
    }
}
