use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
};

use embedded_graphics::{fonts::Font6x8, pixelcolor::BinaryColor, prelude::*};

use embedded_text::{alignment::horizontal::RightAligned, prelude::*};

fn main() -> Result<(), core::convert::Infallible> {
    let text = "Hello, World!\nLorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book.";

    let textbox_style = TextBoxStyleBuilder::new(Font6x8)
        .alignment(RightAligned)
        .text_color(BinaryColor::On)
        .build();

    let height = textbox_style.measure_text_height(text, 129);

    // Create a window just tall enough to fit the text.
    let mut display: SimulatorDisplay<BinaryColor> = SimulatorDisplay::new(Size::new(129, height));

    TextBox::new(
        text,
        Rectangle::new(Point::zero(), Point::new(128, height as i32 - 1)),
    )
    .into_styled(textbox_style)
    .draw(&mut display)
    .unwrap();

    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledBlue)
        .build();
    Window::new("Hello right aligned TextBox", &output_settings).show_static(&display);
    Ok(())
}
