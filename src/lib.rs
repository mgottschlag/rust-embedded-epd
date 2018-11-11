#![no_std]
extern crate embedded_hal;

#[macro_use(block)]
extern crate nb;

pub mod gdew042z15;

pub enum Error {}

#[derive(Copy, Clone)]
pub enum Color {
    Black,
    White,
}

pub trait Display {
    const WIDTH: u32;
    const HEIGHT: u32;

    fn start_frame(&mut self) -> nb::Result<(), Error>;
    fn end_frame(&mut self);

    /*fn start_partial(&mut self, left: u32, top: u32, right: u32, bottom: u32) -> nb::Result<(), Error>;
    fn end_partial(&mut self);*/

    fn fill(&mut self, width: u32, color: Color);
    fn pixel(&mut self, color: Color);
}
