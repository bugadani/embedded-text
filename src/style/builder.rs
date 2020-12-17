//! Textbox style builder.
use crate::{
    alignment::{HorizontalTextAlignment, LeftAligned, TopAligned, VerticalTextAlignment},
    style::{
        height_mode::{Exact, HeightMode},
        vertical_overdraw::FullRowsOnly,
        TabSize, TextBoxStyle,
    },
};
use embedded_graphics::{
    prelude::*,
    style::{MonoTextStyle, MonoTextStyleBuilder},
};

/// [`TextBoxStyle`] builder object.
///
/// [`TextBoxStyle`]: ../struct.TextBoxStyle.html
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TextBoxStyleBuilder<C, F, A, V, H>
where
    C: PixelColor,
    F: MonoFont,
{
    text_style_builder: MonoTextStyleBuilder<C, F>,
    alignment: A,
    vertical_alignment: V,
    height_mode: H,
    line_spacing: i32,
    tab_size: TabSize<F>,
    underlined: bool,
    strikethrough: bool,
}

impl<C, F> TextBoxStyleBuilder<C, F, LeftAligned, TopAligned, Exact<FullRowsOnly>>
where
    C: PixelColor,
    F: MonoFont,
{
    /// Creates a new `TextBoxStyleBuilder` with a given MonoFont.
    ///
    /// Default settings are:
    ///  - [`LeftAligned`]
    ///  - [`TopAligned`]
    ///  - Text color: transparent
    ///  - Background color: transparent
    ///  - Height mode: [`Exact`]
    ///  - Line spacing: 0px
    #[inline]
    #[must_use]
    pub fn new(font: F) -> Self {
        Self {
            text_style_builder: MonoTextStyleBuilder::new().font(font),
            alignment: LeftAligned,
            vertical_alignment: TopAligned,
            height_mode: Exact(FullRowsOnly),
            line_spacing: 0,
            tab_size: TabSize::default(),
            underlined: false,
            strikethrough: false,
        }
    }

    /// Creates a `TextBoxStyleBuilder` from existing `MonoTextStyle` object.
    ///
    /// # Example
    ///
    /// ```rust
    /// use embedded_text::prelude::*;
    /// use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor, style::MonoTextStyleBuilder};
    ///
    /// let text_style = MonoTextStyleBuilder::new()
    ///     .font(Font6x8)
    ///     .background_color(BinaryColor::On)
    ///     .build();
    ///
    /// let style = TextBoxStyleBuilder::from_text_style(text_style)
    ///     .build();
    /// ```
    #[inline]
    #[must_use]
    pub fn from_text_style(text_style: MonoTextStyle<C, F>) -> Self {
        let mut text_style_builder = MonoTextStyleBuilder::new().font(text_style.font);

        if let Some(color) = text_style.background_color {
            text_style_builder = text_style_builder.background_color(color);
        }

        if let Some(color) = text_style.text_color {
            text_style_builder = text_style_builder.text_color(color);
        }

        Self {
            text_style_builder,
            ..Self::new(text_style.font)
        }
    }
}

impl<C, F, A, V, H> TextBoxStyleBuilder<C, F, A, V, H>
where
    C: PixelColor,
    F: MonoFont,
    A: HorizontalTextAlignment,
    V: VerticalTextAlignment,
    H: HeightMode,
{
    /// Sets the text color.
    ///
    /// *Note:* once the text color is set, there is no way to reset it to transparent.
    ///
    /// # Example: text with transparent background.
    ///
    /// ```rust
    /// use embedded_text::prelude::*;
    /// use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor};
    ///
    /// let style = TextBoxStyleBuilder::new(Font6x8)
    ///     .text_color(BinaryColor::On)
    ///     .build();
    /// ```
    #[inline]
    #[must_use]
    pub fn text_color(self, text_color: C) -> Self {
        Self {
            text_style_builder: self.text_style_builder.text_color(text_color),
            ..self
        }
    }
    /// Sets the vertical space between lines, in pixels.
    ///
    /// *Note:* You can set negative values as line spacing if you wish your lines to overlap.
    ///
    /// # Example
    ///
    /// ```rust
    /// use embedded_text::prelude::*;
    /// use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor};
    ///
    /// let style = TextBoxStyleBuilder::new(Font6x8)
    ///     .text_color(BinaryColor::On)
    ///     .line_spacing(3)
    ///     .build();
    /// ```
    #[inline]
    #[must_use]
    pub fn line_spacing(self, line_spacing: i32) -> Self {
        Self {
            line_spacing,
            ..self
        }
    }

    /// Sets the background color.
    ///
    /// *Note:* once the background color is set, there is no way to reset it to transparent.
    ///
    /// # Example: transparent text with background.
    ///
    /// ```rust
    /// use embedded_text::prelude::*;
    /// use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor};
    ///
    /// let style = TextBoxStyleBuilder::new(Font6x8)
    ///     .background_color(BinaryColor::On)
    ///     .build();
    /// ```
    #[inline]
    #[must_use]
    pub fn background_color(self, background_color: C) -> Self {
        Self {
            text_style_builder: self.text_style_builder.background_color(background_color),
            ..self
        }
    }

    /// Copies properties from an existing text style object.
    ///
    /// # Example
    ///
    /// ```rust
    /// use embedded_text::prelude::*;
    /// use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor, style::MonoTextStyleBuilder};
    ///
    /// let text_style = MonoTextStyleBuilder::new()
    ///     .font(Font6x8)
    ///     .background_color(BinaryColor::On)
    ///     .build();
    ///
    /// let style = TextBoxStyleBuilder::new(Font6x8)
    ///     .text_style(text_style)
    ///     .build();
    /// ```
    ///
    /// This method has been deprecated and will be removed in a later release. Use
    /// [`TextBoxStyleBuilder::from_text_style`] instead.
    ///
    /// [`TextBoxStyleBuilder::from_text_style`]: #method.from_text_style
    #[inline]
    #[must_use]
    #[deprecated]
    pub fn text_style(self, text_style: MonoTextStyle<C, F>) -> Self {
        let mut text_style_builder = self.text_style_builder;

        if let Some(color) = text_style.background_color {
            text_style_builder = text_style_builder.background_color(color);
        }

        if let Some(color) = text_style.text_color {
            text_style_builder = text_style_builder.text_color(color);
        }

        Self {
            text_style_builder,
            ..self
        }
    }

    /// Sets the horizontal text alignment.
    #[inline]
    #[must_use]
    pub fn alignment<TA: HorizontalTextAlignment>(
        self,
        alignment: TA,
    ) -> TextBoxStyleBuilder<C, F, TA, V, H> {
        TextBoxStyleBuilder {
            text_style_builder: self.text_style_builder,
            alignment,
            line_spacing: self.line_spacing,
            vertical_alignment: self.vertical_alignment,
            height_mode: self.height_mode,
            tab_size: self.tab_size,
            underlined: self.underlined,
            strikethrough: self.strikethrough,
        }
    }

    /// Sets the vertical text alignment.
    #[inline]
    #[must_use]
    pub fn vertical_alignment<VA: VerticalTextAlignment>(
        self,
        vertical_alignment: VA,
    ) -> TextBoxStyleBuilder<C, F, A, VA, H> {
        TextBoxStyleBuilder {
            text_style_builder: self.text_style_builder,
            alignment: self.alignment,
            line_spacing: self.line_spacing,
            vertical_alignment,
            height_mode: self.height_mode,
            tab_size: self.tab_size,
            underlined: self.underlined,
            strikethrough: self.strikethrough,
        }
    }

    /// Sets the height mode.
    #[inline]
    #[must_use]
    pub fn height_mode<HM: HeightMode>(
        self,
        height_mode: HM,
    ) -> TextBoxStyleBuilder<C, F, A, V, HM> {
        TextBoxStyleBuilder {
            text_style_builder: self.text_style_builder,
            alignment: self.alignment,
            line_spacing: self.line_spacing,
            vertical_alignment: self.vertical_alignment,
            height_mode,
            tab_size: self.tab_size,
            underlined: self.underlined,
            strikethrough: self.strikethrough,
        }
    }

    /// Sets the tab size.
    #[inline]
    #[must_use]
    pub fn tab_size(self, tab_size: TabSize<F>) -> Self {
        Self { tab_size, ..self }
    }

    /// Enables or disables underlined text.
    #[inline]
    #[must_use]
    pub fn underlined(self, underlined: bool) -> Self {
        Self { underlined, ..self }
    }

    /// Enables or disables strikethrough text.
    #[inline]
    #[must_use]
    pub fn strikethrough(self, strikethrough: bool) -> Self {
        Self {
            strikethrough,
            ..self
        }
    }

    /// Builds the [`TextBoxStyle`].
    ///
    /// [`TextBoxStyle`]: ../struct.TextBoxStyle.html
    #[inline]
    #[must_use]
    pub fn build(self) -> TextBoxStyle<C, F, A, V, H> {
        TextBoxStyle {
            text_style: self.text_style_builder.build(),
            alignment: self.alignment,
            line_spacing: self.line_spacing,
            vertical_alignment: self.vertical_alignment,
            height_mode: self.height_mode,
            tab_size: self.tab_size,
            underlined: self.underlined,
            strikethrough: self.strikethrough,
        }
    }
}

#[cfg(test)]
mod test {
    use super::TextBoxStyleBuilder;
    use embedded_graphics::{
        fonts::Font6x8,
        pixelcolor::BinaryColor,
        style::{MonoTextStyle, MonoTextStyleBuilder},
    };

    #[test]
    #[allow(deprecated)]
    fn test_text_style_copy() {
        let text_styles: [MonoTextStyle<_, _>; 2] = [
            MonoTextStyleBuilder::new()
                .font(Font6x8)
                .text_color(BinaryColor::On)
                .build(),
            MonoTextStyleBuilder::new()
                .font(Font6x8)
                .background_color(BinaryColor::On)
                .build(),
        ];

        for &text_style in text_styles.iter() {
            let style = TextBoxStyleBuilder::new(Font6x8)
                .text_style(text_style)
                .build();

            assert_eq!(style.text_style, text_style);
        }
    }

    #[test]
    fn test_text_style_copy_ctr() {
        let text_styles: [MonoTextStyle<_, _>; 2] = [
            MonoTextStyleBuilder::new()
                .font(Font6x8)
                .text_color(BinaryColor::On)
                .build(),
            MonoTextStyleBuilder::new()
                .font(Font6x8)
                .background_color(BinaryColor::On)
                .build(),
        ];

        for &text_style in text_styles.iter() {
            let style = TextBoxStyleBuilder::from_text_style(text_style).build();

            assert_eq!(style.text_style, text_style);
        }
    }
}
