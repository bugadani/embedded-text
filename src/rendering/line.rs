//! Line rendering.
use core::convert::Infallible;

use crate::{
    parser::{ChangeTextStyle, Parser},
    plugin::{PluginMarker as Plugin, PluginWrapper, ProcessingState},
    rendering::{
        cursor::LineCursor,
        line_iter::{LineElementParser, LineEndType},
    },
    style::TextBoxStyle,
    utils::str_width,
};
use az::SaturatingAs;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    pixelcolor::{BinaryColor, Rgb888},
    prelude::{PixelColor, Size},
    primitives::Rectangle,
    text::{
        renderer::{CharacterStyle, TextRenderer},
        Baseline, DecorationColor,
    },
    Drawable,
};

use super::{line_iter::ElementHandler, space_config::SpaceConfig};

impl<C> ChangeTextStyle<C>
where
    C: PixelColor + From<Rgb888>,
{
    pub(crate) fn apply<S: CharacterStyle<Color = C>>(self, style: &mut S) {
        match self {
            ChangeTextStyle::Reset => {
                style.set_text_color(Some(Into::<Rgb888>::into(BinaryColor::On).into()));
                style.set_background_color(None);
                style.set_underline_color(DecorationColor::None);
                style.set_strikethrough_color(DecorationColor::None);
            }
            ChangeTextStyle::TextColor(color) => style.set_text_color(color),
            ChangeTextStyle::BackgroundColor(color) => style.set_background_color(color),
            ChangeTextStyle::Underline(color) => style.set_underline_color(color),
            ChangeTextStyle::Strikethrough(color) => style.set_strikethrough_color(color),
        }
    }
}

/// Render a single line of styled text.
pub(crate) struct StyledLineRenderer<'a, 'b, S, M>
where
    S: TextRenderer + Clone,
    M: Plugin<'a, <S as TextRenderer>::Color>,
{
    cursor: LineCursor,
    state: LineRenderState<'a, 'b, S, M>,
}

#[derive(Clone)]
pub(crate) struct LineRenderState<'a, 'b, S, M>
where
    S: TextRenderer + Clone,
    M: Plugin<'a, S::Color>,
{
    pub parser: Parser<'a, S::Color>,
    pub character_style: S,
    pub style: TextBoxStyle,
    pub end_type: LineEndType,
    pub plugin: &'b PluginWrapper<'a, M, S::Color>,
}

impl<'a, 'b, F, M> StyledLineRenderer<'a, 'b, F, M>
where
    F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle,
    <F as CharacterStyle>::Color: From<Rgb888>,
    M: Plugin<'a, <F as TextRenderer>::Color>,
{
    /// Creates a new line renderer.
    pub fn new(cursor: LineCursor, state: LineRenderState<'a, 'b, F, M>) -> Self {
        Self { cursor, state }
    }
}

struct RenderElementHandler<'a, 'b, F, D, M>
where
    F: TextRenderer,
    D: DrawTarget<Color = F::Color>,
{
    style: &'b mut F,
    display: &'b mut D,
    pos: Point,
    plugin: &'b PluginWrapper<'a, M, F::Color>,
}

impl<'a, 'b, 'c, F, D, M> ElementHandler for RenderElementHandler<'a, 'c, F, D, M>
where
    F: CharacterStyle + TextRenderer,
    <F as CharacterStyle>::Color: From<Rgb888>,
    D: DrawTarget<Color = <F as TextRenderer>::Color>,
    M: Plugin<'a, <F as TextRenderer>::Color>,
{
    type Error = D::Error;
    type Color = <F as CharacterStyle>::Color;

    fn measure(&self, st: &str) -> u32 {
        str_width(self.style, st)
    }

    fn whitespace(&mut self, st: &str, space_count: u32, width: u32) -> Result<(), Self::Error> {
        let top_left = self.pos;
        if space_count > 0 {
            self.pos = self
                .style
                .draw_whitespace(width, self.pos, Baseline::Top, self.display)?;
        } else {
            self.pos += Point::new(width.saturating_as(), 0);
        }

        let size = Size::new(width, self.style.line_height().saturating_as());
        let bounds = Rectangle::new(top_left, size);

        self.plugin
            .post_render(self.display, self.style, st, bounds)?;

        Ok(())
    }

    fn printed_characters(&mut self, st: &str, width: u32) -> Result<(), Self::Error> {
        let top_left = self.pos;
        self.style
            .draw_string(st, self.pos, Baseline::Top, self.display)?;

        self.pos += Point::new(width.saturating_as(), 0);

        let size = Size::new(width, self.style.line_height().saturating_as());
        let bounds = Rectangle::new(top_left, size);

        self.plugin
            .post_render(self.display, self.style, st, bounds)?;

        Ok(())
    }

    fn move_cursor(&mut self, by: i32) -> Result<(), Self::Error> {
        // LineElementIterator ensures this new pos is valid.
        self.pos = Point::new(self.pos.x + by, self.pos.y);
        Ok(())
    }

    #[cfg(feature = "ansi")]
    fn change_text_style(
        &mut self,
        change: ChangeTextStyle<<F as CharacterStyle>::Color>,
    ) -> Result<(), Self::Error> {
        change.apply(self.style);
        Ok(())
    }
}

struct StyleOnlyRenderElementHandler<'a, F> {
    style: &'a mut F,
}

impl<'a, F> ElementHandler for StyleOnlyRenderElementHandler<'a, F>
where
    F: CharacterStyle + TextRenderer,
    <F as CharacterStyle>::Color: From<Rgb888>,
{
    type Error = Infallible;
    type Color = <F as CharacterStyle>::Color;

    fn measure(&self, st: &str) -> u32 {
        str_width(self.style, st)
    }

    #[cfg(feature = "ansi")]
    fn change_text_style(
        &mut self,
        change: ChangeTextStyle<<F as CharacterStyle>::Color>,
    ) -> Result<(), Self::Error> {
        change.apply(self.style);
        Ok(())
    }
}

impl<'a, 'b, F, M> Drawable for StyledLineRenderer<'a, 'b, F, M>
where
    F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle,
    <F as CharacterStyle>::Color: From<Rgb888>,
    M: Plugin<'a, <F as TextRenderer>::Color> + Plugin<'a, <F as CharacterStyle>::Color>,
{
    type Color = <F as CharacterStyle>::Color;
    type Output = LineRenderState<'a, 'b, F, M>;

    #[inline]
    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let LineRenderState {
            mut parser,
            mut character_style,
            style,
            plugin,
            ..
        } = self.state.clone();

        let lm = {
            // Ensure the clone lives for as short as possible.
            let mut cloned_parser = parser.clone();
            let measure_plugin = plugin.clone();
            measure_plugin.set_state(ProcessingState::Measure);
            style.measure_line(
                &measure_plugin,
                &character_style,
                &mut cloned_parser,
                self.cursor.line_width(),
            )
        };

        let (end_type, end_pos) = if display.bounding_box().size.height == 0 {
            // We're outside of the view. Use simpler render element handler and space config.
            let mut elements = LineElementParser::new(
                &mut parser,
                plugin,
                self.cursor.clone(),
                SpaceConfig::new_from_renderer(&character_style),
                style.alignment,
            );

            let end_type = elements
                .process(&mut StyleOnlyRenderElementHandler {
                    style: &mut character_style,
                })
                .unwrap();

            (end_type, elements.cursor.pos())
        } else {
            let (left, space_config) = style.alignment.place_line(&character_style, lm);

            let mut cursor = self.cursor.clone();
            cursor.move_cursor(left.saturating_as()).ok();

            let pos = cursor.pos();
            let mut elements =
                LineElementParser::new(&mut parser, plugin, cursor, space_config, style.alignment);

            let end_type = elements.process(&mut RenderElementHandler {
                style: &mut character_style,
                display,
                pos,
                plugin,
            })?;

            (end_type, elements.cursor.pos())
        };

        let next_state = LineRenderState {
            parser,
            character_style,
            style,
            end_type,
            plugin,
        };

        if next_state.end_type == LineEndType::EndOfText {
            next_state.plugin.post_render(
                display,
                &next_state.character_style,
                "",
                Rectangle::new(
                    end_pos,
                    Size::new(0, next_state.character_style.line_height()),
                ),
            )?;
        }

        Ok(next_state)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        parser::Parser,
        plugin::{NoPlugin, PluginWrapper},
        rendering::{
            cursor::LineCursor,
            line::{LineRenderState, StyledLineRenderer},
            line_iter::LineEndType,
        },
        style::{TabSize, TextBoxStyle, TextBoxStyleBuilder},
        utils::test::size_for,
    };
    use embedded_graphics::{
        geometry::Point,
        mock_display::MockDisplay,
        mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
        pixelcolor::{BinaryColor, Rgb888},
        primitives::Rectangle,
        text::renderer::{CharacterStyle, TextRenderer},
        Drawable,
    };

    fn test_rendered_text<'a, S>(
        text: &'a str,
        bounds: Rectangle,
        character_style: S,
        style: TextBoxStyle,
        pattern: &[&str],
    ) where
        S: TextRenderer<Color = <S as CharacterStyle>::Color> + CharacterStyle,
        <S as CharacterStyle>::Color: From<Rgb888> + embedded_graphics::mock_display::ColorMapping,
    {
        let parser = Parser::parse(text);
        let cursor = LineCursor::new(
            bounds.size.width,
            TabSize::Spaces(4).into_pixels(&character_style),
        );

        let plugin = PluginWrapper::new(NoPlugin::new());

        let state = LineRenderState {
            parser,
            character_style,
            style,
            end_type: LineEndType::EndOfText,
            plugin: &plugin,
        };

        let renderer = StyledLineRenderer::new(cursor, state);
        let mut display = MockDisplay::new();
        display.set_allow_overdraw(true);

        renderer.draw(&mut display).unwrap();

        display.assert_pattern(pattern);
    }

    #[test]
    fn simple_render() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new().build();

        test_rendered_text(
            "Some sample text",
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, 7, 1)),
            character_style,
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
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new().build();

        test_rendered_text(
            "Some\u{A0}sample text",
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, 7, 1)),
            character_style,
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
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new().build();

        test_rendered_text(
            "Some sample text",
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, 2, 1)),
            character_style,
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
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new().build();

        test_rendered_text(
            "Some \nsample text",
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, 7, 1)),
            character_style,
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
        plugin::{NoPlugin, PluginWrapper},
        rendering::{
            cursor::LineCursor,
            line::{LineRenderState, StyledLineRenderer},
            line_iter::LineEndType,
        },
        style::{TabSize, TextBoxStyleBuilder},
        utils::test::size_for,
    };
    use embedded_graphics::{
        mock_display::MockDisplay,
        mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        Drawable,
    };

    #[test]
    fn ansi_cursor_backwards() {
        let mut display = MockDisplay::new();
        display.set_allow_overdraw(true);

        let parser = Parser::parse("foo\x1b[2Dsample");

        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new().build();

        let cursor = LineCursor::new(
            size_for(&FONT_6X9, 7, 1).width,
            TabSize::Spaces(4).into_pixels(&character_style),
        );

        let plugin = PluginWrapper::new(NoPlugin::new());
        let state = LineRenderState {
            parser,
            character_style,
            style,
            end_type: LineEndType::EndOfText,
            plugin: &plugin,
        };
        StyledLineRenderer::new(cursor, state)
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
