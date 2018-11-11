# `embedded-epd`

> embedded-hal driver for e-paper displays

This library is meant as a driver for various two- and three-color e-paper
displays. The API is supposed to provide support for a wide range of graphical
interfaces even on systems with very little RAM. The library is therefore not
going to allocate complete framebuffers in RAM, and the whole image is
constructed on-the-fly in code.

Note that some of the interfaces are pretty ugly - if you have better ideas,
feel free to create a github issue.

Currently supported devices:

> Waveshare 4.2in BWR E-Paper Display

# [Documentation](https://docs.rs/embedded-epd)

# License

Licensed under the MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

