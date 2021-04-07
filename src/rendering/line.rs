//! Line rendering.
use core::cell::RefCell;
use core::convert::Infallible;

use crate::{
    alignment::{HorizontalTextAlignment, VerticalTextAlignment},
    parser::{Parser, Token},
    rendering::{cursor::LineCursor, line_iter::LineElementParser},
    style::{color::Rgb, height_mode::HeightMode, TextBoxStyle},
    utils::str_width,
};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    text::{CharacterStyle, TextRenderer},
    Drawable,
};

#[cfg(feature = "ansi")]
use super::ansi::Sgr;
use super::{line_iter::ElementHandler, space_config::UniformSpaceConfig};

#[derive(Debug)]
struct Refs<'a, 'b, F, A, V, H> {
    parser: &'b mut Parser<'a>,
    style: &'b mut TextBoxStyle<F, A, V, H>,
    carried_token: &'b mut Option<Token<'a>>,
}

/// Render a single line of styled text.
#[derive(Debug)]
pub struct StyledLineRenderer<'a, 'b, F, A, V, H> {
    cursor: LineCursor,
    inner: RefCell<Refs<'a, 'b, F, A, V, H>>,
}

impl<'a, 'b, F, A, V, H> StyledLineRenderer<'a, 'b, F, A, V, H>
where
    F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle,
    <F as CharacterStyle>::Color: From<Rgb>,
    H: HeightMode,
{
    /// Creates a new line renderer.
    #[inline]
    pub fn new(
        parser: &'b mut Parser<'a>,
        cursor: LineCursor,
        style: &'b mut TextBoxStyle<F, A, V, H>,
        carried_token: &'b mut Option<Token<'a>>,
    ) -> Self {
        Self {
            cursor,
            inner: RefCell::new(Refs {
                parser,
                style,
                carried_token,
            }),
        }
    }
}

struct RenderElementHandler<'a, F, D> {
    style: &'a mut F,
    display: &'a mut D,
    pos: Point,
}

impl<'a, F, D> ElementHandler for RenderElementHandler<'a, F, D>
where
    F: CharacterStyle + TextRenderer,
    <F as CharacterStyle>::Color: From<Rgb>,
    D: DrawTarget<Color = <F as TextRenderer>::Color>,
{
    type Error = D::Error;

    fn measure(&self, st: &str) -> u32 {
        str_width(self.style, st)
    }

    fn whitespace(&mut self, width: u32) -> Result<(), Self::Error> {
        self.pos = self.style.draw_whitespace(width, self.pos, self.display)?;
        Ok(())
    }

    fn printed_characters(&mut self, st: &str, _: u32) -> Result<(), Self::Error> {
        self.pos = self.style.draw_string(st, self.pos, self.display)?;
        Ok(())
    }

    fn move_cursor(&mut self, by: i32) -> Result<(), Self::Error> {
        // LineElementIterator ensures this new pos is valid.
        self.pos = Point::new(self.pos.x + by, self.pos.y);
        Ok(())
    }

    #[cfg(feature = "ansi")]
    fn sgr(&mut self, sgr: Sgr) -> Result<(), Self::Error> {
        sgr.apply(self.style);
        Ok(())
    }
}

struct StyleOnlyRenderElementHandler<'a, F> {
    style: &'a mut F,
}

impl<'a, F> ElementHandler for StyleOnlyRenderElementHandler<'a, F>
where
    F: CharacterStyle + TextRenderer,
    <F as CharacterStyle>::Color: From<Rgb>,
{
    type Error = Infallible;

    fn measure(&self, st: &str) -> u32 {
        str_width(self.style, st)
    }

    #[cfg(feature = "ansi")]
    fn sgr(&mut self, sgr: Sgr) -> Result<(), Self::Error> {
        sgr.apply(self.style);
        Ok(())
    }
}

impl<F, A, V, H> Drawable for StyledLineRenderer<'_, '_, F, A, V, H>
where
    F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle,
    <F as CharacterStyle>::Color: From<Rgb>,
    A: HorizontalTextAlignment,
    V: VerticalTextAlignment,
    H: HeightMode,
{
    type Color = <F as CharacterStyle>::Color;

    #[inline]
    fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut inner = self.inner.borrow_mut();
        let Refs {
            parser,
            style,
            carried_token,
        } = &mut *inner;

        let renderer = RefCell::new(&mut style.character_style);
        let carried = if display.bounding_box().size.height == 0 {
            // We're outside of the view - no need for a separate measure pass.
            let mut elements = LineElementParser::<'_, '_, _, A>::new(
                parser,
                self.cursor.clone(),
                UniformSpaceConfig::new(&**renderer.borrow()),
                carried_token.clone(),
            );

            elements
                .process(&mut StyleOnlyRenderElementHandler {
                    style: &mut style.character_style,
                })
                .unwrap()
        } else {
            // We have to resort to trickery to figure out the string that is rendered as the line.
            let mut cloned_parser = parser.clone();
            let lm = style.measure_line(
                &mut cloned_parser,
                &mut carried_token.clone(),
                self.cursor.line_width(),
            );

            let consumed_bytes = parser.as_str().len() - cloned_parser.as_str().len();
            let line_str = unsafe { parser.as_str().get_unchecked(..consumed_bytes) };

            let (left, space_config) = A::place_line(line_str, &style.character_style, lm);

            let mut cursor = self.cursor.clone();
            cursor.move_cursor(left as i32).ok();

            let pos = cursor.pos();
            let mut elements = LineElementParser::<'_, '_, _, A>::new(
                parser,
                cursor,
                space_config,
                carried_token.clone(),
            );

            elements.process(&mut RenderElementHandler {
                style: &mut style.character_style,
                display,
                pos,
            })?
        };
        **carried_token = carried;

        Ok(())
    }
}

#[cfg(feature = "ansi")]
impl Sgr {
    fn apply<F>(self, renderer: &mut F)
    where
        F: CharacterStyle,
        <F as CharacterStyle>::Color: From<Rgb>,
    {
        use embedded_graphics::text::DecorationColor;
        match self {
            Sgr::Reset => {
                renderer.set_text_color(None);
                renderer.set_background_color(None);
                renderer.set_underline_color(DecorationColor::None);
                renderer.set_strikethrough_color(DecorationColor::None);
            }
            Sgr::ChangeTextColor(color) => {
                renderer.set_text_color(Some(color.into()));
            }
            Sgr::DefaultTextColor => {
                renderer.set_text_color(None);
            }
            Sgr::ChangeBackgroundColor(color) => {
                renderer.set_background_color(Some(color.into()));
            }
            Sgr::DefaultBackgroundColor => {
                renderer.set_background_color(None);
            }
            Sgr::Underline => {
                renderer.set_underline_color(DecorationColor::TextColor);
            }
            Sgr::UnderlineOff => {
                renderer.set_underline_color(DecorationColor::None);
            }
            Sgr::CrossedOut => {
                renderer.set_strikethrough_color(DecorationColor::TextColor);
            }
            Sgr::NotCrossedOut => {
                renderer.set_strikethrough_color(DecorationColor::None);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        alignment::{HorizontalTextAlignment, VerticalTextAlignment},
        parser::Parser,
        rendering::{cursor::LineCursor, line::StyledLineRenderer},
        style::{color::Rgb, height_mode::HeightMode, TabSize, TextBoxStyle, TextBoxStyleBuilder},
        utils::test::size_for,
    };
    use embedded_graphics::{
        geometry::Point,
        mock_display::MockDisplay,
        mono_font::{ascii::Font6x9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        primitives::Rectangle,
        text::{CharacterStyle, TextRenderer},
        Drawable,
    };

    fn test_rendered_text<'a, F, A, V, H>(
        text: &'a str,
        bounds: Rectangle,
        mut style: TextBoxStyle<F, A, V, H>,
        pattern: &[&str],
    ) where
        F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle,
        <F as CharacterStyle>::Color: From<Rgb> + embedded_graphics::mock_display::ColorMapping,
        A: HorizontalTextAlignment,
        V: VerticalTextAlignment,
        H: HeightMode,
    {
        let mut parser = Parser::parse(text);
        let cursor = LineCursor::new(
            bounds.size.width,
            TabSize::Spaces(4).into_pixels(&style.character_style),
        );
        let mut carried = None;

        let renderer = StyledLineRenderer::new(&mut parser, cursor, &mut style, &mut carried);
        let mut display = MockDisplay::new();
        display.set_allow_overdraw(true);

        renderer.draw(&mut display).unwrap();

        display.assert_pattern(pattern);
    }

    #[test]
    fn simple_render() {
        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .build();

        test_rendered_text(
            "Some sample text",
            Rectangle::new(Point::zero(), size_for(Font6x9, 7, 1)),
            style,
            &[
                "........................",
                "..##....................",
                ".#..#...................",
                "..#.....##..##.#....##..",
                "...#...#..#.#.#.#..#.##.",
                ".#..#..#..#.#.#.#..##...",
                "..##....##..#...#...###.",
                "........................",
                "........................",
            ],
        );
    }

    #[test]
    fn simple_render_nbsp() {
        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .build();

        test_rendered_text(
            "Some\u{A0}sample text",
            Rectangle::new(Point::zero(), size_for(Font6x9, 7, 1)),
            style,
            &[
                "..........................................",
                "..##......................................",
                ".#..#.....................................",
                "..#.....##..##.#....##..........###...###.",
                "...#...#..#.#.#.#..#.##........##....#..#.",
                ".#..#..#..#.#.#.#..##............##..#..#.",
                "..##....##..#...#...###........###....###.",
                "..........................................",
                "..........................................",
            ],
        );
    }

    #[test]
    fn simple_render_first_word_not_wrapped() {
        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .build();

        test_rendered_text(
            "Some sample text",
            Rectangle::new(Point::zero(), size_for(Font6x9, 2, 1)),
            style,
            &[
                "............",
                "..##........",
                ".#..#.......",
                "..#.....##..",
                "...#...#..#.",
                ".#..#..#..#.",
                "..##....##..",
                "............",
                "............",
            ],
        );
    }

    #[test]
    fn newline_stops_render() {
        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .build();

        test_rendered_text(
            "Some \nsample text",
            Rectangle::new(Point::zero(), size_for(Font6x9, 7, 1)),
            style,
            &[
                "........................",
                "..##....................",
                ".#..#...................",
                "..#.....##..##.#....##..",
                "...#...#..#.#.#.#..#.##.",
                ".#..#..#..#.#.#.#..##...",
                "..##....##..#...#...###.",
                "........................",
                "........................",
            ],
        );
    }
}

#[cfg(all(test, feature = "ansi"))]
mod ansi_parser_tests {
    use crate::{
        parser::Parser,
        rendering::{cursor::LineCursor, line::StyledLineRenderer},
        style::{TabSize, TextBoxStyleBuilder},
        utils::test::size_for,
    };
    use embedded_graphics::{
        mock_display::MockDisplay,
        mono_font::{ascii::Font6x9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        Drawable,
    };

    #[test]
    fn ansi_cursor_backwards() {
        let mut display = MockDisplay::new();
        display.set_allow_overdraw(true);

        let mut parser = Parser::parse("foo\x1b[2Dsample");

        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let mut style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .build();

        let cursor = LineCursor::new(
            size_for(Font6x9, 7, 1).width,
            TabSize::Spaces(4).into_pixels(&character_style),
        );
        let mut carried = None;
        StyledLineRenderer::new(&mut parser, cursor, &mut style, &mut carried)
            .draw(&mut display)
            .unwrap();

        display.assert_pattern(&[
            "..........................................",
            "...#...........................##.........",
            "..#.#...........................#.........",
            "..#.....###...###.##.#...###....#.....##..",
            ".###...##....#..#.#.#.#..#..#...#....#.##.",
            "..#......##..#..#.#.#.#..#..#...#....##...",
            "..#....###....###.#...#..###...###....###.",
            ".........................#................",
            ".........................#................",
        ]);
    }
}
