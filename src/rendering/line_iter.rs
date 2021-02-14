//! Line iterator.
//!
//! Provide elements (spaces or characters) to render as long as they fit in the current line
use crate::{
    alignment::HorizontalTextAlignment,
    parser::{Parser, Token, SPEC_CHAR_NBSP},
    rendering::cursor::Cursor,
};
use core::marker::PhantomData;
use embedded_graphics::geometry::Point;

#[cfg(feature = "ansi")]
use super::ansi::{try_parse_sgr, Sgr};
use super::space_config::SpaceConfig;
#[cfg(feature = "ansi")]
use ansi_parser::AnsiSequence;
#[cfg(feature = "ansi")]
use as_slice::AsSlice;

/// Internal state used to render a line.
#[derive(Debug)]
enum State<'a> {
    /// Decide what to do next.
    ProcessToken(Token<'a>),

    FirstWord(&'a str),
    Word(&'a str),

    /// Signal that the renderer has finished.
    Done,
}

/// What to draw
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderElement<'a> {
    /// Render a whitespace block with the given width and count
    Space(u32, u32),

    /// Render the given character
    PrintedCharacters(&'a str),

    /// Move the cursor
    #[cfg(feature = "ansi")]
    MoveCursor(i32),

    /// A Select Graphic Rendition code
    #[cfg(feature = "ansi")]
    Sgr(Sgr),
}

/// Parser to break down a line into primitive elements used by measurement and rendering.
#[derive(Debug)]
pub struct LineElementParser<'a, 'b, M, SP, A> {
    /// Position information.
    pub cursor: Cursor,

    /// The text to draw.
    pub parser: &'b mut Parser<'a>,

    pub(crate) pos: Point,
    current_token: State<'a>,
    config: SP,
    first_word: bool,
    alignment: PhantomData<A>,
    carried_token: &'b mut Option<Token<'a>>,
    measure: M,
}

impl<'a, 'b, M, SP, A> LineElementParser<'a, 'b, M, SP, A>
where
    SP: SpaceConfig,
    A: HorizontalTextAlignment,
    M: Fn(&str) -> u32,
{
    /// Creates a new element parser.
    #[inline]
    #[must_use]
    pub fn new(
        parser: &'b mut Parser<'a>,
        cursor: Cursor,
        config: SP,
        carried_token: &'b mut Option<Token<'a>>,
        measure: M,
    ) -> Self {
        let current_token = carried_token
            .take() // forget the old carried token
            .filter(|t| ![Token::NewLine, Token::CarriageReturn, Token::Break(None)].contains(t))
            .or_else(|| parser.next())
            .map_or(State::Done, State::ProcessToken);

        Self {
            parser,
            current_token,
            config,
            cursor,
            first_word: true,
            alignment: PhantomData,
            pos: Point::zero(),
            measure,
            carried_token,
        }
    }

    fn next_token(&mut self) {
        match self.parser.next() {
            None => self.finish_end_of_string(),
            Some(t) => self.current_token = State::ProcessToken(t),
        }
    }

    fn finish_end_of_string(&mut self) {
        self.current_token = State::Done;
    }

    fn finish_wrapped(&mut self) {
        self.finish(Token::Break(None));
    }

    fn finish(&mut self, t: Token<'a>) {
        self.carried_token.replace(t);
        self.current_token = State::Done;
    }

    fn next_word_width(&mut self) -> Option<u32> {
        let mut width = None;
        let mut lookahead = self.parser.clone();

        'lookahead: loop {
            match lookahead.next() {
                Some(Token::Word(w)) => {
                    let w = self.str_width(w);

                    width = width.map_or(Some(w), |acc| Some(acc + w));
                }

                Some(Token::Break(Some(c))) => {
                    let w = self.str_width(c);
                    width = width.map_or(Some(w), |acc| Some(acc + w));
                    break 'lookahead;
                }

                #[cfg(feature = "ansi")]
                Some(Token::EscapeSequence(_)) => {}

                _ => break 'lookahead,
            }
        }

        width
    }

    fn str_width(&self, s: &str) -> u32 {
        let measure = &self.measure;
        measure(s)
    }

    fn count_widest_space_seq(&self, n: u32) -> u32 {
        // we could also binary search but I don't think it's worth it
        let mut spaces_to_render = 0;
        let available = self.cursor.space();
        while spaces_to_render < n && self.config.peek_next_width(spaces_to_render + 1) < available
        {
            spaces_to_render += 1;
        }

        spaces_to_render
    }

    fn advance(&mut self, by: u32) -> Result<u32, u32> {
        self.cursor.advance(by)
    }

    fn advance_unchecked(&mut self, by: u32) {
        self.cursor.advance_unchecked(by);
    }

    #[cfg(feature = "ansi")]
    fn move_cursor(&mut self, by: i32) {
        // FIXME: clean this up
        if by < 0 {
            if !self.cursor.rewind(by.abs() as u32) {
                self.cursor.carriage_return();
            }
        } else {
            let _ = self.advance(by as u32);
        }
    }
}

impl<'a, M, SP, A> Iterator for LineElementParser<'a, '_, M, SP, A>
where
    SP: SpaceConfig,
    A: HorizontalTextAlignment,
    M: Fn(&str) -> u32,
{
    type Item = RenderElement<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.pos = self.cursor.position;
            match core::mem::replace(&mut self.current_token, State::Done) {
                // No token being processed, get next one
                State::ProcessToken(ref token) => {
                    let token = token.clone();
                    match token {
                        Token::Whitespace(n) => {
                            // This mess decides if we want to render whitespace at all.
                            // The current horizontal alignment can ignore spaces at the beginning
                            // and end of a line.
                            let mut would_wrap = false;
                            let render_whitespace = if self.first_word {
                                if A::STARTING_SPACES {
                                    self.first_word = false;
                                }
                                A::STARTING_SPACES
                            } else if let Some(word_width) = self.next_word_width() {
                                // Check if space + w fits in line, otherwise it's up to config
                                let space_width = self.config.peek_next_width(n);
                                let fits = self.cursor.fits_in_line(space_width + word_width);

                                would_wrap = !fits;

                                A::ENDING_SPACES || fits
                            } else {
                                A::ENDING_SPACES
                            };

                            if render_whitespace {
                                // take as many spaces as possible and save the rest in state
                                let n = if would_wrap { n.saturating_sub(1) } else { n };
                                let spaces_to_render = self.count_widest_space_seq(n);

                                if spaces_to_render > 0 {
                                    let space_width = self.config.consume(spaces_to_render);
                                    self.advance_unchecked(space_width);
                                    let carried = n - spaces_to_render;

                                    if carried == 0 {
                                        self.next_token();
                                    } else {
                                        // n > 0 only if not every space was rendered
                                        self.finish(Token::Whitespace(carried));
                                    }

                                    return Some(RenderElement::Space(
                                        space_width,
                                        spaces_to_render,
                                    ));
                                } else {
                                    // there are spaces to render but none fit the line
                                    // eat one as a newline and stop
                                    if n > 1 {
                                        self.finish(Token::Whitespace(n - 1));
                                    } else {
                                        self.finish_wrapped();
                                    }
                                }
                            } else if would_wrap {
                                self.finish_wrapped();
                            } else {
                                // nothing, process next token
                                self.next_token();
                            }
                        }

                        Token::Break(c) => {
                            let fits = if let Some(word_width) = self.next_word_width() {
                                self.cursor.fits_in_line(word_width)
                            } else {
                                // Next token is not a Word, consume Break and continue
                                true
                            };

                            if fits {
                                self.next_token();
                            } else if let Some(c) = c {
                                // If a Break contains a character, display it if the next
                                // Word token does not fit the line.
                                if self.advance(self.str_width(c)).is_ok() {
                                    self.finish_wrapped();
                                    return Some(RenderElement::PrintedCharacters(c));
                                } else {
                                    // this line is done
                                    self.finish(Token::Word(c));
                                }
                            } else {
                                // this line is done
                                self.finish_wrapped();
                            }
                        }

                        Token::Word(w) => {
                            // FIXME: this isn't exactly optimal when outside of the display area
                            if self.cursor.fits_in_line(self.str_width(w)) {
                                self.first_word = false;
                                self.current_token = State::Word(w);
                            } else if self.first_word {
                                self.first_word = false;
                                self.current_token = State::FirstWord(w);
                            } else {
                                self.finish(token);
                            }
                        }

                        Token::Tab => {
                            let sp_width = self.cursor.next_tab_width();

                            let tab_width = match self.advance(sp_width) {
                                Ok(width) => {
                                    self.next_token();
                                    width
                                }
                                Err(width) => {
                                    // If we can't render the whole tab since we don't fit in the line,
                                    // render it using all the available space - it will be < tab size.
                                    self.finish_wrapped();
                                    width
                                }
                            };

                            // don't count tabs as spaces
                            return Some(RenderElement::Space(tab_width, 0));
                        }

                        #[cfg(feature = "ansi")]
                        Token::EscapeSequence(seq) => {
                            self.next_token();
                            match seq {
                                AnsiSequence::SetGraphicsMode(vec) => {
                                    if let Some(sgr) = try_parse_sgr(vec.as_slice()) {
                                        return Some(RenderElement::Sgr(sgr));
                                    }
                                }

                                AnsiSequence::CursorForward(n) => {
                                    let delta = (n * self.str_width(" ")) as i32;
                                    self.move_cursor(delta);
                                    return Some(RenderElement::MoveCursor(delta));
                                }

                                AnsiSequence::CursorBackward(n) => {
                                    let delta = -((n * self.str_width(" ")) as i32);
                                    self.move_cursor(delta);
                                    return Some(RenderElement::MoveCursor(delta));
                                    // no spaces rendered here
                                }

                                _ => {
                                    // ignore for now
                                }
                            }
                        }

                        Token::NewLine | Token::CarriageReturn => {
                            // we're done
                            self.finish(token);
                        }
                    }
                }

                State::Word(w) => {
                    // need to update the space config
                    if let Some((space_pos, _)) =
                        w.char_indices().find(|(_, c)| *c == SPEC_CHAR_NBSP)
                    {
                        if space_pos == 0 {
                            if let Some(word) = w.get(SPEC_CHAR_NBSP.len_utf8()..) {
                                self.current_token = State::Word(word);
                            } else {
                                self.next_token();
                            }
                            let sp_width = self.config.consume(1);

                            self.advance_unchecked(sp_width);
                            return Some(RenderElement::Space(sp_width, 1));
                        } else {
                            let word = unsafe { w.get_unchecked(0..space_pos) };
                            self.current_token =
                                State::Word(unsafe { w.get_unchecked(space_pos..) });

                            self.advance_unchecked(self.str_width(word));
                            return Some(RenderElement::PrintedCharacters(word));
                        }
                    } else {
                        self.next_token();

                        self.advance_unchecked(self.str_width(w));
                        return Some(RenderElement::PrintedCharacters(w));
                    }
                }

                State::FirstWord(w) => {
                    let mut start_idx = 0;
                    let mut width = 0;
                    for c in w.chars() {
                        let end_idx = start_idx + c.len_utf8();

                        let char_width = if c == SPEC_CHAR_NBSP {
                            self.config.peek_next_width(1)
                        } else {
                            let c_str = unsafe { w.get_unchecked(start_idx..end_idx) };
                            self.str_width(c_str)
                        };

                        if self.cursor.fits_in_line(width + char_width) {
                            // We return the non-breaking space as a different render element
                            if c == SPEC_CHAR_NBSP {
                                return if start_idx == 0 {
                                    // we have peeked the space width, now consume it
                                    self.config.consume(1);

                                    // here, width == 0 so don't need to add
                                    self.advance_unchecked(char_width);

                                    if let Some(word) = w.get(SPEC_CHAR_NBSP.len_utf8()..) {
                                        self.current_token = State::FirstWord(word);
                                    } else {
                                        self.next_token();
                                    }

                                    Some(RenderElement::Space(char_width, 1))
                                } else {
                                    // we know the previous characters fit in the line
                                    self.advance_unchecked(width);

                                    // New state starts with the current space
                                    self.current_token =
                                        State::FirstWord(unsafe { w.get_unchecked(start_idx..) });

                                    Some(RenderElement::PrintedCharacters(unsafe {
                                        w.get_unchecked(..start_idx)
                                    }))
                                };
                            }
                            width += char_width;
                        } else {
                            // `word` does not fit into the space - this can happen for first words
                            // in this case, we return the widest we can and carry the rest

                            return if start_idx == 0 {
                                // Weird case where width doesn't permit drawing anything.
                                // Consume token to avoid infinite loop.
                                self.finish_end_of_string();
                                None
                            } else {
                                // This can happen because words can be longer than the line itself.
                                self.advance_unchecked(width);
                                // `start_idx` is actually the end of the substring that fits
                                self.finish(Token::Word(unsafe { w.get_unchecked(start_idx..) }));
                                Some(RenderElement::PrintedCharacters(unsafe {
                                    w.get_unchecked(..start_idx)
                                }))
                            };
                        }

                        start_idx = end_idx;
                    }

                    self.next_token();
                    self.advance_unchecked(width);
                    return Some(RenderElement::PrintedCharacters(w));
                }

                State::Done => return None,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        alignment::LeftAligned,
        rendering::space_config::UniformSpaceConfig,
        style::TabSize,
        utils::{str_width, test::size_for},
    };
    use embedded_graphics::{
        mono_font::{ascii::Font6x9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        primitives::Rectangle,
        text::TextRenderer,
    };

    pub fn assert_line_elements<'a>(
        parser: &mut Parser<'a>,
        carried: &mut Option<Token<'a>>,
        max_chars: u32,
        elements: &[RenderElement],
    ) {
        let style = MonoTextStyleBuilder::new()
            .font(Font6x9)
            .text_color(BinaryColor::On)
            .build();

        let config = UniformSpaceConfig::new(&style);
        let cursor = Cursor::new(
            Rectangle::new(Point::zero(), size_for(Font6x9, max_chars, 1)),
            style.line_height(),
            0,
            TabSize::Spaces(4).into_pixels(&style),
        );

        let line1: LineElementParser<'_, '_, _, _, LeftAligned> =
            LineElementParser::new(parser, cursor, config, carried, |s| str_width(&style, s));

        assert_eq!(line1.into_iter().collect::<Vec<_>>(), elements);
    }

    #[test]
    fn soft_hyphen_no_wrapping() {
        let mut parser = Parser::parse("sam\u{00AD}ple");
        let mut carried = None;

        assert_line_elements(
            &mut parser,
            &mut carried,
            6,
            &[
                RenderElement::PrintedCharacters("sam"),
                RenderElement::PrintedCharacters("ple"),
            ],
        );
    }

    #[test]
    fn soft_hyphen() {
        let mut parser = Parser::parse("sam\u{00AD}ple");
        let mut carried = None;

        assert_line_elements(
            &mut parser,
            &mut carried,
            5,
            &[
                RenderElement::PrintedCharacters("sam"),
                RenderElement::PrintedCharacters("-"),
            ],
        );
        assert_line_elements(
            &mut parser,
            &mut carried,
            5,
            &[RenderElement::PrintedCharacters("ple")],
        );
    }

    #[test]
    fn soft_hyphen_issue_42() {
        let mut parser =
            Parser::parse("super\u{AD}cali\u{AD}fragi\u{AD}listic\u{AD}espeali\u{AD}docious");
        let mut carried = None;

        assert_line_elements(
            &mut parser,
            &mut carried,
            5,
            &[RenderElement::PrintedCharacters("super")],
        );
        assert_line_elements(
            &mut parser,
            &mut carried,
            5,
            &[
                RenderElement::PrintedCharacters("-"),
                RenderElement::PrintedCharacters("cali"),
            ],
        );
    }

    #[test]
    fn nbsp_is_rendered_as_space() {
        let mut parser = Parser::parse("glued\u{a0}words");

        assert_line_elements(
            &mut parser,
            &mut None,
            50,
            &[
                RenderElement::PrintedCharacters("glued"),
                RenderElement::Space(6, 1),
                RenderElement::PrintedCharacters("words"),
            ],
        );
    }

    #[test]
    fn tabs() {
        let mut parser = Parser::parse("a\tword\nand\t\tanother\t");
        let mut carried = None;

        assert_line_elements(
            &mut parser,
            &mut carried,
            16,
            &[
                RenderElement::PrintedCharacters("a"),
                RenderElement::Space(6 * 3, 0),
                RenderElement::PrintedCharacters("word"),
            ],
        );
        assert_line_elements(
            &mut parser,
            &mut carried,
            16,
            &[
                RenderElement::PrintedCharacters("and"),
                RenderElement::Space(6, 0),
                RenderElement::Space(6 * 4, 0),
                RenderElement::PrintedCharacters("another"),
                RenderElement::Space(6, 0),
            ],
        );
    }

    #[test]
    fn cursor_limit() {
        let mut parser = Parser::parse("Some sample text");

        assert_line_elements(
            &mut parser,
            &mut None,
            2,
            &[RenderElement::PrintedCharacters("So")],
        );
    }
}

#[cfg(all(test, feature = "ansi"))]
mod ansi_parser_tests {
    use super::{test::assert_line_elements, *};
    use crate::style::color::Rgb;

    #[test]
    fn colors() {
        let mut parser = Parser::parse("Lorem \x1b[92mIpsum");

        assert_line_elements(
            &mut parser,
            &mut None,
            100,
            &[
                RenderElement::PrintedCharacters("Lorem"),
                RenderElement::Space(6, 1),
                RenderElement::Sgr(Sgr::ChangeTextColor(Rgb::new(22, 198, 12))),
                RenderElement::PrintedCharacters("Ipsum"),
            ],
        );
    }

    #[test]
    fn ansi_code_does_not_break_word() {
        let mut parser = Parser::parse("Lorem foo\x1b[92mbarum");

        assert_line_elements(
            &mut parser,
            &mut None,
            8,
            &[RenderElement::PrintedCharacters("Lorem")],
        );

        assert_line_elements(
            &mut parser,
            &mut None,
            8,
            &[
                RenderElement::PrintedCharacters("foo"),
                RenderElement::Sgr(Sgr::ChangeTextColor(Rgb::new(22, 198, 12))),
                RenderElement::PrintedCharacters("barum"),
            ],
        );
    }
}
