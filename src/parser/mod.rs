//! Parse text into words, newlines and whitespace sequences.
//!
//! ```rust,ignore
//! use embedded_text::parser::{Parser, Token};
//!
//! let parser = Parser::parse("Hello, world!\n");
//! let tokens = parser.collect::<Vec<Token<'_>>>();
//!
//! assert_eq!(
//!     vec![
//!         Token::Word("Hello,"),
//!         Token::Whitespace(1, " "),
//!         Token::Word("world!"),
//!         Token::NewLine
//!     ],
//!     tokens
//! );
//! ```
use ansi_parser::AnsiSequence;
use core::{marker::PhantomData, str::Chars};
use embedded_graphics::{prelude::PixelColor, text::DecorationColor};

/// Change text style.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChangeTextStyle<C>
where
    C: PixelColor,
{
    /// Reset text style. Disables decoration, removes background color and sets a default text color.
    Reset,

    /// Change text color. `None` means transparent.
    TextColor(Option<C>),

    /// Change background color. `None` means transparent.
    BackgroundColor(Option<C>),

    /// Change color of underlining.
    Underline(DecorationColor<C>),

    /// Change color of strikethrough decoration.
    Strikethrough(DecorationColor<C>),
}

/// A text token
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a, C>
where
    C: PixelColor,
{
    /// A newline character.
    NewLine,

    /// A \r character.
    CarriageReturn,

    /// A \t character.
    Tab,

    /// A number of whitespace characters.
    Whitespace(u32, &'a str),

    /// A word (a sequence of non-whitespace characters).
    Word(&'a str),

    /// A possible wrapping point
    Break(&'a str, &'a str),

    /// Change of text style.
    ChangeTextStyle(ChangeTextStyle<C>),

    /// An ANSI escape sequence
    EscapeSequence(AnsiSequence),
}

/// Text parser. Turns a string into a stream of [`Token`] objects.
///
/// [`Token`]: enum.Token.html
#[derive(Clone, Debug)]
pub(crate) struct Parser<'a, C>
where
    C: PixelColor,
{
    inner: Chars<'a>,
    _marker: PhantomData<C>,
}

pub(crate) const SPEC_CHAR_NBSP: char = '\u{a0}';
pub(crate) const SPEC_CHAR_ZWSP: char = '\u{200b}';
pub(crate) const SPEC_CHAR_SHY: char = '\u{ad}';

fn is_word_char(c: char) -> bool {
    // Word tokens are terminated when a whitespace, zwsp or shy character is found. An exception
    // to this rule is the nbsp, which is whitespace but is included in the word.
    (!c.is_whitespace() || c == SPEC_CHAR_NBSP) && ![SPEC_CHAR_ZWSP, SPEC_CHAR_SHY].contains(&c)
}

fn is_space_char(c: char) -> bool {
    // zero-width space breaks whitespace sequences - this works as long as
    // space handling is symmetrical (i.e. starting == ending behaviour)
    c.is_whitespace() && !['\n', '\r', '\t', SPEC_CHAR_NBSP].contains(&c) || c == SPEC_CHAR_ZWSP
}

impl<'a, C> Parser<'a, C>
where
    C: PixelColor,
{
    /// Create a new parser object to process the given piece of text.
    #[inline]
    #[must_use]

    pub fn parse(text: &'a str) -> Self {
        Self {
            inner: text.chars(),
            _marker: PhantomData,
        }
    }

    pub unsafe fn consume(&mut self, bytes: usize) {
        // SAFETY: caller needs to make sure we end up on character boundary
        self.inner = self.inner.as_str().get_unchecked(bytes..).chars();
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl<'a, C> Iterator for Parser<'a, C>
where
    C: PixelColor,
{
    type Item = Token<'a, C>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let string = self.inner.as_str();

        if let Some(c) = self.inner.next() {
            if is_word_char(c) {
                // find the longest consecutive slice of text for a Word token
                for c in &mut self.inner {
                    if !is_word_char(c) {
                        // pointer arithmetic to get the offset of `c` relative to `string`
                        let offset = {
                            let ptr_start = string.as_ptr() as usize;
                            let ptr_cur = self.inner.as_str().as_ptr() as usize;
                            ptr_cur - ptr_start - c.len_utf8()
                        };
                        self.inner = unsafe {
                            // SAFETY: we only work with character boundaries and
                            // offset is <= length
                            string.get_unchecked(offset..).chars()
                        };
                        return Some(Token::Word(unsafe {
                            // SAFETY: we only work with character boundaries and
                            // offset is <= length
                            string.get_unchecked(0..offset)
                        }));
                    }
                }

                // consumed all the text
                Some(Token::Word(string))
            } else {
                match c {
                    // special characters
                    '\n' => Some(Token::NewLine),
                    '\r' => Some(Token::CarriageReturn),
                    '\t' => Some(Token::Tab),
                    SPEC_CHAR_ZWSP => Some(Token::Whitespace(0, unsafe {
                        // SAFETY: we only work with character boundaries and
                        // offset is <= length
                        string.get_unchecked(0..c.len_utf8())
                    })),
                    SPEC_CHAR_SHY => Some(Token::Break(
                        "-", // translate SHY to a printable character
                        unsafe {
                            // SAFETY: we only work with character boundaries and
                            // offset is <= length
                            string.get_unchecked(0..c.len_utf8())
                        },
                    )),

                    // count consecutive whitespace
                    _ => {
                        let mut len = 1;
                        for c in &mut self.inner {
                            if is_space_char(c) {
                                if c != SPEC_CHAR_ZWSP {
                                    len += 1;
                                }
                            } else {
                                // pointer arithmetic to get the offset of `c` relative to `string`
                                let offset = {
                                    let ptr_start = string.as_ptr() as usize;
                                    let ptr_cur = self.inner.as_str().as_ptr() as usize;
                                    ptr_cur - ptr_start - c.len_utf8()
                                };
                                // consume the whitespaces
                                self.inner = unsafe {
                                    // SAFETY: we only work with character boundaries and
                                    // offset is <= length
                                    string.get_unchecked(offset..).chars()
                                };
                                return Some(Token::Whitespace(len, unsafe {
                                    // SAFETY: we only work with character boundaries and
                                    // offset is <= length
                                    string.get_unchecked(0..offset)
                                }));
                            }
                        }

                        // consumed all the text
                        Some(Token::Whitespace(len, string))
                    }
                }
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use embedded_graphics::pixelcolor::BinaryColor;

    use super::{Parser, Token};

    #[track_caller]
    pub fn assert_tokens(text: &str, tokens: std::vec::Vec<Token<BinaryColor>>) {
        assert_eq!(
            Parser::parse(text).collect::<std::vec::Vec<Token<BinaryColor>>>(),
            tokens
        )
    }

    #[test]
    fn test_parse() {
        assert_tokens(
            "Lorem ipsum \r dolor sit am\u{00AD}et,\tconse😅ctetur adipiscing\nelit",
            vec![
                Token::Word("Lorem"),
                Token::Whitespace(1, " "),
                Token::Word("ipsum"),
                Token::Whitespace(1, " "),
                Token::CarriageReturn,
                Token::Whitespace(1, " "),
                Token::Word("dolor"),
                Token::Whitespace(1, " "),
                Token::Word("sit"),
                Token::Whitespace(1, " "),
                Token::Word("am"),
                Token::Break("-", "\u{ad}"),
                Token::Word("et,"),
                Token::Tab,
                Token::Word("conse😅ctetur"),
                Token::Whitespace(1, " "),
                Token::Word("adipiscing"),
                Token::NewLine,
                Token::Word("elit"),
            ],
        );
    }

    #[test]
    fn parse_zwsp() {
        assert_eq!(9, "two\u{200B}words".chars().count());

        assert_tokens(
            "two\u{200B}words",
            vec![
                Token::Word("two"),
                Token::Whitespace(0, "\u{200B}"),
                Token::Word("words"),
            ],
        );

        // ZWSP is not counted
        assert_tokens("  \u{200B} ", vec![Token::Whitespace(3, "  \u{200B} ")]);
    }

    #[test]
    fn parse_multibyte_last() {
        assert_tokens("test😅", vec![Token::Word("test😅")]);
    }

    #[test]
    fn parse_nbsp_as_word_char() {
        assert_eq!(9, "test\u{A0}word".chars().count());
        assert_tokens("test\u{A0}word", vec![Token::Word("test\u{A0}word")]);
        assert_tokens(
            " \u{A0}word",
            vec![Token::Whitespace(1, " "), Token::Word("\u{A0}word")],
        );
    }

    #[test]
    fn parse_shy_issue_42() {
        assert_tokens(
            "foo\u{AD}bar",
            vec![
                Token::Word("foo"),
                Token::Break("-", "\u{ad}"),
                Token::Word("bar"),
            ],
        );
    }
}
