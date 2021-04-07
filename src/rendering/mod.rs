//! Pixel iterators used for text rendering.
#[cfg(feature = "ansi")]
mod ansi;
pub(crate) mod cursor;
mod line;
pub(crate) mod line_iter;
pub(crate) mod space_config;

use crate::{
    alignment::{HorizontalTextAlignment, VerticalTextAlignment},
    parser::{Parser, Token},
    rendering::{cursor::Cursor, line::StyledLineRenderer},
    style::{color::Rgb, height_mode::HeightMode},
    StyledTextBox,
};
use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt},
    prelude::{Point, Size},
    primitives::Rectangle,
    text::{CharacterStyle, TextRenderer},
    Drawable,
};

impl<'a, F, A, V, H> Drawable for StyledTextBox<'a, F, A, V, H>
where
    F: TextRenderer<Color = <F as CharacterStyle>::Color> + CharacterStyle + Clone,
    <F as CharacterStyle>::Color: From<Rgb>,
    A: HorizontalTextAlignment,
    V: VerticalTextAlignment,
    H: HeightMode,
{
    type Color = <F as CharacterStyle>::Color;

    #[inline]
    fn draw<D: DrawTarget<Color = Self::Color>>(&self, display: &mut D) -> Result<(), D::Error> {
        let mut cursor = Cursor::new(
            self.text_box.bounds,
            self.style.character_style.line_height(),
            self.style.line_spacing,
            self.style.tab_size.into_pixels(&self.style.character_style),
        );

        V::apply_vertical_alignment(&mut cursor, self);

        let style = &mut self.style.clone();

        let mut carried = None;
        let mut parser = Parser::parse(self.text_box.text);

        while carried.is_some() || !parser.is_empty() {
            let line_cursor = cursor.line();
            let display_range = H::calculate_displayed_row_range(&cursor);
            let display_size = Size::new(cursor.line_width(), display_range.clone().count() as u32);

            // FIXME: cropping isn't necessary for whole lines, but make sure not to blow up the
            // binary size as well.
            let mut display = display.clipped(&Rectangle::new(
                line_cursor.pos() + Point::new(0, display_range.start),
                display_size,
            ));
            StyledLineRenderer::new(&mut parser, line_cursor, style, &mut carried)
                .draw(&mut display)?;

            if carried != Some(Token::CarriageReturn) {
                cursor.new_line();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use embedded_graphics::{
        mock_display::MockDisplay,
        mono_font::{ascii::Font6x9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        prelude::*,
        primitives::Rectangle,
    };

    use crate::{
        alignment::{HorizontalTextAlignment, LeftAligned},
        style::TextBoxStyleBuilder,
        utils::test::size_for,
        TextBox,
    };

    pub fn assert_rendered<A: HorizontalTextAlignment>(
        alignment: A,
        text: &str,
        size: Size,
        pattern: &[&str],
    ) {
        let mut display = MockDisplay::new();

        let character_style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyleBuilder::new()
            .character_style(character_style)
            .alignment(alignment)
            .build();

        TextBox::new(text, Rectangle::new(Point::zero(), size))
            .into_styled(style)
            .draw(&mut display)
            .unwrap();

        display.assert_pattern(pattern);
    }

    #[test]
    fn nbsp_doesnt_break() {
        assert_rendered(
            LeftAligned,
            "a b c\u{a0}d e f",
            size_for(Font6x9, 5, 3),
            &[
                "..................            ",
                ".............#....            ",
                ".............#....            ",
                "..###........###..            ",
                ".#..#........#..#.            ",
                ".#..#........#..#.            ",
                "..###........###..            ",
                "..................            ",
                "..................            ",
                "..............................",
                "................#.............",
                "................#.............",
                "..###.........###.........##..",
                ".#...........#..#........#.##.",
                ".#...........#..#........##...",
                "..###.........###.........###.",
                "..............................",
                "..............................",
                "......                        ",
                "...#..                        ",
                "..#.#.                        ",
                "..#...                        ",
                ".###..                        ",
                "..#...                        ",
                "..#...                        ",
                "......                        ",
                "......                        ",
            ],
        );
    }
}
