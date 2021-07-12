//! # Example: Styling with ANSI sequences
//!
//! This example demonstrates text styling using in-line ANSI escape sequences.
//!
//! Note: you need to enable the `ansi` feature to use the ANSI sequence support.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    text::LineHeight,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use embedded_text::{plugin::ansi::Ansi, style::TextBoxStyleBuilder, TextBox};
use std::convert::Infallible;

fn main() -> Result<(), Infallible> {
    let text = format!(
        "{comment}/// Comment\n\
        {base_text}#[{attribute}derive{base_text}(Debug)]\n\
        {keyword}enum {type_name}{underlined}Foo{underlined_off}{base_text}<{lifetime}'a{base_text}> {{\n\
        {comment}\t/// Decide what {strikethrough}not{strikethrough_off} to do next.\n\
        {highlighted_background}\t{enum_variant}Bar{base_text}({type_name}{underlined}Token{underlined_off}{base_text}<{lifetime}'a{base_text}>),{end_of_line}\n\
        {line_background}{base_text}}}",
        // Name the ANSI escape sequences we use for styling
        line_background = "\x1b[48;5;16m",
        highlighted_background = "\x1b[48;5;235m",
        enum_variant = "\x1b[38;2;36;144;241m",
        keyword = "\x1b[38;2;84;128;166m",
        comment = "\x1b[38;2;94;153;73m",
        base_text = "\x1b[97m",
        attribute ="\x1b[38;2;220;220;157m",
        type_name = "\x1b[38;2;78;201;176m",
        lifetime = "\x1b[38;2;84;128;166m",
        end_of_line = "\x1b[40C",
        underlined = "\x1b[4m",
        underlined_off = "\x1b[24m",
        strikethrough = "\x1b[9m",
        strikethrough_off = "\x1b[29m",
    );

    let character_style = MonoTextStyle::new(&FONT_6X10, Rgb888::BLACK);
    let textbox_style = TextBoxStyleBuilder::new()
        .line_height(LineHeight::Percent(125))
        .build();

    let mut display = SimulatorDisplay::new(Size::new(241, 97));

    TextBox::with_textbox_style(
        &text,
        display.bounding_box(),
        character_style,
        textbox_style,
    )
    .add_plugin(Ansi::new())
    .draw(&mut display)?;

    // Set up the window and show the display's contents.
    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    Window::new("In-line styling example", &output_settings).show_static(&display);

    Ok(())
}
