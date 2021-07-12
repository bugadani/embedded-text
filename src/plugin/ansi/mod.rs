//! Ansi sequence support plugin

use embedded_graphics::prelude::PixelColor;

use crate::{plugin::Plugin, Token};

pub mod utils;

/// Ansi sequence parser plugin.
#[derive(Clone)]
pub struct Ansi<'a, C: PixelColor> {
    carry: Option<Token<'a, C>>,
}

impl<C: PixelColor> Ansi<'_, C> {
    /// Returns a new plugin object.
    #[inline]
    pub fn new() -> Self {
        Self { carry: None }
    }
}

impl<'a, C: PixelColor> Plugin<'a, C> for Ansi<'a, C> {
    fn next_token(
        &mut self,
        mut next_token: impl FnMut() -> Option<crate::Token<'a, C>>,
    ) -> Option<crate::Token<'a, C>> {
        let token = if let Some(token) = self.carry.take() {
            Some(token)
        } else {
            next_token()
        };

        if let Some(Token::Word(text)) = token {
            let mut chars = text.char_indices();

            match chars.find(|(_, c)| *c == '\u{1b}') {
                Some((0, _)) => match ansi_parser::parse_escape(text) {
                    Ok((string, output)) => {
                        self.carry = Some(Token::Word(string));
                        Some(Token::EscapeSequence(output))
                    }
                    Err(_) => {
                        self.carry = Some(Token::Word(chars.as_str()));
                        Some(Token::Word("\u{1b}"))
                    }
                },

                Some((idx, _)) => {
                    // Escape character is not the first one. Strip and return the prefix.
                    let (pre, rem) = text.split_at(idx);

                    self.carry = Some(Token::Word(rem));

                    Some(Token::Word(pre))
                }

                None => {
                    // No escape character.
                    Some(Token::Word(text))
                }
            }
        } else {
            // Not a word token.
            token
        }
    }
}

#[cfg(test)]
mod test {
    use embedded_graphics::{
        mock_display::MockDisplay,
        mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
        pixelcolor::{BinaryColor, Rgb888},
        Drawable,
    };

    use crate::{
        alignment::HorizontalAlignment,
        parser::Parser,
        plugin::{ansi::Ansi, PluginWrapper},
        rendering::{
            cursor::LineCursor,
            line::{LineRenderState, StyledLineRenderer},
            line_iter::{
                test::{assert_line_elements, RenderElement},
                LineEndType,
            },
        },
        style::{TabSize, TextBoxStyleBuilder},
        utils::test::size_for,
        ChangeTextStyle,
    };

    #[test]

    fn test_measure_line_cursor_back() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();

        let mut text = Parser::parse("123\x1b[2D");

        let mut plugin = PluginWrapper::new(Ansi::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut text,
            5 * FONT_6X9.character_size.width,
        );
        assert_eq!(lm.width, 3 * FONT_6X9.character_size.width);

        // Now a case where the string itself without rewind is wider than the line and the
        // continuation after rewind extends the line.
        let mut text = Parser::parse("123\x1b[2D456");

        let mut plugin = PluginWrapper::new(Ansi::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut text,
            5 * FONT_6X9.character_size.width,
        );
        assert_eq!(lm.width, 4 * FONT_6X9.character_size.width);
    }

    #[test]
    fn colors() {
        let mut parser = Parser::parse("Lorem \x1b[92mIpsum");
        let mw = PluginWrapper::new(Ansi::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            100,
            &[
                RenderElement::string("Lorem", 30),
                RenderElement::Space(6, true),
                RenderElement::ChangeTextStyle(ChangeTextStyle::TextColor(Some(Rgb888::new(
                    22, 198, 12,
                )))),
                RenderElement::string("Ipsum", 30),
            ],
            &mw,
        );
    }

    #[test]
    fn ansi_code_does_not_break_word() {
        let mut parser = Parser::parse("Lorem foo\x1b[92mbarum");
        let mw = PluginWrapper::new(Ansi::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            8,
            &[
                RenderElement::string("Lorem", 30),
                RenderElement::Space(6, false),
            ],
            &mw,
        );

        assert_line_elements(
            &mut parser,
            8,
            &[
                RenderElement::string("foo", 18),
                RenderElement::ChangeTextStyle(ChangeTextStyle::TextColor(Some(Rgb888::new(
                    22, 198, 12,
                )))),
                RenderElement::string("barum", 30),
            ],
            &mw,
        );
    }

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

        let plugin = PluginWrapper::new(Ansi::new());
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
