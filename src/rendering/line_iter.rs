//! Line iterator.
//!
//! Turns a token stream into a number of events. A single `LineElementParser` object operates on
//! a single line and is responsible for handling word wrapping, eating leading/trailing whitespace,
//! handling tab characters, soft wrapping characters, non-breaking spaces, etc.
use crate::{
    alignment::HorizontalAlignment,
    parser::{ChangeTextStyle, Parser, Token, SPEC_CHAR_NBSP},
    plugin::{PluginMarker as Plugin, PluginWrapper},
    rendering::{cursor::LineCursor, space_config::SpaceConfig},
};
use az::{SaturatingAs, SaturatingCast};
use embedded_graphics::{pixelcolor::Rgb888, prelude::PixelColor};

use crate::plugin::ansi::utils::try_parse_sgr;
use ansi_parser::AnsiSequence;
use as_slice::AsSlice;

/// Parser to break down a line into primitive elements used by measurement and rendering.
#[derive(Debug)]
#[must_use]
pub(crate) struct LineElementParser<'a, 'b, M, C>
where
    C: PixelColor,
{
    /// Position information.
    pub cursor: LineCursor,

    /// The text to draw.
    parser: &'b mut Parser<'a, C>,

    spaces: SpaceConfig,
    alignment: HorizontalAlignment,
    empty: bool,
    plugin: &'b PluginWrapper<'a, M, C>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndType {
    NewLine,
    CarriageReturn,
    EndOfText,
    LineBreak,
}

pub trait ElementHandler {
    type Error;
    type Color: PixelColor;

    /// Returns the width of the given string in pixels.
    fn measure(&self, st: &str) -> u32;

    /// A whitespace block with the given width.
    fn whitespace(&mut self, _st: &str, _space_count: u32, _width: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// A string of printable characters.
    fn printed_characters(&mut self, _st: &str, _width: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// A cursor movement event.
    fn move_cursor(&mut self, _by: i32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Text style change
    fn change_text_style(
        &mut self,
        _change: ChangeTextStyle<Self::Color>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, 'b, M, C> LineElementParser<'a, 'b, M, C>
where
    C: PixelColor + From<Rgb888>,
    M: Plugin<'a, C>,
{
    /// Creates a new element parser.
    #[inline]
    pub fn new(
        parser: &'b mut Parser<'a, C>,
        plugin: &'b PluginWrapper<'a, M, C>,
        cursor: LineCursor,
        spaces: SpaceConfig,
        alignment: HorizontalAlignment,
    ) -> Self {
        Self {
            parser,
            spaces,
            cursor,
            alignment,
            empty: true,
            plugin,
        }
    }

    fn next_word_width<E: ElementHandler>(&mut self, handler: &E) -> Option<u32> {
        let mut width = None;

        // This looks extremely inefficient.
        let lookahead = self.plugin.clone();
        let mut lookahead_parser = self.parser.clone();

        // We don't want to count the current token.
        lookahead.consume_peeked_token(&mut lookahead_parser);

        'lookahead: loop {
            match lookahead.peek_token(&mut lookahead_parser) {
                Some(Token::Word(w)) => {
                    *width.get_or_insert(0) += handler.measure(w);
                }

                Some(Token::Break(w, _original)) => {
                    *width.get_or_insert(0) += handler.measure(w);

                    break 'lookahead;
                }

                Some(Token::EscapeSequence(_)) => {}

                _ => break 'lookahead,
            }
            lookahead.consume_peeked_token(&mut lookahead_parser);
        }

        width
    }

    fn move_cursor(&mut self, by: i32) -> Result<i32, i32> {
        self.cursor.move_cursor(by)
    }

    fn longest_fitting_substr<E: ElementHandler>(
        &mut self,
        handler: &E,
        w: &'a str,
    ) -> (&'a str, Option<&'a str>) {
        let mut width = 0;
        for (idx, c) in w.char_indices() {
            let char_width = handler.measure(unsafe {
                // SAFETY: we are working on character boundaries
                w.get_unchecked(idx..idx + c.len_utf8())
            });
            if !self.cursor.fits_in_line(width + char_width) {
                return (
                    unsafe {
                        // SAFETY: we are working on character boundaries
                        w.get_unchecked(0..idx)
                    },
                    w.get(idx..),
                );
            }
            width += char_width;
        }

        (w, None)
    }

    fn next_word_fits<E: ElementHandler>(&self, space_width: i32, handler: &mut E) -> bool {
        let mut cursor = self.cursor.clone();
        let mut spaces = self.spaces;

        let mut exit = false;

        // This looks extremely inefficient.
        let lookahead = self.plugin.clone();
        let mut lookahead_parser = self.parser.clone();

        // We don't want to count the current token.
        lookahead.consume_peeked_token(&mut lookahead_parser);

        let _ = cursor.move_cursor(space_width);
        while !exit {
            let width = match lookahead.peek_token(&mut lookahead_parser) {
                Some(Token::Word(w)) | Some(Token::Break(w, _)) => {
                    exit = true;
                    handler.measure(w).saturating_as()
                }

                Some(Token::Whitespace(n, _)) => spaces.consume(n).saturating_as(),
                Some(Token::Tab) => cursor.next_tab_width().saturating_as(),

                Some(Token::EscapeSequence(AnsiSequence::CursorForward(by))) => {
                    (by * handler.measure(" ")).saturating_as()
                }

                Some(Token::EscapeSequence(AnsiSequence::CursorBackward(by))) => {
                    -(by * handler.measure(" ")).saturating_as::<i32>()
                }

                Some(Token::EscapeSequence(_)) => 0,

                _ => return false,
            };

            lookahead.consume_peeked_token(&mut lookahead_parser);
            if cursor.move_cursor(width).is_err() {
                return false;
            }
        }

        true
    }

    fn render_trailing_spaces(&self) -> bool {
        // TODO: make this configurable
        false
    }

    fn render_leading_spaces(&self) -> bool {
        // TODO: make this configurable
        match self.alignment {
            HorizontalAlignment::Left => true,
            HorizontalAlignment::Center => false,
            HorizontalAlignment::Right => false,
            HorizontalAlignment::Justified => false,
        }
    }

    fn draw_whitespace<E: ElementHandler>(
        &mut self,
        handler: &mut E,
        string: &'a str,
        space_count: u32,
        space_width: u32,
    ) -> Result<(), E::Error> {
        if self.empty && !self.render_leading_spaces() {
            handler.whitespace(string, 0, 0)?;
            return Ok(());
        }
        let draw_whitespace = (self.empty && self.render_leading_spaces())
            || self.render_trailing_spaces()
            || self.next_word_fits(space_width.saturating_as(), handler);

        match self.move_cursor(space_width.saturating_cast()) {
            Ok(moved) => {
                let spaces = if draw_whitespace { space_count } else { 0 };
                handler.whitespace(string, spaces, moved.saturating_as())?;
            }

            Err(moved) => {
                let single = space_width / space_count;
                let consumed = moved as u32 / single;
                if consumed > 0 {
                    let (pos, _) = string.char_indices().nth(consumed as usize).unwrap();
                    let (consumed_str, _) = string.split_at(pos);
                    handler.whitespace(consumed_str, consumed, consumed * single)?;

                    self.replace_peeked_token(
                        consumed as usize,
                        Token::Whitespace(consumed, consumed_str),
                    );
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn draw_tab<E: ElementHandler>(
        &mut self,
        handler: &mut E,
        space_width: u32,
    ) -> Result<(), E::Error> {
        if self.empty && !self.render_leading_spaces() {
            return Ok(());
        }

        let draw_whitespace = (self.empty && self.render_leading_spaces())
            || self.render_trailing_spaces()
            || self.next_word_fits(space_width.saturating_as(), handler);

        match self.move_cursor(space_width.saturating_cast()) {
            Ok(moved) if draw_whitespace => handler.whitespace("\t", 1, moved.saturating_as())?,

            Ok(moved) | Err(moved) => {
                handler.move_cursor(moved.saturating_as())?;
            }
        }
        Ok(())
    }

    fn peek_next_token(&mut self) -> Option<Token<'a, C>> {
        self.plugin.peek_token(&mut self.parser)
    }

    fn consume_token(&mut self) {
        self.plugin.consume_peeked_token(&mut self.parser);
    }

    fn replace_peeked_token(&mut self, len: usize, token: Token<'a, C>) {
        self.plugin.replace_peeked_token(len, token);
    }

    #[inline]
    pub fn process<E: ElementHandler<Color = C>>(
        &mut self,
        handler: &mut E,
    ) -> Result<LineEndType, E::Error> {
        while let Some(token) = self.peek_next_token() {
            match token {
                Token::Whitespace(n, seq) => {
                    let space_width = self.spaces.consume(n);
                    self.draw_whitespace(handler, seq, n, space_width)?;
                }

                Token::Tab => {
                    let space_width = self.cursor.next_tab_width();
                    self.draw_tab(handler, space_width)?;
                }

                Token::Break(c, _original) => {
                    if let Some(word_width) = self.next_word_width(handler) {
                        if !self.cursor.fits_in_line(word_width) || self.empty {
                            // this line is done, decide how to end

                            // If the next Word token does not fit the line, display break character
                            let width = handler.measure(c);
                            if self.move_cursor(width.saturating_as()).is_ok() {
                                if let Some(Token::Break(c, _)) = self.plugin.render_token(token) {
                                    handler.printed_characters(c, width)?;
                                }
                                self.consume_token();
                            }

                            if !self.empty {
                                return Ok(LineEndType::LineBreak);
                            }
                        }
                    } else {
                        // Next token is not a Word, consume Break and continue
                    }
                }

                Token::Word(w) => {
                    let width = handler.measure(w);
                    let (word, remainder) = if self.move_cursor(width.saturating_as()).is_ok() {
                        // We can move the cursor here since `process_word()`
                        // doesn't depend on it.
                        (w, None)
                    } else if self.empty {
                        // This word does not fit into an empty line. Find longest part
                        // that fits and push the rest to the next line.
                        match self.longest_fitting_substr(handler, w) {
                            ("", _) => {
                                // Weird case where width doesn't permit drawing anything.
                                // End here to prevent infinite looping.
                                self.consume_token();
                                return Ok(LineEndType::LineBreak);
                            }
                            other => other,
                        }
                    } else {
                        // word wrapping - push this word to the next line
                        return Ok(LineEndType::LineBreak);
                    };

                    self.empty = false;

                    if let Some(Token::Word(word)) = self.plugin.render_token(Token::Word(word)) {
                        self.process_word(handler, word)?;
                    }

                    if remainder.is_some() {
                        // Consume what was printed.
                        self.replace_peeked_token(word.len(), Token::Word(word));
                        self.consume_token();
                        return Ok(LineEndType::LineBreak);
                    }
                }

                Token::EscapeSequence(seq) => {
                    match seq {
                        AnsiSequence::SetGraphicsMode(vec) => {
                            if let Some(sgr) = try_parse_sgr(vec.as_slice()) {
                                handler.change_text_style(sgr.into())?;
                            }
                        }

                        AnsiSequence::CursorForward(n) => {
                            // Cursor movement can't rely on the text, as it's permitted
                            // to move the cursor outside of the current line.
                            // Example:
                            // (| denotes the cursor, [ and ] are the limits of the line):
                            // [Some text|    ]
                            // Cursor forward 2 characters
                            // [Some text  |  ]
                            let delta = (n * handler.measure(" ")).saturating_as();
                            match self.move_cursor(delta) {
                                Ok(delta) | Err(delta) => {
                                    handler.whitespace("", 1, delta.saturating_as())?;
                                }
                            }
                        }

                        AnsiSequence::CursorBackward(n) => {
                            // The above poses an issue with variable-width fonts.
                            // If cursor movement ignores the variable width, the cursor
                            // will be placed in positions other than glyph boundaries.
                            let delta = -(n * handler.measure(" ")).saturating_as::<i32>();
                            match self.move_cursor(delta) {
                                Ok(delta) | Err(delta) => {
                                    handler.move_cursor(delta)?;
                                    handler.whitespace("", 1, delta.abs().saturating_as())?;
                                    handler.move_cursor(delta)?;
                                }
                            }
                        }

                        _ => {
                            // ignore for now
                        }
                    }
                }

                Token::ChangeTextStyle(change) => handler.change_text_style(change)?,

                Token::CarriageReturn => {
                    handler.whitespace("\r", 0, 0)?;
                    self.consume_token();
                    return Ok(LineEndType::CarriageReturn);
                }

                Token::NewLine => {
                    handler.whitespace("\n", 0, 0)?;
                    self.consume_token();
                    return Ok(LineEndType::NewLine);
                }
            }
            self.consume_token();
        }

        Ok(LineEndType::EndOfText)
    }

    fn process_word<E: ElementHandler>(
        &mut self,
        handler: &mut E,
        w: &str,
    ) -> Result<(), E::Error> {
        match w.char_indices().find(|(_, c)| *c == SPEC_CHAR_NBSP) {
            Some((space_pos, _)) => {
                // If we have anything before the space...
                if space_pos != 0 {
                    let word = unsafe {
                        // Safety: space_pos must be a character boundary
                        w.get_unchecked(0..space_pos)
                    };
                    handler.printed_characters(word, handler.measure(word))?;
                }

                handler.whitespace("\u{a0}", 1, self.spaces.consume(1))?;

                // If we have anything after the space...
                if let Some(word) = w.get(space_pos + SPEC_CHAR_NBSP.len_utf8()..) {
                    return self.process_word(handler, word);
                }
            }

            None => {
                handler.printed_characters(w, handler.measure(w))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use std::convert::Infallible;

    use super::*;
    use crate::{
        plugin::{NoPlugin, PluginMarker as Plugin, PluginWrapper},
        rendering::{cursor::Cursor, space_config::SpaceConfig},
        style::TabSize,
        utils::{str_width, test::size_for},
    };
    use embedded_graphics::{
        geometry::{Point, Size},
        mono_font::{ascii::FONT_6X9, MonoTextStyle},
        pixelcolor::BinaryColor,
        primitives::Rectangle,
        text::{renderer::TextRenderer, LineHeight},
    };

    #[derive(PartialEq, Eq, Debug)]
    pub enum RenderElement<C: PixelColor> {
        Space(u32, bool),
        String(String, u32),
        MoveCursor(i32),
        ChangeTextStyle(ChangeTextStyle<C>),
    }

    impl<C: PixelColor> RenderElement<C> {
        pub fn string(st: &str, width: u32) -> Self {
            Self::String(st.to_owned(), width)
        }
    }

    struct TestElementHandler<F>
    where
        F: TextRenderer,
    {
        elements: Vec<RenderElement<F::Color>>,
        style: F,
    }

    impl<F> TestElementHandler<F>
    where
        F: TextRenderer,
    {
        fn new(style: F) -> Self {
            Self {
                elements: vec![],
                style,
            }
        }
    }

    impl<'el, F: TextRenderer> ElementHandler for TestElementHandler<F> {
        type Error = Infallible;
        type Color = F::Color;

        fn measure(&self, st: &str) -> u32 {
            str_width(&self.style, st)
        }

        fn whitespace(&mut self, _string: &str, count: u32, width: u32) -> Result<(), Self::Error> {
            self.elements
                .push(RenderElement::Space(width, (count > 0) as bool));
            Ok(())
        }

        fn printed_characters(&mut self, st: &str, width: u32) -> Result<(), Self::Error> {
            self.elements
                .push(RenderElement::String(st.to_owned(), width));
            Ok(())
        }

        fn move_cursor(&mut self, by: i32) -> Result<(), Self::Error> {
            self.elements.push(RenderElement::MoveCursor(by));
            Ok(())
        }

        fn change_text_style(
            &mut self,
            change: ChangeTextStyle<Self::Color>,
        ) -> Result<(), Self::Error> {
            self.elements.push(RenderElement::ChangeTextStyle(change));
            Ok(())
        }
    }

    #[track_caller]
    pub(crate) fn assert_line_elements<'a, M>(
        parser: &mut Parser<'a, Rgb888>,
        max_chars: u32,
        elements: &[RenderElement<Rgb888>],
        plugin: &PluginWrapper<'a, M, Rgb888>,
    ) where
        M: Plugin<'a, Rgb888>,
    {
        let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On.into());

        let config = SpaceConfig::new_from_renderer(&style);
        let cursor = Cursor::new(
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, max_chars, 1)),
            style.line_height(),
            LineHeight::Percent(100),
            TabSize::Spaces(4).into_pixels(&style),
        )
        .line();

        let mut handler = TestElementHandler::new(style);
        let mut line1 =
            LineElementParser::new(parser, plugin, cursor, config, HorizontalAlignment::Left);

        line1.process(&mut handler).unwrap();

        assert_eq!(handler.elements, elements);
    }

    #[test]
    fn insufficient_width_no_looping() {
        let mut parser = Parser::parse("foobar");

        let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On.into());

        let config = SpaceConfig::new_from_renderer(&style);
        let cursor = Cursor::new(
            Rectangle::new(Point::zero(), size_for(&FONT_6X9, 1, 1) - Size::new(1, 0)),
            style.line_height(),
            LineHeight::Percent(100),
            TabSize::Spaces(4).into_pixels(&style),
        )
        .line();

        let mut handler = TestElementHandler::new(style);
        let mut mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());
        let mut line1 = LineElementParser::new(
            &mut parser,
            &mut mw,
            cursor,
            config,
            HorizontalAlignment::Left,
        );

        line1.process(&mut handler).unwrap();

        assert_eq!(handler.elements, &[]);
    }

    #[test]
    fn soft_hyphen_no_wrapping() {
        let mut parser = Parser::parse("sam\u{00AD}ple");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            6,
            &[
                RenderElement::string("sam", 18),
                RenderElement::string("ple", 18),
            ],
            &mw,
        );
    }

    #[test]
    fn soft_hyphen() {
        let mut parser = Parser::parse("sam\u{00AD}ple");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            5,
            &[
                RenderElement::string("sam", 18),
                RenderElement::string("-", 6),
            ],
            &mw,
        );
        assert_line_elements(&mut parser, 5, &[RenderElement::string("ple", 18)], &mw);
    }

    #[test]
    fn soft_hyphen_wrapped() {
        let mut parser = Parser::parse("sam\u{00AD}mm");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(&mut parser, 3, &[RenderElement::string("sam", 18)], &mw);
        assert_line_elements(
            &mut parser,
            3,
            &[
                RenderElement::string("-", 6),
                RenderElement::string("mm", 12),
            ],
            &mw,
        );
    }

    #[test]
    fn nbsp_issue() {
        let mut parser = Parser::parse("a b c\u{a0}d e f");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            5,
            &[
                RenderElement::string("a", 6),
                RenderElement::Space(6, true),
                RenderElement::string("b", 6),
                RenderElement::Space(6, false),
            ],
            &mw,
        );
        assert_line_elements(
            &mut parser,
            5,
            &[
                RenderElement::string("c", 6),
                RenderElement::Space(6, true),
                RenderElement::string("d", 6),
                RenderElement::Space(6, true),
                RenderElement::string("e", 6),
            ],
            &mw,
        );
        assert_line_elements(&mut parser, 5, &[RenderElement::string("f", 6)], &mw);
    }

    #[test]
    fn soft_hyphen_issue_42() {
        let mut parser =
            Parser::parse("super\u{AD}cali\u{AD}fragi\u{AD}listic\u{AD}espeali\u{AD}docious");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(&mut parser, 5, &[RenderElement::string("super", 30)], &mw);
        assert_line_elements(
            &mut parser,
            5,
            &[
                RenderElement::string("-", 6),
                RenderElement::string("cali", 24),
            ],
            &mw,
        );
    }

    #[test]
    fn nbsp_is_rendered_as_space() {
        let mut parser = Parser::parse("glued\u{a0}words");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            50,
            &[
                RenderElement::string("glued", 30),
                RenderElement::Space(6, true),
                RenderElement::string("words", 30),
            ],
            &mw,
        );
    }

    #[test]
    fn tabs() {
        let mut parser = Parser::parse("a\tword\nand\t\tanother\t");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(
            &mut parser,
            16,
            &[
                RenderElement::string("a", 6),
                RenderElement::Space(6 * 3, true),
                RenderElement::string("word", 24),
                RenderElement::Space(0, false), // the newline
            ],
            &mw,
        );
        assert_line_elements(
            &mut parser,
            16,
            &[
                RenderElement::string("and", 18),
                RenderElement::Space(6, true),
                RenderElement::Space(6 * 4, true),
                RenderElement::string("another", 42),
                RenderElement::MoveCursor(6),
            ],
            &mw,
        );
    }

    #[test]
    fn cursor_limit() {
        let mut parser = Parser::parse("Some sample text");
        let mw = PluginWrapper::new(NoPlugin::<Rgb888>::new());

        assert_line_elements(&mut parser, 2, &[RenderElement::string("So", 12)], &mw);
    }
}
