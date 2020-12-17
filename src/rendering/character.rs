//! Character rendering.
use crate::utils::font_ext::FontExt;
use core::{marker::PhantomData, ops::Range};
use embedded_graphics::{prelude::*, style::MonoTextStyle};

/// Represents a glyph (a symbol) to be drawn.
#[derive(Copy, Clone, Debug)]
pub struct Glyph<F: MonoFont> {
    _font: PhantomData<F>,
    char_glyph_offset: u32,
}

impl<F> Glyph<F>
where
    F: MonoFont,
{
    /// Creates a glyph from a character.
    #[inline]
    #[must_use]
    pub fn new(c: char) -> Self {
        let char_offset = F::char_offset(c);
        let char_per_row = F::FONT_IMAGE_WIDTH / F::CHARACTER_SIZE.width;

        // Top left corner of character, in pixels.
        let char_x = char_offset % char_per_row * F::CHARACTER_SIZE.width;
        let char_y = char_offset / char_per_row * F::CHARACTER_SIZE.height;

        Self {
            _font: PhantomData,
            char_glyph_offset: char_x + char_y * F::FONT_IMAGE_WIDTH,
        }
    }

    /// Returns the value of a given point:
    ///  * `true` for foreground pixels
    ///  * `false` for background pixels
    #[inline]
    #[must_use]
    pub fn point(&self, p: Point) -> bool {
        // Bit index
        // = X pixel offset for char
        // + Character row offset (row 0 = 0, row 1 = (192 * 8) = 1536)
        // + X offset for the pixel block that comprises this char
        // + Y offset for pixel block
        let bitmap_bit_index =
            self.char_glyph_offset + p.x as u32 + p.y as u32 * F::FONT_IMAGE_WIDTH;

        let bitmap_byte = bitmap_bit_index / 8;
        let bitmap_bit = bitmap_bit_index % 8;

        F::FONT_IMAGE[bitmap_byte as usize] & (0x80 >> bitmap_bit) != 0
    }
}

/// Pixel iterator to render a single styled character.
///
/// This struct may be used to implement custom rendering algorithms. Internally, this pixel
/// iterator is used by [`StyledLinePixelIterator`] to render characters.
///
/// [`StyledLinePixelIterator`]: ../line/struct.StyledLinePixelIterator.html
#[derive(Clone, Debug)]
pub struct CharacterIterator<C, F>
where
    C: PixelColor,
    F: MonoFont,
{
    character: Glyph<F>,
    style: MonoTextStyle<C, F>,
    pos: Point,
    char_walk: Point,
    max_coordinates: Point,
    underline: bool,
    strikethrough: bool,
}

impl<C, F> CharacterIterator<C, F>
where
    C: PixelColor,
    F: MonoFont,
{
    /// Creates a new pixel iterator to draw the given character.
    #[inline]
    #[must_use]
    pub fn new(
        character: char,
        pos: Point,
        style: MonoTextStyle<C, F>,
        rows: Range<i32>,
        underline: bool,
        strikethrough: bool,
    ) -> Self {
        let mut max_height = (F::CHARACTER_SIZE.height as i32).min(rows.end);
        if underline {
            // adjust height if whole character is displayed for underline
            if rows.end == max_height {
                max_height += 1;
            }
        }
        Self {
            character: Glyph::new(character),
            style,
            pos,
            char_walk: Point::new(0, rows.start),
            max_coordinates: Point::new(F::CHARACTER_SIZE.width as i32 - 1, max_height),
            underline,
            strikethrough,
        }
    }
}

impl<C, F> Iterator for CharacterIterator<C, F>
where
    C: PixelColor,
    F: MonoFont,
{
    type Item = Pixel<C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.char_walk.y >= self.max_coordinates.y {
                // Done with this char, move on to the next one
                break None;
            }
            let pos = self.char_walk;

            if pos.x < self.max_coordinates.x {
                self.char_walk.x += 1;
            } else {
                self.char_walk.x = 0;
                self.char_walk.y += 1;
            }

            let is_underline = self.underline && pos.y as u32 == F::CHARACTER_SIZE.height;
            let is_strikethrough = self.strikethrough && pos.y as u32 == F::strikethrough_pos();

            let color = if is_underline || is_strikethrough || self.character.point(pos) {
                self.style.text_color
            } else {
                self.style.background_color
            };

            // Skip to next point if pixel is transparent
            if let Some(color) = color {
                let p = self.pos + pos;
                break Some(Pixel(p, color));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::CharacterIterator;
    use embedded_graphics::{
        fonts::Font6x8, mock_display::MockDisplay, pixelcolor::BinaryColor, prelude::*,
        style::MonoTextStyleBuilder,
    };

    #[test]
    fn transparent_char() {
        let mut display = MockDisplay::new();
        let style = MonoTextStyleBuilder::new()
            .font(Font6x8)
            .background_color(BinaryColor::On)
            .build();

        CharacterIterator::new(
            'A',
            Point::zero(),
            style,
            0..Font6x8::CHARACTER_SIZE.height as i32,
            false,
            false,
        )
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "#   ##   ",
                " ### #   ",
                " ### #   ",
                "     #   ",
                " ### #   ",
                " ### #   ",
                " ### #   ",
                "######   "
            ])
        );
    }

    #[test]
    fn partial_draw() {
        let mut display = MockDisplay::new();
        let style = MonoTextStyleBuilder::new()
            .font(Font6x8)
            .background_color(BinaryColor::On)
            .build();

        CharacterIterator::new(
            'A',
            Point::zero(),
            style,
            2..Font6x8::CHARACTER_SIZE.height as i32 - 2,
            false,
            false,
        )
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "         ",
                "         ",
                " ### #   ",
                "     #   ",
                " ### #   ",
                " ### #   ",
                "         ",
                "         "
            ])
        );
    }
}
