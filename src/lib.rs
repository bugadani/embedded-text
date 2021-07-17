//! TextBox for embedded-graphics.
//!
//! This crate provides a configurable [`TextBox`] to render multiline text inside a bounding
//! `Rectangle` using [embedded-graphics].
//!
//! [`TextBox`] supports the common text alignments:
//!  - [`Horizontal`]:
//!      - `Left`
//!      - `Right`
//!      - `Center`
//!      - `Justified`
//!  - [`Vertical`]:
//!      - `Top`
//!      - `Middle`
//!      - `Bottom`
//!
//! [`TextBox`] also supports some special characters not handled by embedded-graphics' `Text`:
//!  - non-breaking space (`\u{200b}`)
//!  - zero-width space (`\u{a0}`)
//!  - soft hyphen (`\u{ad}`)
//!  - carriage return (`\r`)
//!  - tab (`\t`) with configurable tab size
//!
//! `TextBox` also supports text coloring using [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code).
//!
//! ### Example
//!
//! The examples are based on [the embedded-graphics simulator]. The simulator is built on top of
//! `SDL2`. See the [simulator README] for more information.
//!
//! ![embedded-text example](https://raw.githubusercontent.com/embedded-graphics/embedded-text/master/assets/paragraph_spacing.png)
//!
//! ![embedded-text example with colored text](https://raw.githubusercontent.com/embedded-graphics/embedded-text/master/assets/styles-ansi.png)
//!
//! ```rust,no_run
//! use embedded_graphics::{
//!     mono_font::{ascii::FONT_6X10, MonoTextStyle},
//!     pixelcolor::BinaryColor,
//!     prelude::*,
//!     primitives::Rectangle,
//! };
//! use embedded_graphics_simulator::{
//!     BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
//! };
//! use embedded_text::{
//!     alignment::HorizontalAlignment,
//!     style::{HeightMode, TextBoxStyleBuilder},
//!     TextBox,
//! };
//!
//! fn main() {
//!     let text = "Hello, World!\n\
//!     A paragraph is a number of lines that end with a manual newline. Paragraph spacing is the \
//!     number of pixels between two paragraphs.\n\
//!     Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when \
//!     an unknown printer took a galley of type and scrambled it to make a type specimen book.";
//!
//!     // Specify the styling options:
//!     // * Use the 6x10 MonoFont from embedded-graphics.
//!     // * Draw the text fully justified.
//!     // * Use `FitToText` height mode to stretch the text box to the exact height of the text.
//!     // * Draw the text with `BinaryColor::On`, which will be displayed as light blue.
//!     let character_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
//!     let textbox_style = TextBoxStyleBuilder::new()
//!         .height_mode(HeightMode::FitToText)
//!         .alignment(HorizontalAlignment::Justified)
//!         .paragraph_spacing(6)
//!         .build();
//!
//!     // Specify the bounding box. Note the 0px height. The `FitToText` height mode will
//!     // measure and adjust the height of the text box in `into_styled()`.
//!     let bounds = Rectangle::new(Point::zero(), Size::new(128, 0));
//!
//!     // Create the text box and apply styling options.
//!     let text_box = TextBox::with_textbox_style(text, bounds, character_style, textbox_style);
//!
//!     // Create a simulated display with the dimensions of the text box.
//!     let mut display = SimulatorDisplay::new(text_box.bounding_box().size);
//!
//!     // Draw the text box.
//!     text_box.draw(&mut display).unwrap();
//!
//!     // Set up the window and show the display's contents.
//!     let output_settings = OutputSettingsBuilder::new()
//!         .theme(BinaryColorTheme::OledBlue)
//!         .scale(2)
//!         .build();
//!     Window::new("TextBox example with paragraph spacing", &output_settings).show_static(&display);
//! }
//! ```
//!
//! ## Cargo features
//!
//! * `ansi`: enables ANSI sequence support. This feature is enabled by default.
//! * `plugin` (*experimental*): allows the user to implement plugins.
//!
//! [embedded-graphics]: https://github.com/embedded-graphics/embedded-graphics/
//! [the embedded-graphics simulator]: https://github.com/embedded-graphics/embedded-graphics/tree/master/simulator
//! [simulator README]: https://github.com/embedded-graphics/embedded-graphics/tree/master/simulator#usage-without-sdl2
//! [`TextBox`]: ./struct.TextBox.html
//! [`Horizontal`]: ./alignment/enum.HorizontalAlignment.html
//! [`Vertical`]: ./alignment/enum.VerticalAlignment.html

#![cfg_attr(not(test), no_std)]
#![deny(clippy::missing_inline_in_public_items)]
#![deny(clippy::cargo)]
#![deny(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::needless_doctest_main)]

pub mod alignment;
mod parser;
pub mod plugin;
mod rendering;
pub mod style;

mod utils;

use crate::{
    alignment::{HorizontalAlignment, VerticalAlignment},
    plugin::{NoPlugin, PluginMarker as Plugin, PluginWrapper},
    style::TextBoxStyle,
};
use embedded_graphics::{
    geometry::{Dimensions, Point},
    pixelcolor::Rgb888,
    primitives::Rectangle,
    text::renderer::{CharacterStyle, TextRenderer},
    transform::Transform,
};
use object_chain::{Chain, ChainElement, Link};
pub use parser::{ChangeTextStyle, Token};
pub use rendering::{cursor::Cursor, TextBoxProperties};

/// A text box object.
///
/// The `TextBox` struct represents a piece of text that can be drawn on a display inside the given
/// bounding box.
///
/// Use the [`draw`] method to draw the text box on a display.
///
/// See the [module-level documentation] for more information.
///
/// [`into_styled`]: #method.into_styled
/// [`TextBoxStyle`]: style/struct.TextBoxStyle.html
/// [module-level documentation]: index.html
/// [`draw`]: #method.draw
#[derive(Clone, Debug, Hash)]
#[must_use]
pub struct TextBox<'a, S, M = NoPlugin<<S as TextRenderer>::Color>>
where
    S: TextRenderer,
{
    /// The text to be displayed in this `TextBox`
    pub text: &'a str,

    /// The bounding box of this `TextBox`
    pub bounds: Rectangle,

    /// The character style of the [`TextBox`].
    pub character_style: S,

    /// The style of the [`TextBox`].
    pub style: TextBoxStyle,

    /// Vertical offset applied to the text just before rendering.
    pub vertical_offset: i32,

    plugin: PluginWrapper<'a, M, S::Color>,
}

impl<'a, S> TextBox<'a, S, NoPlugin<<S as TextRenderer>::Color>>
where
    <S as TextRenderer>::Color: From<Rgb888>,
    S: TextRenderer + CharacterStyle,
{
    /// Creates a new `TextBox` instance with a given bounding `Rectangle`.
    #[inline]
    pub fn new(text: &'a str, bounds: Rectangle, character_style: S) -> Self {
        TextBox::with_textbox_style(text, bounds, character_style, TextBoxStyle::default())
    }

    /// Creates a new `TextBox` instance with a given bounding `Rectangle` and a given `TextBoxStyle`.
    #[inline]
    pub fn with_textbox_style(
        text: &'a str,
        bounds: Rectangle,
        character_style: S,
        textbox_style: TextBoxStyle,
    ) -> Self {
        let mut styled = TextBox {
            text,
            bounds,
            character_style,
            style: textbox_style,
            vertical_offset: 0,
            plugin: PluginWrapper::new(NoPlugin::new()),
        };

        styled.style.height_mode.apply(&mut styled);

        styled
    }

    /// Creates a new `TextBox` instance with a given bounding `Rectangle` and a given `TextBoxStyle`.
    #[inline]
    pub fn with_alignment(
        text: &'a str,
        bounds: Rectangle,
        character_style: S,
        alignment: HorizontalAlignment,
    ) -> Self {
        TextBox::with_textbox_style(
            text,
            bounds,
            character_style,
            TextBoxStyle::with_alignment(alignment),
        )
    }

    /// Creates a new `TextBox` instance with a given bounding `Rectangle` and a given `TextBoxStyle`.
    #[inline]
    pub fn with_vertical_alignment(
        text: &'a str,
        bounds: Rectangle,
        character_style: S,
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        TextBox::with_textbox_style(
            text,
            bounds,
            character_style,
            TextBoxStyle::with_vertical_alignment(vertical_alignment),
        )
    }

    /// Sets the vertical text offset.
    #[inline]
    pub fn set_vertical_offset(&mut self, offset: i32) -> &mut Self {
        self.vertical_offset = offset;
        self
    }

    /// Adds a new plugin to the `TextBox`.
    #[inline]
    pub fn add_plugin<M>(self, plugin: M) -> TextBox<'a, S, Chain<M>>
    where
        M: Plugin<'a, <S as TextRenderer>::Color>,
    {
        let mut textbox = TextBox {
            text: self.text,
            bounds: self.bounds,
            character_style: self.character_style,
            style: self.style,
            vertical_offset: self.vertical_offset,
            plugin: PluginWrapper::new(Chain::new(plugin)),
        };
        textbox.style.height_mode.apply(&mut textbox);

        textbox
    }
}

impl<'a, S, P> TextBox<'a, S, P>
where
    <S as TextRenderer>::Color: From<Rgb888>,
    S: TextRenderer + CharacterStyle,
    P: Plugin<'a, <S as TextRenderer>::Color> + ChainElement,
{
    /// Adds a new plugin to the `TextBox`.
    #[inline]
    pub fn add_plugin<M>(self, plugin: M) -> TextBox<'a, S, Link<M, P>>
    where
        M: Plugin<'a, <S as TextRenderer>::Color>,
    {
        let parent = self.plugin.inner.into_inner();

        let mut textbox = TextBox {
            text: self.text,
            bounds: self.bounds,
            character_style: self.character_style,
            style: self.style,
            vertical_offset: self.vertical_offset,
            plugin: PluginWrapper::new(parent.plugin.append(plugin)),
        };
        textbox.style.height_mode.apply(&mut textbox);

        textbox
    }
}

impl<'a, S, M> Transform for TextBox<'a, S, M>
where
    S: TextRenderer + Clone,
    M: Plugin<'a, S::Color>,
{
    #[inline]
    fn translate(&self, by: Point) -> Self {
        Self {
            bounds: self.bounds.translate(by),
            ..self.clone()
        }
    }

    #[inline]
    fn translate_mut(&mut self, by: Point) -> &mut Self {
        self.bounds.translate_mut(by);

        self
    }
}

impl<'a, S, M> Dimensions for TextBox<'a, S, M>
where
    S: TextRenderer,
    M: Plugin<'a, S::Color>,
{
    #[inline]
    fn bounding_box(&self) -> Rectangle {
        self.bounds
    }
}

impl<'a, S, M> TextBox<'a, S, M>
where
    S: TextRenderer,
    M: Plugin<'a, S::Color>,
    S::Color: From<Rgb888>,
{
    /// Sets the height of the [`TextBox`] to the height of the text.
    #[inline]
    pub fn fit_height(&mut self) -> &mut Self {
        self.fit_height_limited(u32::max_value())
    }

    /// Sets the height of the [`TextBox`] to the height of the text, limited to `max_height`.
    ///
    /// This method allows you to set a maximum height. The [`TextBox`] will take up at most
    /// `max_height` pixel vertical space.
    #[inline]
    pub fn fit_height_limited(&mut self, max_height: u32) -> &mut Self {
        // Measure text given the width of the textbox
        let text_height = self
            .style
            .measure_text_height(
                &self.character_style,
                self.text,
                self.bounding_box().size.width,
            )
            .min(max_height)
            .min(i32::max_value() as u32);

        // Apply height
        self.bounds.size.height = text_height;

        self
    }
}
