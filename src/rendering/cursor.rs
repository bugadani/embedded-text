//! Cursor to track rendering position.
use embedded_graphics::{geometry::Point, primitives::Rectangle, text::LineHeight};

use az::{SaturatingAs, SaturatingCast};

/// Tracks position within a line.
#[derive(Debug, Clone)]
pub struct LineCursor {
    start: Point,
    width: u32,
    position: u32,
    tab_width: u32,
}

impl LineCursor {
    /// Creates a new object whose position isn't important.
    pub fn new(width: u32, tab_width: u32) -> Self {
        Self {
            start: Point::zero(),
            width,
            tab_width,
            position: 0,
        }
    }

    pub fn pos(&self) -> Point {
        self.start + Point::new(self.position.saturating_as(), 0)
    }

    /// Returns the distance to the next tab position.
    pub fn next_tab_width(&self) -> u32 {
        let next_tab_pos = (self.position / self.tab_width + 1) * self.tab_width;
        next_tab_pos - self.position
    }

    /// Returns the width of the text box.
    pub fn line_width(&self) -> u32 {
        self.width
    }

    /// Returns whether the current line has enough space to also include an object of given width.
    pub fn fits_in_line(&self, width: u32) -> bool {
        width <= self.space()
    }

    /// Returns the amount of empty space in the line.
    pub fn space(&self) -> u32 {
        self.width - self.position
    }

    /// Moves the cursor by a given amount.
    pub fn move_cursor(&mut self, by: i32) -> Result<i32, i32> {
        if by < 0 {
            let abs = by.abs() as u32;
            if abs <= self.position {
                self.position -= abs;
                Ok(by)
            } else {
                Err(-self.position.saturating_as::<i32>())
            }
        } else {
            let space = self.space().saturating_cast();
            if by <= space {
                // Here we know by > 0, cast is safe
                self.position += by as u32;
                Ok(by)
            } else {
                Err(space)
            }
        }
    }
}

/// Internal structure that keeps track of position information while rendering a [`TextBox`].
///
/// [`TextBox`]: crate::TextBox
#[derive(Copy, Clone, Debug)]
pub struct Cursor {
    /// Current cursor position
    pub y: i32,

    /// TextBox bounding rectangle
    bounds: Rectangle,

    line_height: i32,
    line_spacing: i32,
    tab_width: u32,
}

impl Cursor {
    /// Creates a new `Cursor` object located at the top left of the given bounding [`Rectangle`].
    #[inline]
    #[must_use]
    pub fn new(
        bounds: Rectangle,
        base_line_height: u32,
        line_height: LineHeight,
        tab_width: u32,
    ) -> Self {
        Self {
            y: bounds.top_left.y,
            line_height: base_line_height.saturating_as(),
            line_spacing: line_height.to_absolute(base_line_height).saturating_as(),
            bounds,
            tab_width,
        }
    }

    #[must_use]
    pub(crate) fn line(&self) -> LineCursor {
        LineCursor {
            start: Point::new(self.bounds.top_left.x, self.y),
            width: self.bounds.size.width,
            position: 0,
            tab_width: self.tab_width,
        }
    }

    /// Returns the coordinates of the bottom right corner.
    #[inline]
    pub fn bottom_right(&self) -> Point {
        self.bounds.bottom_right().unwrap_or(self.bounds.top_left)
    }

    /// Returns the coordinates of the bottom right corner.
    #[inline]
    pub fn top_left(&self) -> Point {
        self.bounds.top_left
    }

    /// Returns the width of the text box.
    #[inline]
    pub fn line_width(&self) -> u32 {
        self.bounds.size.width
    }

    /// Returns the height of a line.
    #[inline]
    pub fn line_height(&self) -> i32 {
        self.line_height
    }

    /// Starts a new line.
    #[inline]
    pub fn new_line(&mut self) {
        self.y += self.line_spacing;
    }

    /// Returns whether the cursor is completely in the bounding box.
    ///
    /// Completely means, that the line that is marked by the cursor can be drawn without any
    /// vertical clipping or drawing outside the bounds.
    ///
    /// *Note:* Only vertical overrun is checked.
    #[inline]
    #[must_use]
    pub fn in_display_area(&self) -> bool {
        self.bounds.top_left.y <= self.y && self.y <= self.bottom_right().y - self.line_height + 1
    }
}
