//! Display the last lines of the text.

use embedded_graphics::{
    prelude::PixelColor,
    text::renderer::{CharacterStyle, TextRenderer},
};

use crate::{plugin::Plugin, rendering::cursor::Cursor, TextBoxProperties};

/// Text tail display plugin.
///
/// Aligns the last line of the text to be always visible. If the text fits inside the text box,
/// it will be top aligned. If the text is longer, it will be bottom aligned.
#[derive(Clone)]
pub struct Tail;

impl<'a, C: PixelColor> Plugin<'a, C> for Tail {
    fn on_start_render<S: CharacterStyle + TextRenderer>(
        &mut self,
        cursor: &mut Cursor,
        props: &TextBoxProperties<'_, S>,
    ) {
        if props.text_height > props.box_height {
            let offset = props.box_height - props.text_height;

            cursor.y += offset
        }
    }
}

#[cfg(test)]
mod test {
    use embedded_graphics::{
        mock_display::MockDisplay,
        mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        prelude::{Point, Size},
        primitives::Rectangle,
        Drawable,
    };

    use crate::{plugin::tail::Tail, style::TextBoxStyle, utils::test::size_for, TextBox};

    #[track_caller]
    pub fn assert_rendered(text: &str, size: Size, pattern: &[&str]) {
        let mut display = MockDisplay::new();

        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        let style = TextBoxStyle::default();

        TextBox::with_textbox_style(
            text,
            Rectangle::new(Point::zero(), size),
            character_style,
            style,
        )
        .add_plugin(Tail)
        .draw(&mut display)
        .unwrap();

        display.assert_pattern(pattern);
    }

    #[test]
    fn scrolling_behaves_as_top_if_lines_dont_overflow() {
        assert_rendered(
            "word",
            size_for(&FONT_6X9, 4, 2),
            &[
                "........................",
                "......................#.",
                "......................#.",
                "#...#...##...#.#....###.",
                "#.#.#..#..#..##.#..#..#.",
                "#.#.#..#..#..#.....#..#.",
                ".#.#....##...#......###.",
                "........................",
                "........................",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
                "                        ",
            ],
        );
    }

    #[test]
    fn scrolling_behaves_as_bottom_if_lines_overflow() {
        assert_rendered(
            "word word2 word3 word4",
            size_for(&FONT_6X9, 5, 2),
            &[
                "..............................",
                "......................#..####.",
                "......................#....#..",
                "#...#...##...#.#....###...##..",
                "#.#.#..#..#..##.#..#..#.....#.",
                "#.#.#..#..#..#.....#..#.....#.",
                ".#.#....##...#......###..###..",
                "..............................",
                "..............................",
                "..............................",
                "......................#....#..",
                "......................#...##..",
                "#...#...##...#.#....###..#.#..",
                "#.#.#..#..#..##.#..#..#.#..#..",
                "#.#.#..#..#..#.....#..#.#####.",
                ".#.#....##...#......###....#..",
                "..............................",
                "..............................",
            ],
        );
    }
}
