#![feature(extern_crate_item_prelude)]
#![no_std]
extern crate embedded_hal;
#[macro_use(block)]
extern crate nb;
#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
use std::prelude::*;

pub mod gdew042z15;
pub mod gui;

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
