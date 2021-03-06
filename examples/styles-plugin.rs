//! # Example: styling using plugin.
//!
//! This example demonstrates plugin that affects styling.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::DecorationColor,
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
};
use embedded_text::{
    alignment::HorizontalAlignment, plugin::Plugin, style::TextBoxStyle, ChangeTextStyle, TextBox,
    Token,
};
use std::convert::Infallible;

#[derive(Clone)]
struct Underliner<'a, C>
where
    C: PixelColor,
{
    underlined: bool,
    current_token: Option<Token<'a, C>>,
}

impl<'a, C: PixelColor> Underliner<'a, C> {
    fn new() -> Self {
        Self {
            underlined: false,
            current_token: None,
        }
    }

    fn process_token(
        &mut self,
        token: Option<Token<'a, C>>,
        substitute_underline: impl FnOnce(&mut Self) -> Option<Token<'a, C>>,
    ) -> Option<Token<'a, C>> {
        match token {
            Some(Token::Word(w)) => {
                if let Some(pos) = w.find('_') {
                    if pos == 0 {
                        self.current_token = Some(Token::Word(&w[1..]));
                        substitute_underline(self)
                    } else {
                        let prefix = &w[0..pos];
                        self.current_token = Some(Token::Word(&w[pos..]));
                        Some(Token::Word(prefix))
                    }
                } else {
                    Some(Token::Word(w))
                }
            }
            token => token,
        }
    }
}

impl<'a, C> Plugin<'a, C> for Underliner<'a, C>
where
    C: PixelColor,
{
    fn next_token(
        &mut self,
        mut next_token: impl FnMut() -> Option<Token<'a, C>>,
    ) -> Option<Token<'a, C>> {
        let token = if let Some(token) = self.current_token.take() {
            Some(token)
        } else {
            next_token()
        };

        self.process_token(token, |this| {
            this.underlined = !this.underlined;

            Some(Token::ChangeTextStyle(ChangeTextStyle::Underline(
                if this.underlined {
                    DecorationColor::TextColor
                } else {
                    DecorationColor::None
                },
            )))
        })
    }
}

fn main() -> Result<(), Infallible> {
    let text = "Hello, World!\n\
    Lorem Ipsum is _simply_ dummy text of the printing and typesetting industry. \
    Lorem Ipsum has been the industry's _standard_ dummy text ever since the 1500s, when \
    an unknown printer _took a galley of type and scrambled it_ to make a type specimen book.";

    // Create a simulated display.
    let mut display = SimulatorDisplay::new(Size::new(128, 140));

    let character_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let textbox_style = TextBoxStyle::with_alignment(HorizontalAlignment::Justified);

    // Specify the bounding box. Note the 0px height. The `FitToText` height mode will
    // measure and adjust the height of the text box in `into_styled()`.
    let bounds = Rectangle::new(Point::zero(), display.size());

    // Create and draw the text boxes.
    TextBox::with_textbox_style(text, bounds, character_style, textbox_style)
        .add_plugin(Underliner::new())
        .draw(&mut display)?;

    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledBlue)
        .scale(2)
        .build();
    Window::new("TextBox plugin demonstration", &output_settings).show_static(&display);

    Ok(())
}
