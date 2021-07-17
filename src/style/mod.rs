//! `TextBox` styling.
//!
//! Style objects and why you need them
//! ===================================
//!
//! By itself, a [`TextBox`] does not contain the information necessary to draw it on a display.
//! This information is called "style" and it is contained in [`TextBoxStyle`] objects.
//!
//! The recommended (and most flexible) way of constructing a style object is using the
//! [`TextBoxStyleBuilder`] builder object. The least amount of information necessary to create a
//! text style is the `MonoFont` used to render the text, so you'll need to specify this when you call
//! [`TextBoxStyleBuilder::new`].
//! You can then chain together various builder methods to customize MonoFont rendering.
//!
//! See the [`TextBoxStyleBuilder`] for more information on what styling options you have.
//!
//! To apply a style, call [`TextBox::into_styled`].
//!
//! In-band text styling using ANSI escape codes
//! ============================================
//!
//! Sometimes you need more flexibility than what a single style object can provide, like changing
//! MonoFont color for a specific word in the text. `embedded-text` supports this use case by using a
//! subset of the standard [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code).
//! These are special character sequences you can use *in the text* to change the MonoFont style of the
//! text itself. This documentation does not aim to provide a full specification of all the ANSI
//! escape codes, only describes the supported subset.
//!
//! > *Note:* if `embedded-text` fails to parse an escape sequence, it will ignore the `\x1b` character
//! and display the rest as normal text.
//!
//! All escape sequences start with the `\x1b[` sequence, where `\x1b` is the ASCII `escape`
//! character. `embedded-text` supports a subset of the `SGR` parameters, which are numeric codes
//! with specific functions, followed by a number of parameters and end with the `m` character.
//!
//! Currently, `embedded-text` supports changing the text and background colors. To do this, you
//! have the following options:
//!
//! Standard color codes
//! --------------------
//!
//! <style>
//! .ansi_color {
//!     display: block;
//!     text-align: center;
//!     color: white;
//! }
//! </style>
//!
//! The standard color codes option is the simplest, and least flexible way to set color.
//!
//! | Color name          | Text color | Background color | RGB888                                                                                          |
//! |---------------------|------------|------------------|-------------------------------------------------------------------------------------------------|
//! | Black               | `\x1b[30m` | `\x1b[40m`       | <span class="ansi_color" style="background: rgb(12,12,12);"> 12,12,12 </span>                     |
//! | Red                 | `\x1b[31m` | `\x1b[41m`       | <span class="ansi_color" style="background: rgb(197,15,31);"> 197,15,31 </span>                   |
//! | Green               | `\x1b[32m` | `\x1b[42m`       | <span class="ansi_color" style="background: rgb(19,161,14);"> 19,161,14 </span>                   |
//! | Yellow              | `\x1b[33m` | `\x1b[43m`       | <span class="ansi_color" style="background: rgb(193,156,0);"> 193,156,0 </span>                   |
//! | Blue                | `\x1b[34m` | `\x1b[44m`       | <span class="ansi_color" style="background: rgb(0,55,218);"> 0,55,218 </span>                     |
//! | Magenta             | `\x1b[35m` | `\x1b[45m`       | <span class="ansi_color" style="background: rgb(136,23,152);"> 136,23,152 </span>                 |
//! | Cyan                | `\x1b[36m` | `\x1b[46m`       | <span class="ansi_color" style="background: rgb(58,150,221);"> 58,150,221 </span>                 |
//! | White               | `\x1b[37m` | `\x1b[47m`       | <span class="ansi_color" style="background: rgb(204,204,204); color: black;"> 204,204,204 </span> |
//! | Gray (Bright Black) | `\x1b[90m` | `\x1b[100m`      | <span class="ansi_color" style="background: rgb(118,118,118); color: black;"> 118,118,118 </span> |
//! | Bright Red          | `\x1b[91m` | `\x1b[101m`      | <span class="ansi_color" style="background: rgb(231,72,86);"> 231,72,86 </span>                   |
//! | Bright Green        | `\x1b[92m` | `\x1b[102m`      | <span class="ansi_color" style="background: rgb(22,198,12); color: black;"> 22,198,12 </span>     |
//! | Bright Yellow       | `\x1b[93m` | `\x1b[103m`      | <span class="ansi_color" style="background: rgb(249,241,165); color: black;"> 249,241,165 </span> |
//! | Bright Blue         | `\x1b[94m` | `\x1b[104m`      | <span class="ansi_color" style="background: rgb(59,120,255);"> 59,120,255 </span>                 |
//! | Bright Magenta      | `\x1b[95m` | `\x1b[105m`      | <span class="ansi_color" style="background: rgb(180,0,158);"> 180,0,158 </span>                   |
//! | Bright Cyan         | `\x1b[96m` | `\x1b[106m`      | <span class="ansi_color" style="background: rgb(97,214,214); color: black;"> 97,214,214 </span>   |
//! | Bright White        | `\x1b[97m` | `\x1b[107m`      | <span class="ansi_color" style="background: rgb(242,242,242); color: black;"> 242,242,242 </span> |
//!
//! 8 bit colors
//! ------------
//!
//! 8 bit colors are in the form of either `\x1b[38;5;<n>m` (text color) or `\x1b[48;5;<n>m`
//! (background color) sequence. Here, `<n>` marks a parameter that determines the color. `<n>` can
//! have the following values:
//!
//! * 0-15: standard colors in the order of the above table.
//!   For example, `\x1b[38;5;12m` is the `Bright Blue` color.
//! * 16-231: 6 × 6 × 6 cube (216 colors): `16 + 36 × r + 6 × g + b (0 ≤ r, g, b ≤ 5)`
//! * 232-255: grayscale from black to white
//!
//! 24 bit colors
//! -------------
//!
//! 8 bit colors are in the form of either `\x1b[38;2;<r>;<g>;<b>m` (text color) or
//! `\x1b[48;2;<r>;<g>;<b>m` (background color) sequence. Here, `<r>`, `<g>` and `<b>` can take any
//! value between `0` and `255`.
//!
//! Supported color types
//! ---------------------
//!
//! `embedded-text` supports all color types that are included in `embedded-graphics`.
//!
//! If you wish to use a different color type, the types needs to implement `From<Rgb888>`.
//!
//! Other text styling options
//! --------------------------
//!
//! The following SGR sequences are supported:
//!
//!  * `\x1b[0m`: Reset everything
//!  * `\x1b[4m`: Underlined text
//!  * `\x1b[24m`: Turn off text underline
//!  * `\x1b[9m`: Crossed out/strikethrough text
//!  * `\x1b[29m`: Turn off strikethrough
//!  * `\x1b[39m`: Reset text color
//!  * `\x1b[49m`: Reset background color
//!
//! Reset style options to default
//! ------------------------------
//!
//! `embedded-text` supports the `Reset all` (`\x1b[0m`), `Default text color` (`\x1b[39m`) and
//! `Default background color` (`\x1b[49m`) codes. These codes can be used to reset colors to
//! *transparent* (i.e. no pixels drawn for text or background).
//!
//! In addition, `Reset all` turns off the underlined and crossed out styles.
//!
//! Other supported ANSI escape codes
//! ---------------------------------
//!
//! Besides changing text style, you can also move the cursor using ANSI escape codes!
//! You have the following options:
//!
//!  - Move the cursor forward `<n>` characters: `\x1b[<n>C`. This command will stop at the end of
//!    line, so you can use it to simulate a highlighted line, for example.
//!    *Note:* Moving the cursor *forward* fills the line with the background color. If you want to
//!    avoid this, make sure to reset the background color before moving the cursor!
//!  - Move the cursor backward `<n>` characters: `\x1b[<n>D`. This command will stop at the start
//!    of line.
//!
//! [`TextBox`]: crate::TextBox
//! [`TextBoxStyleBuilder`]: builder/struct.TextBoxStyleBuilder.html
//! [`TextBoxStyleBuilder::new`]: builder/struct.TextBoxStyleBuilder.html#method.new
//! [`TextBox::into_styled`]: ../struct.TextBox.html#method.into_styled

mod builder;
mod height_mode;
mod vertical_overdraw;

use core::convert::Infallible;

use crate::{
    alignment::{HorizontalAlignment, VerticalAlignment},
    parser::{Parser, SPEC_CHAR_NBSP},
    plugin::{NoPlugin, PluginMarker as Plugin, PluginWrapper, ProcessingState},
    rendering::{
        cursor::LineCursor,
        line_iter::{ElementHandler, LineElementParser, LineEndType},
        space_config::SpaceConfig,
    },
    utils::str_width,
};
use az::SaturatingAs;
use embedded_graphics::{
    pixelcolor::Rgb888,
    text::{renderer::TextRenderer, LineHeight},
};

pub use self::{
    builder::TextBoxStyleBuilder, height_mode::HeightMode, vertical_overdraw::VerticalOverdraw,
};

/// Tab size helper
///
/// This type makes it more obvious what unit is used to define the width of tabs.
/// The default tab size is 4 spaces.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TabSize {
    /// Tab width as a number of pixels.
    Pixels(u16),

    /// Tab width as a number of space characters.
    Spaces(u16),
}

impl TabSize {
    /// Returns the default tab size, which is 4 spaces.
    #[inline]
    pub const fn default() -> Self {
        Self::Spaces(4)
    }

    /// Calculate the rendered with of the next tab
    #[inline]
    pub(crate) fn into_pixels(self, renderer: &impl TextRenderer) -> u32 {
        match self {
            TabSize::Pixels(px) => px as u32,
            TabSize::Spaces(n) => n as u32 * str_width(renderer, " "),
        }
    }
}

/// Styling options of a [`TextBox`].
///
/// `TextBoxStyle` contains the font, foreground and background `PixelColor`, line spacing,
/// [`HeightMode`], [`HorizontalAlignment`] and [`VerticalAlignment`] information necessary
/// to draw a [`TextBox`].
///
/// To construct a new `TextBoxStyle` object, use the [`new`] or [`from_text_style`] methods or
/// the [`TextBoxStyleBuilder`] object.
///
/// [`TextBox`]: ../struct.TextBox.html
/// [`HeightMode`]: ./enum.HeightMode.html
/// [`HorizontalAlignment`]: ../alignment/enum.HorizontalAlignment.html
/// [`VerticalAlignment`]: ../alignment/enum.VerticalAlignment.html
/// [`TextBoxStyleBuilder`]: builder/struct.TextBoxStyleBuilder.html
/// [`new`]: #method.new
/// [`from_text_style`]: #method.from_text_style
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
#[must_use]
pub struct TextBoxStyle {
    /// Horizontal text alignment.
    pub alignment: HorizontalAlignment,

    /// Vertical text alignment.
    pub vertical_alignment: VerticalAlignment,

    /// The height behaviour.
    pub height_mode: HeightMode,

    /// Line height.
    pub line_height: LineHeight,

    /// Paragraph spacing.
    pub paragraph_spacing: u32,

    /// Desired column width for tabs
    pub tab_size: TabSize,
}

impl TextBoxStyle {
    /// Creates a new text box style object with default settings.
    #[inline]
    pub const fn default() -> Self {
        TextBoxStyleBuilder::new().build()
    }

    /// Creates a new text box style with the given alignment.
    #[inline]
    pub const fn with_alignment(alignment: HorizontalAlignment) -> TextBoxStyle {
        TextBoxStyleBuilder::new().alignment(alignment).build()
    }

    /// Creates a new text box style with the given vertical alignment.
    #[inline]
    pub const fn with_vertical_alignment(alignment: VerticalAlignment) -> TextBoxStyle {
        TextBoxStyleBuilder::new()
            .vertical_alignment(alignment)
            .build()
    }
}

/// Information about a line.
#[derive(Debug)]
#[must_use]
pub(crate) struct LineMeasurement {
    /// Maximum line width in pixels.
    pub max_line_width: u32,

    /// Width in pixels, using the default space width returned by the text renderer.
    pub width: u32,

    /// Whether this line is the last line of a paragraph.
    pub last_line: bool,

    /// Whether this line ended with a \r.
    pub line_end_type: LineEndType,

    /// Number of spaces in the current line.
    pub space_count: u32,
}

struct MeasureLineElementHandler<'a, S> {
    style: &'a S,
    right: u32,
    max_line_width: u32,
    pos: u32,
    space_count: u32,
    partial_space_count: u32,
}

impl<'a, S: TextRenderer> ElementHandler for MeasureLineElementHandler<'a, S> {
    type Error = Infallible;
    type Color = S::Color;

    fn measure(&self, st: &str) -> u32 {
        str_width(self.style, st)
    }

    fn whitespace(&mut self, st: &str, _count: u32, width: u32) -> Result<(), Self::Error> {
        self.pos += width;

        self.partial_space_count += st
            .chars()
            .filter(|c| [' ', SPEC_CHAR_NBSP].contains(c))
            .count()
            .saturating_as::<u32>();

        Ok(())
    }

    fn printed_characters(&mut self, _: &str, width: u32) -> Result<(), Self::Error> {
        self.right = self.right.max(self.pos + width);
        self.pos += width;
        self.space_count = self.partial_space_count;
        Ok(())
    }

    fn move_cursor(&mut self, by: i32) -> Result<(), Self::Error> {
        self.pos = (self.pos.saturating_as::<i32>() + by)
            .max(0)
            .min(self.max_line_width.saturating_as()) as u32;

        Ok(())
    }
}

impl TextBoxStyle {
    /// Measure the width and count spaces in a single line of text.
    ///
    /// Returns (width, rendered space count, carried token)
    ///
    /// Instead of peeking ahead when processing tokens, this function advances the parser before
    /// processing a token. If a token opens a new line, it will be returned as the carried token.
    /// If the carried token is `None`, the parser has finished processing the text.
    #[inline]
    pub(crate) fn measure_line<'a, S, M>(
        &self,
        plugin: &PluginWrapper<'a, M, S::Color>,
        character_style: &S,
        parser: &mut Parser<'a, S::Color>,
        max_line_width: u32,
    ) -> LineMeasurement
    where
        S: TextRenderer,
        M: Plugin<'a, S::Color>,
        S::Color: From<Rgb888>,
    {
        let cursor = LineCursor::new(max_line_width, self.tab_size.into_pixels(character_style));

        let mut iter = LineElementParser::new(
            parser,
            plugin,
            cursor,
            SpaceConfig::new(str_width(character_style, " "), None),
            self.alignment,
        );

        let mut handler = MeasureLineElementHandler {
            style: character_style,
            right: 0,
            pos: 0,
            max_line_width,
            space_count: 0,
            partial_space_count: 0,
        };
        let last_token = iter.process(&mut handler).unwrap();

        LineMeasurement {
            max_line_width,
            width: handler.right,
            space_count: handler.space_count,
            last_line: matches!(last_token, LineEndType::NewLine | LineEndType::EndOfText),
            line_end_type: last_token,
        }
    }

    /// Measures text height when rendered using a given width.
    ///
    /// # Example: measure height of text when rendered using a 6x8 MonoFont and 72px width.
    ///
    /// ```rust
    /// # use embedded_text::style::TextBoxStyleBuilder;
    /// # use embedded_graphics::{
    /// #     mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
    /// #     pixelcolor::BinaryColor,
    /// # };
    /// #
    /// let character_style = MonoTextStyleBuilder::new()
    ///     .font(&FONT_6X9)
    ///     .text_color(BinaryColor::On)
    ///     .build();
    /// let style = TextBoxStyleBuilder::new().build();
    ///
    /// let height = style.measure_text_height(
    ///     &character_style,
    ///     "Lorem Ipsum is simply dummy text of the printing and typesetting industry.",
    ///     72,
    /// );
    ///
    /// // Expect 7 lines of text, wrapped in something like the following:
    ///
    /// // |Lorem Ipsum |
    /// // |is simply   |
    /// // |dummy text  |
    /// // |of the      |
    /// // |printing and|
    /// // |typesetting |
    /// // |industry.   |
    ///
    /// assert_eq!(7 * 9, height);
    /// ```
    #[inline]
    #[must_use]
    pub fn measure_text_height<S>(&self, character_style: &S, text: &str, max_width: u32) -> u32
    where
        S: TextRenderer,
        S::Color: From<Rgb888>,
    {
        let plugin = PluginWrapper::new(NoPlugin::new());
        self.measure_text_height_impl(plugin, character_style, text, max_width)
    }

    pub(crate) fn measure_text_height_impl<'a, S, M>(
        &self,
        plugin: PluginWrapper<'a, M, S::Color>,
        character_style: &S,
        text: &'a str,
        max_width: u32,
    ) -> u32
    where
        S: TextRenderer,
        M: Plugin<'a, S::Color>,
        S::Color: From<Rgb888>,
    {
        let mut parser = Parser::parse(text);
        let mut closed_paragraphs: u32 = 0;
        let line_height = self.line_height.to_absolute(character_style.line_height());
        let last_line_height = character_style.line_height();
        let mut height = last_line_height;
        let mut paragraph_ended = false;

        plugin.set_state(ProcessingState::Measure);

        let mut prev_end = LineEndType::EndOfText;

        loop {
            plugin.new_line();
            let lm = self.measure_line(&plugin, character_style, &mut parser, max_width);

            if paragraph_ended {
                closed_paragraphs += 1;
            }
            paragraph_ended = lm.last_line;

            if prev_end == LineEndType::LineBreak && lm.width != 0 {
                height += line_height;
            }

            match lm.line_end_type {
                LineEndType::CarriageReturn => {}
                LineEndType::LineBreak => {}
                LineEndType::NewLine => {
                    height += line_height;
                }
                LineEndType::EndOfText => {
                    return height + closed_paragraphs * self.paragraph_spacing;
                }
            }
            prev_end = lm.line_end_type;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        alignment::*,
        parser::Parser,
        plugin::{NoPlugin, PluginWrapper},
        style::{builder::TextBoxStyleBuilder, TextBoxStyle},
    };
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        text::{renderer::TextRenderer, LineHeight},
    };

    #[test]
    fn no_infinite_loop() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let _ = TextBoxStyleBuilder::new()
            .build()
            .measure_text_height(&character_style, "a", 5);
    }

    #[test]
    fn test_measure_height() {
        let data = [
            // (text; max width in characters; number of expected lines)
            ("", 0, 1),
            (" ", 6, 1),
            ("\r", 6, 1),
            ("\n", 6, 2),
            ("\n ", 6, 2),
            ("word", 4 * 6, 1),   // exact fit into 1 line
            ("word\n", 4 * 6, 2), // newline
            ("word", 4 * 6 - 1, 2),
            ("word", 2 * 6, 2),      // exact fit into 2 lines
            ("word word", 4 * 6, 2), // exact fit into 2 lines
            ("word\n", 2 * 6, 3),
            ("word\nnext", 50, 2),
            ("word\n\nnext", 50, 3),
            ("word\n  \nnext", 50, 3),
            ("verylongword", 50, 2),
            ("some verylongword", 50, 3),
            ("1 23456 12345 61234 561", 36, 5),
            ("    Word      ", 36, 2),
            ("\rcr", 36, 1),
            ("cr\r", 36, 1),
            ("cr\rcr", 36, 1),
            ("Longer\r", 36, 1),
            ("Longer\rnowrap", 36, 1),
        ];

        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyle::default();

        for (i, (text, width, expected_n_lines)) in data.iter().enumerate() {
            let height = style.measure_text_height(&character_style, text, *width);
            let expected_height = *expected_n_lines * character_style.line_height();
            assert_eq!(
                height,
                expected_height,
                r#"#{}: Height of "{}" is {} but is expected to be {}"#,
                i,
                text.replace('\r', "\\r").replace('\n', "\\n"),
                height,
                expected_height
            );
        }
    }

    #[test]
    fn test_measure_height_ignored_spaces() {
        let data = [
            ("", 0, 1),
            (" ", 0, 1),
            (" ", 6, 1),
            ("\n ", 6, 2),
            ("word\n  \nnext", 50, 3),
            ("    Word      ", 36, 1),
        ];

        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();

        for (i, (text, width, expected_n_lines)) in data.iter().enumerate() {
            let height = style.measure_text_height(&character_style, text, *width);
            let expected_height = *expected_n_lines * character_style.line_height();
            assert_eq!(
                height, expected_height,
                r#"#{}: Height of "{}" is {} but is expected to be {}"#,
                i, text, height, expected_height
            );
        }
    }

    #[test]
    fn test_measure_line() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();

        let mut text = Parser::parse("123 45 67");

        let mut plugin = PluginWrapper::new(NoPlugin::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut text,
            6 * FONT_6X9.character_size.width,
        );
        assert_eq!(lm.width, 6 * FONT_6X9.character_size.width);
    }

    #[test]
    #[cfg(feature = "ansi")]
    fn test_measure_line_cursor_back() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();

        let mut text = Parser::parse("123\x1b[2D");

        let mut plugin = PluginWrapper::new(NoPlugin::new());
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

        let mut plugin = PluginWrapper::new(NoPlugin::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut text,
            5 * FONT_6X9.character_size.width,
        );
        assert_eq!(lm.width, 4 * FONT_6X9.character_size.width);
    }

    #[test]
    fn test_measure_line_counts_nbsp() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();

        let mut text = Parser::parse("123\u{A0}45");

        let mut plugin = PluginWrapper::new(NoPlugin::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut text,
            5 * FONT_6X9.character_size.width,
        );
        assert_eq!(lm.width, 5 * FONT_6X9.character_size.width);
    }

    #[test]
    fn test_measure_height_nbsp() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .build();
        let text = "123\u{A0}45 123";

        let height =
            style.measure_text_height(&character_style, text, 5 * FONT_6X9.character_size.width);
        assert_eq!(height, 2 * character_style.line_height());

        // bug discovered while using the interactive example
        let style = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Left)
            .build();

        let text = "embedded-text also\u{A0}supports non-breaking spaces.";

        let height = style.measure_text_height(&character_style, text, 79);
        assert_eq!(height, 4 * character_style.line_height());
    }

    #[test]
    fn height_with_line_spacing() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .line_height(LineHeight::Pixels(11))
            .build();

        let height = style.measure_text_height(
            &character_style,
            "Lorem Ipsum is simply dummy text of the printing and typesetting industry.",
            72,
        );

        assert_eq!(height, 6 * 11 + 9);
    }

    #[test]
    fn soft_hyphenated_line_width_includes_hyphen_width() {
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();

        let style = TextBoxStyleBuilder::new()
            .line_height(LineHeight::Pixels(11))
            .build();

        let mut plugin = PluginWrapper::new(NoPlugin::new());
        let lm = style.measure_line(
            &mut plugin,
            &character_style,
            &mut Parser::parse("soft\u{AD}hyphen"),
            50,
        );

        assert_eq!(lm.width, 30);
    }
}
