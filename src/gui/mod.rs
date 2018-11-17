use crate::{Color, Display, PartialRefresh};

use core::cmp::max;

pub mod font;
pub mod rle;

pub trait GUIElement {
    fn resize(&mut self, width: u32, height: u32);
    fn min_size(&self) -> (u32, u32);
    fn size(&self) -> (u32, u32);

    fn render_line<DisplayType>(&self, display: &mut DisplayType, y: i32, mut left: i32, right: i32)
    where
        DisplayType: Display,
    {
        if right <= left {
            return;
        }
        if right <= 0 {
            return;
        }
        if y < 0 {
            display.fill((right - left) as u32, Color::White);
        } else if y as u32 >= self.size().1 {
            display.fill((right - left) as u32, Color::White);
        } else {
            if left < 0 {
                display.fill(-left as u32, Color::White);
                left = 0;
            }
            let y = y as u32;
            let left = left as u32;
            let right = right as u32;
            if right > self.size().0 {
                self.render_line_clipped(display, y, left, self.size().0);
                display.fill(right - self.size().0, Color::White);
            } else {
                self.render_line_clipped(display, y, left, right);
            }
        }
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display;
}

pub struct Layout<Root>
where
    Root: GUIElement,
{
    root: Root,
    width: u32,
    height: u32,
}

impl<Root> Layout<Root>
where
    Root: GUIElement,
{
    pub fn new(width: u32, height: u32, mut root: Root) -> Layout<Root> {
        // TODO: Derive width/height from display constants!
        root.resize(width, height);
        return Layout {
            root: root,
            width: width,
            height: height,
        };
    }

    pub fn render<DisplayType>(&self, display: &mut DisplayType)
    where
        DisplayType: Display,
    {
        for i in 0..self.height {
            self.root
                .render_line(display, i as i32, 0, self.width as i32);
        }
    }

    pub fn render_partial<DisplayType>(
        &self,
        display: &mut DisplayType,
        left: u32,
        top: u32,
        right: u32,
        bottom: u32,
    ) where
        DisplayType: Display + PartialRefresh,
    {
        // TODO: Check whether right/bottom are smaller than width/height?
        for i in top..bottom {
            self.root
                .render_line(display, i as i32, left as i32, right as i32);
        }
    }
}

enum HorizontalSplitMode {
    ExpandLeft(u32),
    ExpandRight(u32),
}

pub struct HorizontalSplit<Left, Right>
where
    Left: GUIElement,
    Right: GUIElement,
{
    mode: HorizontalSplitMode,
    left: Left,
    right: Right,
    width: u32,
    height: u32,
}

impl<Left, Right> HorizontalSplit<Left, Right>
where
    Left: GUIElement,
    Right: GUIElement,
{
    pub fn expand_left(split_at: u32, left: Left, right: Right) -> HorizontalSplit<Left, Right> {
        HorizontalSplit {
            mode: HorizontalSplitMode::ExpandLeft(split_at),
            left: left,
            right: right,
            width: 0,
            height: 0,
        }
    }

    pub fn expand_right(split_at: u32, left: Left, right: Right) -> HorizontalSplit<Left, Right> {
        HorizontalSplit {
            mode: HorizontalSplitMode::ExpandRight(split_at),
            left: left,
            right: right,
            width: 0,
            height: 0,
        }
    }
}

impl<Left, Right> GUIElement for HorizontalSplit<Left, Right>
where
    Left: GUIElement,
    Right: GUIElement,
{
    fn resize(&mut self, width: u32, height: u32) {
        self.width = max(width, self.min_size().0);
        self.height = height;
        let (left_width, right_width) = match self.mode {
            HorizontalSplitMode::ExpandLeft(split_at) => (self.width - split_at, split_at),
            HorizontalSplitMode::ExpandRight(split_at) => (split_at, self.width - split_at),
        };
        self.left.resize(left_width, height);
        self.right.resize(right_width, height);
    }

    fn min_size(&self) -> (u32, u32) {
        (
            match self.mode {
                HorizontalSplitMode::ExpandLeft(split_at) => self.left.min_size().0 + split_at,
                HorizontalSplitMode::ExpandRight(split_at) => split_at + self.right.min_size().0,
            },
            max(self.left.min_size().1, self.right.min_size().1),
        )
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        let split_at = match self.mode {
            HorizontalSplitMode::ExpandLeft(split_at) => self.width - split_at,
            HorizontalSplitMode::ExpandRight(split_at) => split_at,
        };
        if split_at >= right {
            // Only draw left part.
            self.left
                .render_line(display, y as i32, left as i32, right as i32);
        } else if split_at <= left {
            // Only draw right part.
            self.right.render_line(
                display,
                y as i32,
                (left - split_at) as i32,
                (right - split_at) as i32,
            );
        } else {
            self.left
                .render_line(display, y as i32, left as i32, split_at as i32);
            self.right
                .render_line(display, y as i32, 0, (right - split_at) as i32);
        }
    }
}

enum VerticalSplitMode {
    ExpandTop(u32),
    ExpandBottom(u32),
}

pub struct VerticalSplit<Top, Bottom>
where
    Top: GUIElement,
    Bottom: GUIElement,
{
    mode: VerticalSplitMode,
    top: Top,
    bottom: Bottom,
    width: u32,
    height: u32,
}

impl<Top, Bottom> VerticalSplit<Top, Bottom>
where
    Top: GUIElement,
    Bottom: GUIElement,
{
    pub fn expand_top(split_at: u32, top: Top, bottom: Bottom) -> VerticalSplit<Top, Bottom> {
        VerticalSplit {
            mode: VerticalSplitMode::ExpandTop(split_at),
            top: top,
            bottom: bottom,
            width: 0,
            height: 0,
        }
    }

    pub fn expand_bottom(split_at: u32, top: Top, bottom: Bottom) -> VerticalSplit<Top, Bottom> {
        VerticalSplit {
            mode: VerticalSplitMode::ExpandBottom(split_at),
            top: top,
            bottom: bottom,
            width: 0,
            height: 0,
        }
    }
}

impl<Top, Bottom> GUIElement for VerticalSplit<Top, Bottom>
where
    Top: GUIElement,
    Bottom: GUIElement,
{
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = max(height, self.min_size().1);
        let (top_height, bottom_height) = match self.mode {
            VerticalSplitMode::ExpandTop(split_at) => (self.height - split_at, split_at),
            VerticalSplitMode::ExpandBottom(split_at) => (split_at, self.height - split_at),
        };
        self.top.resize(width, top_height);
        self.bottom.resize(width, bottom_height);
    }

    fn min_size(&self) -> (u32, u32) {
        (
            max(self.top.min_size().0, self.bottom.min_size().0),
            match self.mode {
                VerticalSplitMode::ExpandTop(split_at) => self.top.min_size().1 + split_at,
                VerticalSplitMode::ExpandBottom(split_at) => split_at + self.bottom.min_size().1,
            },
        )
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        let split_at = match self.mode {
            VerticalSplitMode::ExpandTop(split_at) => self.height - split_at,
            VerticalSplitMode::ExpandBottom(split_at) => split_at,
        };
        if y < split_at {
            self.top
                .render_line(display, y as i32, left as i32, right as i32);
        } else {
            self.bottom
                .render_line(display, (y - split_at) as i32, left as i32, right as i32);
        }
    }
}

pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

pub struct Align<Element>
where
    Element: GUIElement,
{
    horizontal: HorizontalAlign,
    vertical: VerticalAlign,
    element: Element,
    width: u32,
    height: u32,
}

impl<Element> Align<Element>
where
    Element: GUIElement,
{
    pub fn new(
        horizontal: HorizontalAlign,
        vertical: VerticalAlign,
        element: Element,
    ) -> Align<Element> {
        Align {
            horizontal: horizontal,
            vertical: vertical,
            element: element,
            width: 0,
            height: 0,
        }
    }
}

impl<Element> GUIElement for Align<Element>
where
    Element: GUIElement,
{
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.element.resize(width, height);
    }

    fn min_size(&self) -> (u32, u32) {
        (0, 0)
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        let (element_width, element_height) = self.element.size();
        let x_offset = match self.horizontal {
            HorizontalAlign::Left => 0,
            HorizontalAlign::Center => (self.width as i32 - element_width as i32) / 2,
            HorizontalAlign::Right => (self.width as i32 - element_width as i32),
        };
        let y_offset = match self.vertical {
            VerticalAlign::Top => 0,
            VerticalAlign::Center => (self.height as i32 - element_height as i32) / 2,
            VerticalAlign::Bottom => (self.height as i32 - element_height as i32),
        };
        self.element.render_line(
            display,
            y as i32 - y_offset,
            left as i32 - x_offset,
            right as i32 - x_offset,
        );
    }
}

pub struct Fill {
    color: Color,
    width: u32,
    height: u32,
}

impl Fill {
    pub fn new(color: Color) -> Fill {
        Fill {
            color: color,
            width: 0,
            height: 0,
        }
    }
}

impl GUIElement for Fill {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn min_size(&self) -> (u32, u32) {
        (1, 1)
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        _y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        display.fill(right - left, self.color);
    }
}

pub struct Text {
    text: &'static str,
    font: &'static font::Font,
    width: u32,
    height: u32,
}

impl Text {
    pub fn new(text: &'static str, font: &'static font::Font) -> Text {
        let (width, height) = font.get_text_size(text);
        Text {
            text: text,
            font: font,
            width: width,
            height: height,
        }
    }
}

impl GUIElement for Text {
    fn resize(&mut self, width: u32, height: u32) {
        // Ignore, as the font dictates the size of the text.
    }

    fn min_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn render_line_clipped<DisplayType>(
        &self,
        display: &mut DisplayType,
        y: u32,
        left: u32,
        right: u32,
    ) where
        DisplayType: Display,
    {
        self.font.render_line(display, self.text, y, left, right);
    }
}
