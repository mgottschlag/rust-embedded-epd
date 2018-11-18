#![feature(extern_crate_item_prelude)]
#![no_std]
extern crate embedded_hal;
#[macro_use(block)]
extern crate nb;
#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
use std::vec::Vec;

pub mod gdew042z15;
pub mod gui;

use core::cmp::max;
use core::cmp::min;

pub enum Error {}

/// Custom time type. The timer is supposed to expire in the specified frequency.
#[derive(Clone, Copy)]
pub struct Hertz(pub u32);

#[derive(Copy, Clone)]
pub enum Color {
    White,
    Black,
}

pub trait Display {
    const WIDTH: u32;
    const HEIGHT: u32;

    fn start_frame(&mut self) -> nb::Result<(), Error>;
    fn end_frame(&mut self);

    fn draw_row(&mut self, row: &[u8]);

    fn width(&self) -> u32 {
        Self::WIDTH
    }

    fn height(&self) -> u32 {
        Self::HEIGHT
    }
}

pub trait PartialRefresh {
    fn start_partial(
        &mut self,
        left: u32,
        top: u32,
        right: u32,
        bottom: u32,
    ) -> nb::Result<(), Error>;
    fn end_partial(&mut self);

    fn draw_partial_row(&mut self, row: &[u8]);
}

pub struct RowRenderer<'a> {
    buffer: &'a mut [u8],
    width: u32,
}

impl<'a> RowRenderer<'a> {
    pub fn new(buffer: &'a mut [u8], width: u32) -> RowRenderer<'a> {
        assert!(buffer.len() * 8 >= width as usize);
        RowRenderer {
            buffer: buffer,
            width: width,
        }
    }

    pub fn finish(self) {
        // Destroy the renderer and release the buffer.
    }

    pub fn fill(&mut self, clip: &ClipRow, left: i32, right: i32, color: Color) {
        let line_clip = clip.clip(left, right);
        if line_clip.is_empty() {
            return;
        }
        let (left, right) = line_clip.get();
        let (left_index, right_index) = ((left >> 3) as usize, (right >> 3) as usize);
        let (left_offset, right_offset) = (left & 7, right & 7);
        let left_mask = 0xffu8 >> left_offset;
        let right_mask = (0xff00u16 >> right_offset) as u8;

        if left_index == right_index {
            // Both ends are in the same byte.
            let mask = left_mask & right_mask;
            if let Color::White = color {
                self.buffer[left_index] |= mask;
            } else {
                self.buffer[left_index] &= !mask;
            }
        } else {
            // We cross byte boundaries.
            if let Color::White = color {
                self.buffer[left_index] |= left_mask;
                for i in (left_index + 1)..right_index {
                    self.buffer[i] = 0xff;
                }
                if right_offset != 0 {
                    self.buffer[right_index] |= right_mask;
                }
            } else {
                self.buffer[left_index] &= !left_mask;
                for i in (left_index + 1)..right_index {
                    self.buffer[i] = 0x0;
                }
                if right_offset != 0 {
                    self.buffer[right_index] &= !right_mask;
                }
            }
        }
    }

    pub fn render_bitmap(&mut self, _clip: &ClipRow, _left: i32, _right: i32, _bits: &[u8]) {
        // TODO
        panic!("Not yet implemented.");
    }

    pub fn full_row(&self) -> ClipRow {
        ClipRow {
            left: 0,
            right: self.width as i32,
        }
    }
}

pub struct ClipRow {
    left: i32,
    right: i32,
}

impl ClipRow {
    pub fn get(&self) -> (i32, i32) {
        (self.left, self.right)
    }

    pub fn clip(&self, left: i32, right: i32) -> ClipRow {
        ClipRow {
            left: max(self.left, left),
            right: min(self.right, right),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.left >= self.right
    }
}

#[cfg(test)]
pub struct TestDisplay {
    pub frame: Vec<u8>,
}

#[cfg(test)]
impl TestDisplay {
    pub fn new() -> Self {
        Self { frame: Vec::new() }
    }
}

#[cfg(test)]
impl Display for TestDisplay {
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    fn start_frame(&mut self) -> nb::Result<(), Error> {
        self.frame = Vec::new();
        Ok(())
    }
    fn end_frame(&mut self) {
        assert!(self.frame.len() == (Self::WIDTH * Self::HEIGHT) as usize);
    }

    fn draw_row(&mut self, row: &[u8]) {
        assert!(row.len() >= (Self::WIDTH as usize + 7) / 8);
        for i in 0..Self::WIDTH as usize / 8 {
            self.frame.push(row[i]);
        }
        assert!(row.len() <= (((Self::WIDTH + 7) / 8) * Self::HEIGHT) as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, RowRenderer};

    #[test]
    #[should_panic]
    fn test_row_renderer_new_panic() {
        let mut buffer = [0u8];
        RowRenderer::new(&mut buffer, 12);
    }

    struct FillTest {
        before: [u8; 4],
        clip: (i32, i32),
        fill: (i32, i32),
        color: Color,
        ok: [u8; 4],
    }

    #[test]
    fn test_row_renderer_fill() {
        let tests = [
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (4, 16),
                color: Color::White,
                ok: [0x0f, 0xff, 0x0, 0x0],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (4, 20),
                color: Color::White,
                ok: [0x0f, 0xff, 0xf0, 0x0],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (2, 6),
                color: Color::White,
                ok: [0x3c, 0x0, 0x0, 0x0],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (2, 6),
                color: Color::Black,
                ok: [0x0, 0x0, 0x0, 0x0],
            },
            FillTest {
                before: [0xff; 4],
                clip: (0, 32),
                fill: (2, 6),
                color: Color::Black,
                ok: [0xc3, 0xff, 0xff, 0xff],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (5, 5),
                color: Color::White,
                ok: [0; 4],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (-5, -4),
                color: Color::White,
                ok: [0; 4],
            },
            FillTest {
                before: [0; 4],
                clip: (9, 32),
                fill: (4, 12),
                color: Color::White,
                ok: [0, 0x70, 0, 0],
            },
            FillTest {
                before: [0; 4],
                clip: (6, 5),
                fill: (4, 12),
                color: Color::White,
                ok: [0, 0, 0, 0],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (16, 32),
                color: Color::White,
                ok: [0, 0, 0xff, 0xff],
            },
        ];
        for test in &tests {
            let mut buffer = test.before;
            let mut renderer = RowRenderer::new(&mut buffer[..], 32);
            let clip = renderer.full_row();
            let clip = clip.clip(test.clip.0, test.clip.1);
            renderer.fill(&clip, test.fill.0, test.fill.1, test.color);
            renderer.finish();
            println!("{:?} == {:?}?", buffer, test.ok);
            assert!(buffer == test.ok);
        }
    }
}
