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

    fn fill(&mut self, width: u32, color: Color);
    fn pixel(&mut self, color: Color);

    fn put_byte(&mut self, byte: u8);

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
        let left_mask = 0xffu8 << left_offset;
        let right_mask = (0x00ffu16 >> (8 - right_offset)) as u8;

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
                self.buffer[right_index] |= right_mask;
            } else {
                self.buffer[left_index] &= !left_mask;
                for i in (left_index + 1)..right_index {
                    self.buffer[i] = 0x0;
                }
                self.buffer[right_index] &= !right_mask;
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

#[derive(Clone)]
pub struct BytePacking {
    current_pixels: u8,
    current_pixel_count: u8,
}

impl BytePacking {
    fn new() -> BytePacking {
        BytePacking {
            current_pixels: 0,
            current_pixel_count: 0,
        }
    }

    fn fill<DisplayType>(&mut self, display: &mut DisplayType, mut width: u32, color: Color)
    where
        DisplayType: Display,
    {
        if self.current_pixel_count as u32 + width < 8 {
            self.current_pixels <<= width;
            if let Color::White = color {
                self.current_pixels |= 0xff >> (8 - width);
            }
            self.current_pixel_count += width as u8;
        } else {
            // Complete the first byte.
            if self.current_pixel_count != 0 {
                let remainder = 8 - self.current_pixel_count;
                self.current_pixels <<= 8 - self.current_pixel_count;
                if let Color::White = color {
                    self.current_pixels |= 0xff >> self.current_pixel_count;
                }
                display.put_byte(self.current_pixels);
                width -= remainder as u32;
            }
            // Send as many full bytes as possible.
            if let Color::White = color {
                while width >= 8 {
                    display.put_byte(0xff);
                    width -= 8;
                }
            } else {
                while width >= 8 {
                    display.put_byte(0x0);
                    width -= 8;
                }
            }
            // Last partial byte.
            if let Color::White = color {
                self.current_pixels = 0xff >> (8 - width);
            } else {
                self.current_pixels = 0;
            }
            self.current_pixel_count = width as u8;
        }
    }

    fn pixel<DisplayType>(&mut self, display: &mut DisplayType, color: Color)
    where
        DisplayType: Display,
    {
        self.current_pixels = (self.current_pixels << 1) | color as u8;
        self.current_pixel_count += 1;
        if self.current_pixel_count == 8 {
            display.put_byte(self.current_pixels);
            self.current_pixels = 0;
            self.current_pixel_count = 0;
        }
    }
}

#[cfg(test)]
pub struct TestDisplay {
    pub frame: Vec<u8>,
    byte_packing: BytePacking,
}

#[cfg(test)]
impl TestDisplay {
    pub fn new() -> Self {
        Self {
            frame: Vec::new(),
            byte_packing: BytePacking::new(),
        }
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

    fn fill(&mut self, width: u32, color: Color) {
        let mut bp = self.byte_packing.clone();
        bp.fill(self, width, color);
        self.byte_packing = bp;
    }
    fn pixel(&mut self, color: Color) {
        let mut bp = self.byte_packing.clone();
        bp.pixel(self, color);
        self.byte_packing = bp;
    }

    fn put_byte(&mut self, byte: u8) {
        assert!(self.frame.len() < (Self::WIDTH * Self::HEIGHT) as usize);
        self.frame.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, RowRenderer};

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
                ok: [0xf0, 0xff, 0x0, 0x0],
            },
            FillTest {
                before: [0; 4],
                clip: (0, 32),
                fill: (4, 20),
                color: Color::White,
                ok: [0xf0, 0xff, 0x0f, 0x0],
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
                ok: [0, 0xe, 0, 0],
            },
            FillTest {
                before: [0; 4],
                clip: (6, 5),
                fill: (4, 12),
                color: Color::White,
                ok: [0, 0, 0, 0],
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
