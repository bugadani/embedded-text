//! # Example: tab characters
//!
//! This example demonstrates support for the horizontal tab `\t` character.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::LineHeight,
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
};
use embedded_text::{
    plugin::ansi::Ansi,
    style::{HeightMode, TabSize, TextBoxStyleBuilder},
    TextBox,
};

fn main() {
    // A simple example table
    let text = &format!(
        "{underlined}Column A\t|Column B\t|Column C{underlined_off}\n\
        Foo\t|Bar\t|Baz\n\
        1\t|2\t|3",
        underlined = "\x1b[4m",
        underlined_off = "\x1b[24m",
    );

    // Specify the styling options:
    // * Use the 6x10 MonoFont from embedded-graphics.
    // * Draw the text horizontally left aligned (default option, not specified here).
    // * Use `FitToText` height mode to stretch the text box to the exact height of the text.
    // * Draw the text with `BinaryColor::On`, which will be displayed as light blue.
    // * 10 character wide tabs
    let character_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .tab_size(TabSize::Spaces(10))
        .line_height(LineHeight::Pixels(11))
        .build();

    // Specify the bounding box. Note the 0px height. The `FitToText` height mode will
    // measure and adjust the height of the text box in `into_styled()`.
    let bounds = Rectangle::new(Point::zero(), Size::new(180, 0));

    // Create the text box and apply styling options.
    let text_box = TextBox::with_textbox_style(text, bounds, character_style, textbox_style)
        .add_plugin(Ansi::new());

    // Create a simulated display with the dimensions of the text box.
    let mut display = SimulatorDisplay::new(text_box.bounding_box().size);

    // Draw the text box.
    text_box.draw(&mut display).unwrap();

    // Set up the window and show the display's contents.
    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledBlue)
        .scale(2)
        .build();
    Window::new("TextBox tab support example", &output_settings).show_static(&display);
}
