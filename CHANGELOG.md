Unreleased
==========

## Changed:

 * **breaking** Removed deprecated `TextBoxStyleBuilder::text_style`. Use `TextBoxStyleBuilder::from_text_style` instead.
 * **breaking** Removed `Rectangle` extensions.
 * ANSI sequence support now requires the `ansi` feature which is on by default.
 * Fields of the `style::color::Rgb` struct are now public.

0.4.0 (2020-11-26)
==================

## Added:

 * Added support for strikethrough and underlined text.
 * `RendererFactory` trait that can be used to create a pixel iterator.
 * Handle tabs `\t` with configurable tab size.
 * Added `TabSize` struct and related style builder method `tab_size`.
 * Added partial support for [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code).
 * `Scrolling` vertical alignment
 * `TextBoxStyleBuilder` now implements `Copy` and `Clone`
 * `TextBox` and `StyledTextBox` now implements `Copy`, `Clone`, `Debug`, `Eq`, `PartialEq` and `Hash`

## Changed:

 * **breaking** Left aligned text now eats a single white space at the end of a wrapped line. This changes some height measurements and rendering output.
 * **breaking** `TextBoxStyle::measure_line` now returns whether the line is underlined.
 * **breaking** Renamed `StyledCharacterIterator` to `CharacterIterator`
 * **breaking** Increase the Minimum Supported Rust Version to `1.41.0`
 * **breaking** `rendering::line_iter::State` is no longer public
 * **breaking** `rendering::line::State` is no longer public
 * **breaking** Removed `StateFactory`
 * **breaking** Removed `FontExt::str_width`, `FontExt::max_str_width` and `FontExt::max_str_width_nocr`
 * **breaking** `TextBoxStyle` and `TextBoxStyleBuilder` no longer derives `Ord` and `PartialOrd`

0.3.0 (2020-10-02)
==================

## Added:

 * `TextBoxStyleBuilder::from_text_style`
 * Added `HeightMode` to select whether and how the `StyledTextBox` height should be aligned to the
   actual text height.
 * Added `Hidden`, `Visible` and `FullRowsOnly` overflow control modes to `Exact` and `ShrinkToText` height modes.
 * Added line spacing support via `TextBoxStyleBuilder::line_spacing`
 * Soft hyphen character support (`\u{AD}`), rendered as a normal `-`.

## Changed:

 * Deprecated `TextBoxStyleBuilder::text_style`. Use `TextBoxStyleBuilder::from_text_style` instead.
 * **breaking** Moved `StyledTextBox` to the root module.
 * Added `StyledTextBox::fit_height` and `StyledTextBox::fit_height_limited` to adjust height to text

## Fixed:

 * Fix `CenterAligned` and `BottomAligned` vertical alignments crashing the program when text is
   taller than the `TextBox`.

0.2.0 (2020-08-15)
==================

## Added:

 * Support for vertical text alignment.
 * Added alignment types to `prelude`.
 * Support for zero-width space character (`\u{200B}`).
 * Support for nonbreaking space character (`\u{A0}`).
 * Added optimized measurement function that do not expect carriage returns.
   * `FontExt::measure_line_nocr`
   * `FontExt::str_width_nocr`
   * `FontExt::max_str_width_nocr`
 * Support carriage return (`\r`) control characters.

## Fixed:

 * Fixed an issue where height measurement unexpectedly carried a space that is consumed during drawing.

0.1.0 (2020-07-31)
==================

## Added:

 * Added `TextBoxStyle::from_text_style`

## Changed:

 * **breaking:** Renamed `measure_text` to `measure_text_height`
 * **breaking:** Moved `measure_text` from `FontExt` to `TextBoxStyle`
 * **breaking:** Removed `FontExt` from `prelude`

## Fixed:

 * Fixed an issue where after a line break, the last line was not rendered if the exact height was available.
 * Fixed several text height measurement issues and inconsistencies.
 * Fixed an issue where text height was measured incorrectly when encountering words wider than line.
 * Fixed an issue where characters could be drawn outside of the bounding box when there is not enough space to render a single character.
 * Fixed a word wrapping issue where the first space may sometimes be rendered 0 width.
 * Crash with `Justified` alignment.

0.0.3 (2020-07-28)
==================

## Added:

 * `prelude` import
 * Render fonts with variable character width
 * `FontExt::measure_text` to measure height using a certain width. Implemented for all `Font` instances.

0.0.1 (2020-07-21)
==================
 * Initial release
