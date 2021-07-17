//! # Example: editor
//!
//! This example demonstrates a simple text "editor" that lets you type and delete characters.
//!
//! The demo uses the "Scrolling" vertical layout which is especially useful for
//! editor type applications.
use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{iso_8859_2::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{
        renderer::{CharacterStyle, TextRenderer},
        Baseline,
    },
};
use embedded_graphics_simulator::{
    sdl2::MouseButton, BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent,
    Window,
};
use embedded_text::{plugin::Plugin, Cursor as RenderingCursor, TextBox, TextBoxProperties};
use sdl2::keyboard::{Keycode, Mod};
use std::{
    cell::RefCell, collections::HashMap, convert::Infallible, rc::Rc, thread, time::Duration,
};

trait StrExt {
    fn first_n_chars<'a>(&'a self, n: usize) -> &'a str;
}

impl StrExt for str {
    fn first_n_chars<'a>(&'a self, n: usize) -> &'a str {
        if let Some((i, (idx, _))) = self.char_indices().enumerate().take(n + 1).last() {
            if i < n as usize {
                self
            } else {
                &self[0..idx]
            }
        } else {
            self
        }
    }
}

trait Selector {
    /// Select inserted characters based on key modifiers.
    ///
    /// Some key combinations don't insert characters, so we have to work with strings.
    fn select_modified(&self, keymod: Mod) -> &str;
}

impl Selector for (&str, &str, &str, &str) {
    #[inline]
    fn select_modified(&self, keymod: Mod) -> &str {
        if keymod.contains(Mod::RALTMOD) {
            self.3
        } else if keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
            self.1
        } else if keymod.contains(Mod::CAPSMOD) {
            self.2
        } else {
            self.0
        }
    }
}

pub struct Cursor {
    offset: usize,
    pos: Option<Point>,
    old_desired_position: DesiredPosition,
    desired_position: DesiredPosition,
}

impl Cursor {
    fn plugin<'a, C: PixelColor>(&'a mut self, color: C) -> EditorPlugin<'a, C> {
        EditorPlugin {
            cursor_position: Point::zero(),
            current_offset: 0,
            desired_cursor_position: self.desired_position,
            old_desired_position: self.old_desired_position,
            color,
            cursor_drawn: false,
            cursor: Rc::new(RefCell::new(self)),
        }
    }
}

struct EditorInput {
    pub text: String,
    pub cursor: Cursor,
}

impl EditorInput {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: Cursor {
                offset: text.len(),
                pos: None,

                old_desired_position: DesiredPosition::EndOfText,
                desired_position: DesiredPosition::EndOfText,
            },
        }
    }

    pub fn insert(&mut self, s: &str) {
        self.text.insert_str(self.cursor.offset, s);
        self.cursor.offset += s.len();
        self.cursor.desired_position = DesiredPosition::Offset(self.cursor.offset);
    }

    pub fn delete_before(&mut self) {
        if self.cursor.offset > 0 {
            self.cursor.offset -= 1;
            self.cursor.desired_position = DesiredPosition::Offset(self.cursor.offset);
            self.text.remove(self.cursor.offset);
        }
    }

    pub fn delete_after(&mut self) {
        if self.cursor.offset < self.text.chars().count() {
            self.text.remove(self.cursor.offset);
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor.offset > 0 {
            self.cursor.desired_position = DesiredPosition::Offset(self.cursor.offset - 1);
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor.offset < self.text.len() {
            self.cursor.desired_position = DesiredPosition::Offset(self.cursor.offset + 1);
        }
    }

    pub fn cursor_up(&mut self) {
        self.cursor.old_desired_position = self.cursor.desired_position;
        self.cursor.desired_position = DesiredPosition::OneLineUp;
    }

    pub fn cursor_down(&mut self) {
        self.cursor.old_desired_position = self.cursor.desired_position;
        self.cursor.desired_position = DesiredPosition::OneLineDown;
    }

    pub fn move_cursor_to(&mut self, point: Point) {
        self.cursor.desired_position = DesiredPosition::Coordinates(point);
    }
}

#[derive(Clone, Copy, Debug)]
enum DesiredPosition {
    OneLineUp,
    OneLineDown,
    EndOfText,
    Offset(usize),
    Coordinates(Point),
}

#[derive(Clone)]
struct EditorPlugin<'a, C> {
    cursor: Rc<RefCell<&'a mut Cursor>>,
    desired_cursor_position: DesiredPosition,
    old_desired_position: DesiredPosition,
    cursor_position: Point,
    current_offset: usize,
    color: C,
    cursor_drawn: bool,
}

impl<C: PixelColor> EditorPlugin<'_, C> {
    #[track_caller]
    fn draw_cursor<D>(
        &mut self,
        draw_target: &mut D,
        bounds: Rectangle,
        pos: Point,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>,
    {
        self.cursor_position = pos;
        self.cursor_drawn = true;

        let style = PrimitiveStyle::with_stroke(self.color, 1);
        Line::new(
            pos + Point::new(0, 1),
            pos + Point::new(0, bounds.size.height as i32 - 1),
        )
        .into_styled(style)
        .draw(draw_target)
    }
}

impl<'a, C: PixelColor> Plugin<'a, C> for EditorPlugin<'_, C> {
    fn on_start_render<S: CharacterStyle + TextRenderer>(
        &mut self,
        _cursor: &mut RenderingCursor,
        props: &TextBoxProperties<'_, S>,
    ) {
        let line_height = props.char_style.line_height() as i32;
        let cursor_position = self.cursor.borrow().pos;
        let old_desired_position =
            if let DesiredPosition::Coordinates(point) = self.old_desired_position {
                point
            } else {
                cursor_position.unwrap_or_default()
            };

        // Limiting here results in better behavior when returning from a clipped top/bottom position.
        let old_desired_position = Point::new(
            old_desired_position.x,
            old_desired_position
                .y
                .max(0)
                .min(props.text_height - line_height),
        );

        match self.desired_cursor_position {
            DesiredPosition::OneLineUp => {
                self.desired_cursor_position = DesiredPosition::Coordinates(
                    old_desired_position + Point::new(0, -line_height),
                );
            }
            DesiredPosition::OneLineDown => {
                self.desired_cursor_position =
                    DesiredPosition::Coordinates(old_desired_position + Point::new(0, line_height));
            }
            _ => {}
        }
    }

    fn post_render<T, D>(
        &mut self,
        draw_target: &mut D,
        character_style: &T,
        text: &str,
        bounds: Rectangle,
    ) -> Result<(), D::Error>
    where
        T: TextRenderer<Color = C>,
        D: DrawTarget<Color = T::Color>,
    {
        if self.cursor_drawn {
            return Ok(());
        }

        let len = text.chars().count();
        match self.desired_cursor_position {
            DesiredPosition::EndOfText => {
                if text == "" {
                    self.draw_cursor(draw_target, bounds, bounds.top_left)?;
                    self.current_offset += len;
                }
            }

            DesiredPosition::Offset(desired_offset) => {
                let current_offset = self.current_offset;

                if (len == 0 && current_offset == desired_offset)
                    || (current_offset..current_offset + len).contains(&desired_offset)
                {
                    let chars_before = desired_offset - current_offset;
                    let pos = if chars_before == 0 {
                        bounds.top_left
                    } else {
                        let str_before = text.first_n_chars(chars_before);
                        let metrics = character_style.measure_string(
                            str_before,
                            bounds.top_left,
                            Baseline::Top,
                        );
                        // we want the start of the next character, not the end of the last, hence the +
                        metrics.bounding_box.anchor_point(AnchorPoint::TopRight) + Point::new(1, 0)
                    };
                    self.draw_cursor(draw_target, bounds, pos)?;
                    self.current_offset += chars_before;
                }
            }

            DesiredPosition::Coordinates(point) => {
                let same_line = point.y >= bounds.top_left.y
                    && point.y < bounds.anchor_point(AnchorPoint::BottomRight).y;
                if same_line {
                    if bounds.contains(point) {
                        let mut anchor_point = bounds.anchor_point(AnchorPoint::TopLeft);
                        // Figure out the number of drawn characters, set cursor position
                        for i in 0..len.saturating_sub(1) {
                            // TODO: it might be faster to measure one character at a time and
                            // sum up the width manually.
                            let str_before = text.first_n_chars(i + 1);
                            let metrics = character_style.measure_string(
                                str_before,
                                bounds.top_left,
                                Baseline::Top,
                            );
                            if metrics.bounding_box.contains(point) {
                                self.draw_cursor(draw_target, bounds, anchor_point)?;
                                self.current_offset += i;
                                break;
                            }
                            anchor_point = metrics.bounding_box.anchor_point(AnchorPoint::TopRight);
                        }

                        if !self.cursor_drawn {
                            // The cursor is right before the last character (unmeasured).
                            self.draw_cursor(draw_target, bounds, anchor_point)?;
                            self.current_offset += len.saturating_sub(1);
                        }
                    } else if text == "\n" || text == "" {
                        self.draw_cursor(draw_target, bounds, bounds.top_left)?;
                    }
                } else if text == "" || point.y < bounds.top_left.y {
                    // end of text, or cursor is positioned before the text begins
                    self.draw_cursor(draw_target, bounds, bounds.top_left)?;
                }
            }

            other => unreachable!("{:?} should have been replaced in on_start_render", other),
        }

        if !self.cursor_drawn {
            self.current_offset += len;
        }

        Ok(())
    }

    fn end_of_text(&mut self) {
        // Update the parent object's cursor with the actual info.
        let mut cursor = self.cursor.borrow_mut();

        cursor.pos.insert(self.cursor_position);
        cursor.offset = self.current_offset;
        cursor.desired_position = self.desired_cursor_position;
    }
}

fn main() -> Result<(), Infallible> {
    // Special characters are mapped as they appear on Hungarian layouts. Sorry 😅
    let inputs: HashMap<_, _> = [
        // (Keycode, (NO, SHIFT, CAPS, ALT_GR))
        (Keycode::A, ("a", "A", "A", "ä")),
        (Keycode::B, ("b", "B", "B", "{")),
        (Keycode::C, ("c", "C", "C", "&")),
        (Keycode::D, ("d", "D", "D", "Đ")),
        (Keycode::E, ("e", "E", "E", "Ä")),
        (Keycode::F, ("f", "F", "F", "[")),
        (Keycode::G, ("g", "G", "G", "]")),
        (Keycode::H, ("h", "H", "H", "")),
        (Keycode::I, ("i", "I", "I", "Í")),
        (Keycode::J, ("j", "J", "J", "í")),
        (Keycode::K, ("k", "K", "K", "ł")),
        (Keycode::L, ("l", "L", "L", "Ł")),
        (Keycode::M, ("m", "M", "M", "<")),
        (Keycode::N, ("n", "N", "N", "}")),
        (Keycode::O, ("o", "O", "O", "")),
        (Keycode::P, ("p", "P", "P", "")),
        (Keycode::Q, ("q", "Q", "Q", "\\")),
        (Keycode::R, ("r", "R", "R", "")),
        (Keycode::S, ("s", "S", "S", "đ")),
        (Keycode::T, ("t", "T", "T", "")),
        (Keycode::U, ("u", "U", "U", "€")),
        (Keycode::V, ("v", "V", "V", "@")),
        (Keycode::W, ("w", "W", "W", "|")),
        (Keycode::X, ("x", "X", "X", "#")),
        (Keycode::Y, ("y", "Y", "Y", ">")),
        (Keycode::Z, ("z", "Z", "Z", "")),
        (Keycode::Num0, ("0", "§", "0", "")),
        (Keycode::Num1, ("1", "'", "1", "~")),
        (Keycode::Num2, ("2", "\"", "2", "ˇ")),
        (Keycode::Num3, ("3", "+", "3", "^")),
        (Keycode::Num4, ("4", "!", "4", "˘")),
        (Keycode::Num5, ("5", "%", "5", "°")),
        (Keycode::Num6, ("6", "/", "6", "˛")),
        (Keycode::Num7, ("7", "=", "7", "`")),
        (Keycode::Num8, ("8", "(", "8", "˙")),
        (Keycode::Num9, ("9", ")", "9", "´")),
        (Keycode::Space, (" ", " ", " ", " ")),
        (Keycode::Comma, (",", "?", ",", " ")),
        (Keycode::Period, (".", ":", ".", ">")),
        (Keycode::Minus, ("-", "_", "-", "*")),
        (Keycode::Return, ("\n", "\n", "\n", "\n")),
        (Keycode::KpEnter, ("\n", "\n", "\n", "\n")),
    ]
    .iter()
    .cloned()
    .collect();

    // Set up the window.
    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::OledBlue)
        .scale(2)
        .build();
    let mut window = Window::new("Interactive TextBox input demonstration", &output_settings);

    let character_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut input = EditorInput::new("Hello, \n\nWorld!");

    'demo: loop {
        // Create a simulated display with the dimensions of the text box.
        let mut display = SimulatorDisplay::new(Size::new(128, 64));

        // Display an underscore for the "cursor"
        // Create the text box and apply styling options.
        TextBox::new(&input.text, display.bounding_box(), character_style)
            .add_plugin(input.cursor.plugin(BinaryColor::On))
            .draw(&mut display)?;

        // Update the window.
        window.update(&display);

        // Handle key events.
        for event in window.events() {
            match event {
                SimulatorEvent::KeyDown {
                    keycode, keymod, ..
                } => match keycode {
                    Keycode::Backspace => input.delete_before(),

                    Keycode::Delete => input.delete_after(),
                    Keycode::Left => input.cursor_left(),
                    Keycode::Right => input.cursor_right(),
                    Keycode::Up => input.cursor_up(),
                    Keycode::Down => input.cursor_down(),

                    _ => {
                        if let Some(k) = inputs.get(&keycode) {
                            input.insert(k.select_modified(keymod));
                        }
                    }
                },
                SimulatorEvent::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    point,
                } => input.move_cursor_to(point),
                SimulatorEvent::Quit => break 'demo,
                _ => {}
            }
        }

        // Wait for a little while.
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
