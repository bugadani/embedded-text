//! Plugin trait.

use embedded_graphics::{
    draw_target::DrawTarget,
    prelude::PixelColor,
    primitives::Rectangle,
    text::renderer::{CharacterStyle, TextRenderer},
};
use object_chain::{Chain, ChainElement, Link};

use crate::{parser::Token, rendering::cursor::Cursor, TextBoxProperties};

/// Plugin trait.
///
/// Plugins allow modifying and extending TextBox's internals.
///
/// *Important*:
/// This is an experimental, unstable feature. It can be, and probably will be modified without
/// any prior notice.
/// Using plugins require enabling the `plugin` crate feature.
pub trait Plugin<'a, C>: Clone
where
    C: PixelColor,
{
    /// Called when a new line is started.
    #[inline]
    fn new_line(&mut self) {}

    /// Called after rendering has finished.
    #[inline]
    fn end_of_text(&mut self) {}

    /// Generate the next text token.
    #[inline]
    fn next_token(
        &mut self,
        mut next_token: impl FnMut() -> Option<Token<'a, C>>,
    ) -> Option<Token<'a, C>> {
        next_token()
    }

    /// Modify the current token immediately before it is rendered.
    ///
    /// This function must return the same token type as the input, otherwise the returned token
    /// is ignored.
    #[inline]
    fn render_token(&mut self, token: Token<'a, C>) -> Option<Token<'a, C>> {
        Some(token)
    }

    /// Called after a piece of text is rendered.
    #[inline]
    fn post_render<T, D>(
        &mut self,
        _draw_target: &mut D,
        _character_style: &T,
        _text: &str,
        _bounds: Rectangle,
    ) -> Result<(), D::Error>
    where
        T: TextRenderer<Color = C>,
        D: DrawTarget<Color = C>,
    {
        Ok(())
    }

    /// Called before TextBox rendering is started.
    #[inline]
    fn on_start_render<S: CharacterStyle>(
        &mut self,
        _cursor: &mut Cursor,
        _props: TextBoxProperties<'_, S>,
    ) {
    }
}

impl<'a, C> Plugin<'a, C> for super::NoPlugin<C> where C: PixelColor {}

impl<'a, C, P> Plugin<'a, C> for Chain<P>
where
    P: Plugin<'a, C>,
    C: PixelColor,
    Chain<P>: Clone,
{
    fn new_line(&mut self) {
        self.object.new_line();
    }

    fn end_of_text(&mut self) {
        self.object.end_of_text();
    }

    fn next_token(
        &mut self,
        next_token: impl FnMut() -> Option<Token<'a, C>>,
    ) -> Option<Token<'a, C>> {
        self.object.next_token(next_token)
    }

    fn render_token(&mut self, token: Token<'a, C>) -> Option<Token<'a, C>> {
        self.object.render_token(token)
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
        D: DrawTarget<Color = C>,
    {
        self.object
            .post_render(draw_target, character_style, text, bounds)
    }

    fn on_start_render<S: CharacterStyle>(
        &mut self,
        cursor: &mut Cursor,
        props: TextBoxProperties<'_, S>,
    ) {
        self.object.on_start_render(cursor, props)
    }
}

impl<'a, C, P, CE> Plugin<'a, C> for Link<P, CE>
where
    CE: ChainElement + Plugin<'a, C>,
    P: Plugin<'a, C>,
    C: PixelColor,
    Link<P, CE>: Clone,
{
    fn new_line(&mut self) {
        self.parent.new_line();
        self.object.new_line();
    }

    fn end_of_text(&mut self) {
        self.parent.end_of_text();
        self.object.end_of_text();
    }

    fn next_token(
        &mut self,
        mut next_token: impl FnMut() -> Option<Token<'a, C>>,
    ) -> Option<Token<'a, C>> {
        let parent = &mut self.parent;
        let next_token = || parent.next_token(&mut next_token);
        self.object.next_token(next_token)
    }

    fn render_token(&mut self, token: Token<'a, C>) -> Option<Token<'a, C>> {
        self.parent
            .render_token(token)
            .and_then(|t| self.object.render_token(t))
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
        D: DrawTarget<Color = C>,
    {
        self.parent
            .post_render(draw_target, character_style, text, bounds)?;
        self.object
            .post_render(draw_target, character_style, text, bounds)
    }

    fn on_start_render<S: CharacterStyle>(
        &mut self,
        cursor: &mut Cursor,
        props: TextBoxProperties<'_, S>,
    ) {
        self.parent.on_start_render(cursor, props.clone());
        self.object.on_start_render(cursor, props);
    }
}
