use crate::{
    parser::{expect, try_parse, try_parse_digit, Token, SPEC_CHAR_ESCAPE},
    style::color::Rgb,
};
use core::str::Chars;

fn try_parse_u8<'a>(chars: &mut Chars<'a>) -> Option<u8> {
    try_parse(chars, |chars| {
        if let Some(h) = try_parse_digit(chars) {
            if let Some(t) = try_parse_digit(chars) {
                if let Some(o) = try_parse_digit(chars) {
                    Some(100 * h + 10 * t + o)
                } else {
                    Some(10 * h + t)
                }
            } else {
                Some(h)
            }
        } else {
            None
        }
    })
}

fn try_parse_8b_color<'a>(chars: &mut Chars<'a>) -> Option<Rgb> {
    let color = try_parse_u8(chars)?;
    match color {
        //   0-  7:  standard colors (as in ESC [ 30–37 m)
        //   8- 15:  high intensity colors (as in ESC [ 90–97 m)
        0..=15 => Some(standard_to_rgb(color)),

        //  16-231:  6 × 6 × 6 cube (216 colors): 16 + 36 × r + 6 × g + b (0 ≤ r, g, b ≤ 5)
        16..=231 => {
            let color = color - 16;
            let extend_6 = |c| c * 51;

            let b = extend_6(color % 6);
            let color = color / 6;

            let g = extend_6(color % 6);
            let color = color / 6;

            let r = extend_6(color % 6);

            Some(Rgb::new(r, g, b))
        }

        // 232-255:  grayscale from black to white in 24 steps
        232..=255 => {
            let level = color - 232;
            let g = if level == 23 { 255 } else { level * 11 };
            Some(Rgb::new(g, g, g))
        }
    }
}

fn try_parse_rgb<'a>(chars: &mut Chars<'a>) -> Option<Rgb> {
    let r = try_parse_u8(chars)?;
    expect(chars, ';')?;
    let g = try_parse_u8(chars)?;
    expect(chars, ';')?;
    let b = try_parse_u8(chars)?;

    Some(Rgb::new(r, g, b))
}

fn standard_to_rgb(idx: u8) -> Rgb {
    // These colors are used in PowerShell 6 in Windows 10
    match idx {
        0 => Rgb::new(12, 12, 12),
        1 => Rgb::new(197, 15, 31),
        2 => Rgb::new(19, 161, 14),
        3 => Rgb::new(193, 156, 0),
        4 => Rgb::new(0, 55, 218),
        5 => Rgb::new(136, 23, 152),
        6 => Rgb::new(58, 150, 221),
        7 => Rgb::new(204, 204, 204),

        8 => Rgb::new(118, 118, 118),
        9 => Rgb::new(231, 72, 86),
        10 => Rgb::new(22, 198, 12),
        11 => Rgb::new(249, 241, 165),
        12 => Rgb::new(59, 120, 255),
        13 => Rgb::new(180, 0, 158),
        14 => Rgb::new(97, 214, 214),
        _ => Rgb::new(242, 242, 242),
    }
}

fn try_parse_color<'a>(chars: &mut Chars<'a>) -> Option<Rgb> {
    expect(chars, ';')?;
    let color_type = try_parse_u8(chars)?;
    expect(chars, ';')?;

    match color_type {
        2 => try_parse_rgb(chars),
        5 => try_parse_8b_color(chars),

        _ => None,
    }
}

pub fn try_parse_escape_seq<'a>(chars: &mut Chars<'a>) -> Option<Token<'a>> {
    try_parse(chars, |chars| {
        chars.next().and_then(|c| match c {
            SPEC_CHAR_ESCAPE => Some(Token::Escape),
            '[' => {
                let code = try_parse_u8(chars)?;
                // limitation: only a single attribute is supported at a time

                let possible_token = match code {
                    30..=37 => Some(Token::ChangeTextColor(standard_to_rgb(code - 30))),
                    38 => {
                        let color = try_parse_color(chars)?;
                        Some(Token::ChangeTextColor(color))
                    }
                    90..=97 => Some(Token::ChangeTextColor(standard_to_rgb(code - 82))),
                    40..=47 => Some(Token::ChangeBackgroundColor(standard_to_rgb(code - 40))),
                    48 => {
                        let color = try_parse_color(chars)?;
                        Some(Token::ChangeBackgroundColor(color))
                    }
                    100..=107 => Some(Token::ChangeBackgroundColor(standard_to_rgb(code - 92))),
                    _ => None,
                };

                expect(chars, 'm')?;
                possible_token
            }
            _ => None,
        })
    })
}
