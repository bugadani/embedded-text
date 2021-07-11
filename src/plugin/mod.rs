//! Plugins allow changing TextBox behaviour.
//!
//! Note: Custom plugins are experimental. Ff you wish to implement custom plugins,
//! you need to activate the `plugin` feature.

use core::{
    cell::RefCell,
    hash::{Hash, Hasher},
    marker::PhantomData,
};
use embedded_graphics::{
    draw_target::DrawTarget,
    prelude::PixelColor,
    primitives::Rectangle,
    text::renderer::{CharacterStyle, TextRenderer},
};

use crate::{
    parser::{Parser, Token},
    rendering::cursor::Cursor,
    TextBoxProperties,
};

#[cfg(feature = "plugin")]
pub mod private;
#[cfg(feature = "plugin")]
pub use private::Plugin;

#[cfg(not(feature = "plugin"))]
mod private;
#[cfg(not(feature = "plugin"))]
use private::Plugin;

pub mod tail;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum ProcessingState {
    Measure,
    Render,
}

/// Placeholder type when no plugin is used.
#[derive(Clone, Copy, Default)]
pub struct NoPlugin<C>(PhantomData<C>)
where
    C: PixelColor;

impl<C> NoPlugin<C>
where
    C: PixelColor,
{
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

/// Plugin marker trait.
///
/// This trait is an implementation detail. Most likely you don't need to implement this.
#[cfg_attr(
    feature = "plugin",
    doc = "If you wish to implement a plugin, see [Plugin]."
)]
// TODO: remove this trait once Plugin is stabilized, then move Plugin here
pub trait PluginMarker<'a, C: PixelColor>: Plugin<'a, C> {}

impl<'a, C, T> PluginMarker<'a, C> for T
where
    T: Plugin<'a, C>,
    C: PixelColor,
{
}

#[derive(Clone, Debug)]
pub(crate) struct PluginInner<'a, M, C>
where
    C: PixelColor,
{
    lookahead: M,
    pub plugin: M,
    state: ProcessingState,
    peeked_token: (usize, Option<Token<'a, C>>),
}

#[derive(Clone, Debug)]
pub(crate) struct PluginWrapper<'a, M, C>
where
    C: PixelColor,
{
    pub inner: RefCell<PluginInner<'a, M, C>>,
}

impl<'a, M, C> Hash for PluginWrapper<'a, M, C>
where
    C: PixelColor,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.borrow().state.hash(state)
    }
}

impl<'a, M, C> PluginWrapper<'a, M, C>
where
    C: PixelColor,
    M: private::Plugin<'a, C>,
{
    pub fn new(plugin: M) -> Self {
        Self {
            inner: RefCell::new(PluginInner {
                lookahead: plugin.clone(),
                plugin,
                state: ProcessingState::Measure,
                peeked_token: (0, None),
            }),
        }
    }

    pub fn new_line(&self) {
        let mut this = self.inner.borrow_mut();
        this.peeked_token.0 = 0;
        this.peeked_token.1 = None;
        this.plugin.new_line();

        this.lookahead = this.plugin.clone();
    }

    pub fn end_of_text(&self) {
        let mut this = self.inner.borrow_mut();

        this.lookahead.end_of_text();
    }

    pub fn set_state(&self, state: ProcessingState) {
        self.inner.borrow_mut().state = state;
    }

    #[inline]
    pub fn render_token(&self, token: Token<'a, C>) -> Option<Token<'a, C>> {
        let mut this = self.inner.borrow_mut();
        match this.state {
            ProcessingState::Measure => Some(token),
            ProcessingState::Render => this.lookahead.render_token(token),
        }
    }

    pub fn peek_token(&self, source: &mut Parser<'a, C>) -> Option<Token<'a, C>> {
        let mut this = self.inner.borrow_mut();

        if this.peeked_token.1.is_none() {
            let mut cloned = source.clone();
            this.peeked_token.1 = this.lookahead.next_token(|| cloned.next());
            this.peeked_token.0 = source.as_str().len() - cloned.as_str().len();
        }
        this.peeked_token.1.clone()
    }

    pub fn consume_peeked_token(&self, source: &mut Parser<'a, C>) {
        let mut this = self.inner.borrow_mut();

        unsafe {
            source.consume(this.peeked_token.0);
        }
        this.peeked_token.0 = 0;
        this.peeked_token.1 = None;

        this.plugin = this.lookahead.clone();
    }

    pub fn replace_peeked_token(&self, len: usize, token: Token<'a, C>) {
        let mut this = self.inner.borrow_mut();

        this.peeked_token.0 = len;
        this.peeked_token.1.replace(token);

        this.lookahead = this.plugin.clone();
    }

    pub fn on_start_render<S: CharacterStyle>(
        &self,
        cursor: &mut Cursor,
        props: TextBoxProperties<'_, S>,
    ) {
        let mut this = self.inner.borrow_mut();
        this.peeked_token = (0, None);

        this.plugin.on_start_render(cursor, props);
    }

    pub fn post_render<T, D>(
        &self,
        draw_target: &mut D,
        character_style: &T,
        text: &str,
        bounds: Rectangle,
    ) -> Result<(), D::Error>
    where
        T: TextRenderer<Color = C>,
        D: DrawTarget<Color = C>,
    {
        self.inner
            .borrow_mut()
            .lookahead
            .post_render(draw_target, character_style, text, bounds)
    }
}
