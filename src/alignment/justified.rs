//! Fully justified text
use crate::{
    alignment::TextAlignment,
    parser::Token,
    rendering::{
        cursor::Cursor,
        line::{LineConfiguration, SpaceConfig, StyledLineIterator},
        StateFactory, StyledTextBoxIterator,
    },
    style::StyledTextBox,
    utils::font_ext::{FontExt, LineMeasurement},
};
use embedded_graphics::{drawable::Pixel, fonts::Font, pixelcolor::PixelColor};

/// Marks text to be rendered fully justified
#[derive(Copy, Clone, Debug)]
pub struct Justified;
impl TextAlignment for Justified {}

/// Internal state information used to store width of whitespace characters when rendering fully
/// justified text.
///
/// The fully justified renderer works by calculating the width of whitespace characters for the
/// current line. Due to integer arithmetic, there can be remainder pixels when a single space
/// width is used. This struct stores two width values so the whole line will always (at least if
/// there's a space in the line) take up all available space.
#[derive(Copy, Clone, Debug)]
pub struct JustifiedSpaceConfig {
    /// The width of the whitespace characters
    space_width: u32,

    /// Stores how many characters are rendered using the space_width width. This field changes
    /// during rendering
    space_count: u32,
}

impl JustifiedSpaceConfig {
    #[inline]
    #[must_use]
    fn default<F: Font>() -> Self {
        JustifiedSpaceConfig::new(F::total_char_width(' '), 0)
    }

    #[inline]
    #[must_use]
    fn new(space_width: u32, extra_pixel_count: u32) -> Self {
        JustifiedSpaceConfig {
            space_width,
            space_count: extra_pixel_count,
        }
    }
}

impl SpaceConfig for JustifiedSpaceConfig {
    #[inline]
    fn next_space_width(&mut self) -> u32 {
        if self.space_count == 0 {
            self.space_width
        } else {
            self.space_count -= 1;
            self.space_width + 1
        }
    }

    #[inline]
    fn peek_next_width(&self, whitespace_count: u32) -> u32 {
        whitespace_count * self.space_width + self.space_count.min(whitespace_count)
    }
}

/// State variable used by the fully justified text renderer
#[derive(Debug)]
pub enum JustifiedState<'a, C, F>
where
    C: PixelColor,
    F: Font + Copy,
{
    /// Starts processing a line
    NextLine(Option<Token<'a>>, Cursor<F>),

    /// Renders the processed line
    DrawLine(StyledLineIterator<'a, C, F, JustifiedSpaceConfig>),
}

impl<'a, C, F> StateFactory for StyledTextBox<'a, C, F, Justified>
where
    C: PixelColor,
    F: Font + Copy,
{
    type PixelIteratorState = JustifiedState<'a, C, F>;

    #[inline]
    #[must_use]
    fn create_state(&self) -> Self::PixelIteratorState {
        JustifiedState::NextLine(None, Cursor::new(self.text_box.bounds))
    }
}

impl<C, F> Iterator for StyledTextBoxIterator<'_, C, F, Justified>
where
    C: PixelColor,
    F: Font + Copy,
{
    type Item = Pixel<C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                JustifiedState::NextLine(ref carried_token, ref cursor) => {
                    if !cursor.in_display_area() {
                        break None;
                    }

                    if carried_token.is_none() && self.parser.peek().is_none() {
                        break None;
                    }

                    let max_line_width = cursor.line_width();

                    // initial width is the width of the characters carried over to this row
                    let measurement = if let Some(Token::Word(w)) = carried_token.clone() {
                        F::measure_line(w.chars(), max_line_width)
                    } else {
                        LineMeasurement::empty()
                    };

                    let mut total_width = measurement.width;

                    let mut total_whitespace_count = 0;
                    let mut stretch_line = false;

                    // in some rare cases, the carried over text may not fit into a single line
                    if measurement.fits_line {
                        let mut last_whitespace_width = 0;
                        let mut last_whitespace_count = 0;
                        let mut total_whitespace_width = 0;

                        for token in self.parser.clone() {
                            match token {
                                Token::NewLine => {
                                    break;
                                }

                                Token::Whitespace(_) if total_width == 0 => {
                                    // eat spaces at the start of line
                                }

                                Token::Whitespace(n) => {
                                    last_whitespace_count = n;
                                    last_whitespace_width = (n * F::total_char_width(' '))
                                        .min(max_line_width - total_width);

                                    if total_width + total_whitespace_width + last_whitespace_width
                                        >= max_line_width
                                    {
                                        stretch_line = true;
                                        break;
                                    }
                                }

                                Token::Word(w) => {
                                    let word_measurement = F::measure_line(
                                        w.chars(),
                                        max_line_width
                                            - total_width
                                            - total_whitespace_width
                                            - last_whitespace_width,
                                    );

                                    if !word_measurement.fits_line {
                                        // including the word would wrap the line, stop here instead
                                        stretch_line = true;
                                        break;
                                    }

                                    total_width += word_measurement.width;
                                    total_whitespace_width += last_whitespace_width;
                                    total_whitespace_count += last_whitespace_count;

                                    last_whitespace_count = 0;
                                    last_whitespace_width = 0;
                                }
                            }
                        }
                    }

                    let space_info = if stretch_line && total_whitespace_count != 0 {
                        let total_space_width = max_line_width - total_width;
                        let space_width = total_space_width / total_whitespace_count;
                        let extra_pixels = total_space_width % total_whitespace_count;
                        JustifiedSpaceConfig::new(space_width, extra_pixels)
                    } else {
                        JustifiedSpaceConfig::default::<F>()
                    };

                    self.state = JustifiedState::DrawLine(StyledLineIterator::new(
                        self.parser.clone(),
                        *cursor,
                        LineConfiguration {
                            starting_spaces: false,
                            ending_spaces: false,
                            space_config: space_info,
                        },
                        self.style.text_style,
                        carried_token.clone(),
                    ));
                }

                JustifiedState::DrawLine(ref mut line_iterator) => {
                    if let pixel @ Some(_) = line_iterator.next() {
                        break pixel;
                    }

                    let mut cursor = line_iterator.cursor;
                    cursor.new_line();
                    cursor.carriage_return();
                    self.parser = line_iterator.parser.clone();
                    self.state = JustifiedState::NextLine(line_iterator.remaining_token(), cursor);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use embedded_graphics::{
        fonts::Font6x8, mock_display::MockDisplay, pixelcolor::BinaryColor, prelude::*,
        primitives::Rectangle,
    };

    use crate::{alignment::Justified, style::TextBoxStyleBuilder, TextBox};

    #[test]
    fn simple_render() {
        let mut display = MockDisplay::new();
        let style = TextBoxStyleBuilder::new(Font6x8)
            .alignment(Justified)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        TextBox::new("word", Rectangle::new(Point::zero(), Point::new(54, 54)))
            .into_styled(style)
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "......................#.",
                "......................#.",
                "#...#..###..#.##...##.#.",
                "#...#.#...#.##..#.#..##.",
                "#.#.#.#...#.#.....#...#.",
                "#.#.#.#...#.#.....#...#.",
                ".#.#...###..#......####.",
                "........................",
            ])
        );
    }

    #[test]
    fn simple_word_wrapping() {
        let mut display = MockDisplay::new();
        let style = TextBoxStyleBuilder::new(Font6x8)
            .alignment(Justified)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        TextBox::new(
            "word wrapping",
            Rectangle::new(Point::zero(), Point::new(54, 54)),
        )
        .into_styled(style)
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "......................#.                        ",
                "......................#.                        ",
                "#...#..###..#.##...##.#.                        ",
                "#...#.#...#.##..#.#..##.                        ",
                "#.#.#.#...#.#.....#...#.                        ",
                "#.#.#.#...#.#.....#...#.                        ",
                ".#.#...###..#......####.                        ",
                "........................                        ",
                "................................#...............",
                "................................................",
                "#...#.#.##...###..####..####...##...#.##...####.",
                "#...#.##..#.....#.#...#.#...#...#...##..#.#...#.",
                "#.#.#.#......####.#...#.#...#...#...#...#.#...#.",
                "#.#.#.#.....#...#.####..####....#...#...#..####.",
                ".#.#..#......####.#.....#......###..#...#.....#.",
                "..................#.....#..................###.."
            ])
        );
    }

    #[test]
    fn justified_alignment() {
        let mut display = MockDisplay::new();
        let style = TextBoxStyleBuilder::new(Font6x8)
            .alignment(Justified)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        TextBox::new(
            "word and other word last line",
            Rectangle::new(Point::zero(), Point::new(60, 54)),
        )
        .into_styled(style)
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "......................#....................................#.",
                "......................#....................................#.",
                "#...#..###..#.##...##.#.....................###..#.##...##.#.",
                "#...#.#...#.##..#.#..##........................#.##..#.#..##.",
                "#.#.#.#...#.#.....#...#.....................####.#...#.#...#.",
                "#.#.#.#...#.#.....#...#....................#...#.#...#.#...#.",
                ".#.#...###..#......####.....................####.#...#..####.",
                ".............................................................",
                ".......#....#..............................................#.",
                ".......#....#..............................................#.",
                ".###..###...#.##...###..#.##.........#...#..###..#.##...##.#.",
                "#...#..#....##..#.#...#.##..#........#...#.#...#.##..#.#..##.",
                "#...#..#....#...#.#####.#............#.#.#.#...#.#.....#...#.",
                "#...#..#..#.#...#.#.....#............#.#.#.#...#.#.....#...#.",
                ".###....##..#...#..###..#.............#.#...###..#......####.",
                ".............................................................",
                ".##................#...........##.....#...............       ",
                "..#................#............#.....................       ",
                "..#....###...####.###...........#....##...#.##...###..       ",
                "..#.......#.#......#............#.....#...##..#.#...#.       ",
                "..#....####..###...#............#.....#...#...#.#####.       ",
                "..#...#...#.....#..#..#.........#.....#...#...#.#.....       ",
                ".###...####.####....##.........###...###..#...#..###..       ",
                "......................................................       "
            ])
        );
    }

    #[test]
    fn word_longer_than_line_wraps_word() {
        let mut display = MockDisplay::new();
        let style = TextBoxStyleBuilder::new(Font6x8)
            .alignment(Justified)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        TextBox::new(
            "word somereallylongword",
            Rectangle::new(Point::zero(), Point::new(54, 54)),
        )
        .into_styled(style)
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "......................#.                              ",
                "......................#.                              ",
                "#...#..###..#.##...##.#.                              ",
                "#...#.#...#.##..#.#..##.                              ",
                "#.#.#.#...#.#.....#...#.                              ",
                "#.#.#.#...#.#.....#...#.                              ",
                ".#.#...###..#......####.                              ",
                "........................                              ",
                "...........................................##....##...",
                "............................................#.....#...",
                ".####..###..##.#...###..#.##...###...###....#.....#...",
                "#.....#...#.#.#.#.#...#.##..#.#...#.....#...#.....#...",
                ".###..#...#.#...#.#####.#.....#####..####...#.....#...",
                "....#.#...#.#...#.#.....#.....#.....#...#...#.....#...",
                "####...###..#...#..###..#......###...####..###...###..",
                "......................................................",
                ".......##...........................................#.",
                "........#...........................................#.",
                "#...#...#....###..#.##...####.#...#..###..#.##...##.#.",
                "#...#...#...#...#.##..#.#...#.#...#.#...#.##..#.#..##.",
                "#...#...#...#...#.#...#.#...#.#.#.#.#...#.#.....#...#.",
                ".####...#...#...#.#...#..####.#.#.#.#...#.#.....#...#.",
                "....#..###...###..#...#.....#..#.#...###..#......####.",
                ".###.....................###..........................",
            ])
        );
    }

    #[test]
    fn first_word_longer_than_line_wraps_word() {
        let mut display = MockDisplay::new();
        let style = TextBoxStyleBuilder::new(Font6x8)
            .alignment(Justified)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        TextBox::new(
            "somereallylongword",
            Rectangle::new(Point::zero(), Point::new(54, 54)),
        )
        .into_styled(style)
        .draw(&mut display)
        .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "...........................................##....##...",
                "............................................#.....#...",
                ".####..###..##.#...###..#.##...###...###....#.....#...",
                "#.....#...#.#.#.#.#...#.##..#.#...#.....#...#.....#...",
                ".###..#...#.#...#.#####.#.....#####..####...#.....#...",
                "....#.#...#.#...#.#.....#.....#.....#...#...#.....#...",
                "####...###..#...#..###..#......###...####..###...###..",
                "......................................................",
                ".......##...........................................#.",
                "........#...........................................#.",
                "#...#...#....###..#.##...####.#...#..###..#.##...##.#.",
                "#...#...#...#...#.##..#.#...#.#...#.#...#.##..#.#..##.",
                "#...#...#...#...#.#...#.#...#.#.#.#.#...#.#.....#...#.",
                ".####...#...#...#.#...#..####.#.#.#.#...#.#.....#...#.",
                "....#..###...###..#...#.....#..#.#...###..#......####.",
                ".###.....................###..........................",
            ])
        );
    }
}
