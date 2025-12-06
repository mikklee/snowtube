//! A custom scrollable widget with iOS-style elastic bounce effect at edges

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::animation::{Animation, Easing};
use iced::mouse::{self, Cursor};
use iced::time::Instant;
use iced::{Element, Event, Length, Padding, Rectangle, Size, Vector};

/// A scrollable container with elastic bounce at edges
pub struct BounceableScrollable<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
{
    id: Option<&'static str>,
    content: Element<'a, Message, Theme, Renderer>,
    width: Length,
    height: Length,
    padding: Padding,
}

impl<'a, Message, Theme, Renderer> BounceableScrollable<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    /// Creates a new [`BounceableScrollable`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            id: None,
            content: content.into(),
            width: Length::Fill,
            height: Length::Fill,
            padding: Padding::ZERO,
        }
    }

    /// Sets the unique identifier for this scrollable.
    /// Different IDs will have separate scroll states.
    pub fn id(mut self, id: &'static str) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the width of the [`BounceableScrollable`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`BounceableScrollable`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the padding of the [`BounceableScrollable`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

/// Internal state for the bounceable scrollable
#[derive(Debug, Clone)]
pub struct State {
    /// Widget ID to detect view changes
    id: Option<&'static str>,
    /// Current scroll offset (pixels from top)
    scroll_offset: f32,
    /// Bounce animation offset
    bounce_offset: f32,
    /// Animation for bounce back
    animation: Option<(Animation<bool>, f32, Instant)>,
    /// Content height (cached)
    content_height: f32,
    /// Viewport height (cached)
    viewport_height: f32,
    /// Scheduled time to start bounce-back animation
    bounce_back_at: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            id: None,
            scroll_offset: 0.0,
            bounce_offset: 0.0,
            animation: None,
            content_height: 0.0,
            viewport_height: 0.0,
            bounce_back_at: None,
        }
    }
}

impl State {
    fn reset(&mut self) {
        self.scroll_offset = 0.0;
        self.bounce_offset = 0.0;
        self.animation = None;
        self.bounce_back_at = None;
    }
}

impl State {
    fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    fn at_top(&self) -> bool {
        self.scroll_offset <= 0.0
    }

    fn at_bottom(&self) -> bool {
        self.scroll_offset >= self.max_scroll()
    }

    fn apply_scroll(&mut self, delta: f32) -> bool {
        let max = self.max_scroll();

        // If we have bounce offset, scrolling opposite direction reduces it
        if self.bounce_offset < 0.0 && delta > 0.0 {
            // Bounced at top, scrolling down - reduce bounce
            self.bounce_offset = (self.bounce_offset + delta * 10.0).min(0.0);
            return true;
        } else if self.bounce_offset > 0.0 && delta < 0.0 {
            // Bounced at bottom, scrolling up - reduce bounce
            self.bounce_offset = (self.bounce_offset + delta * 10.0).max(0.0);
            return true;
        }

        // If at edge and scrolling further, apply bounce with rubber band effect
        // Resistance increases quadratically as we stretch further
        const MAX_BOUNCE_TOP: f32 = 100.0;
        const MAX_BOUNCE_BOTTOM: f32 = 400.0;
        if self.at_top() && delta < 0.0 {
            // Overscroll at top - quadratic resistance
            let progress = self.bounce_offset.abs() / MAX_BOUNCE_TOP;
            let resistance = (1.0 - progress).powi(3).max(0.02) * 1.5;
            self.bounce_offset = (self.bounce_offset + delta * resistance).max(-MAX_BOUNCE_TOP);
            return true;
        } else if self.at_bottom() && delta > 0.0 {
            // Overscroll at bottom - quadratic resistance
            let progress = self.bounce_offset.abs() / MAX_BOUNCE_BOTTOM;
            let resistance = (1.0 - progress).powi(3).max(0.02) * 1.5;
            self.bounce_offset = (self.bounce_offset + delta * resistance).min(MAX_BOUNCE_BOTTOM);
            return true;
        }

        // Normal scroll
        let new_offset = (self.scroll_offset + delta).clamp(0.0, max);
        if (new_offset - self.scroll_offset).abs() > 0.01 {
            self.scroll_offset = new_offset;
            return true;
        }
        false
    }

    fn start_bounce_back(&mut self, now: Instant) {
        if self.bounce_offset.abs() > 0.5 {
            let start = self.bounce_offset;
            self.animation = Some((
                Animation::new(false)
                    .easing(Easing::EaseOutElastic)
                    .duration(std::time::Duration::from_millis(1200))
                    .go(true, now),
                start,
                now,
            ));
        }
    }

    fn tick(&mut self, now: Instant) -> bool {
        // Check if it's time to start bounce-back
        if let Some(bounce_at) = self.bounce_back_at
            && now >= bounce_at && self.animation.is_none() && self.bounce_offset.abs() > 0.5 {
                self.start_bounce_back(now);
                self.bounce_back_at = None;
            }

        if let Some((ref anim, start, _)) = self.animation {
            self.bounce_offset = anim.interpolate(start, 0.0, now);

            if self.bounce_offset.abs() < 0.5 {
                self.bounce_offset = 0.0;
                self.animation = None;
            }
            return true;
        }

        // Keep requesting redraws if we have a pending bounce-back
        self.bounce_back_at.is_some()
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for BounceableScrollable<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        // Reset scroll state if ID changed (view switched)
        let state = tree.state.downcast_mut::<State>();
        if state.id != self.id {
            state.reset();
            state.id = self.id;
        }
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // Get outer viewport size first (before padding is applied)
        let outer_limits = limits.width(self.width).height(self.height);
        let viewport_height = outer_limits.max().height;

        let node = layout::padded(limits, self.width, self.height, self.padding, |limits| {
            let content_limits = layout::Limits::with_compression(
                limits.min(),
                Size::new(limits.max().width, f32::INFINITY),
                Size::new(false, true), // compress height, not width
            );

            

            self.content.as_widget_mut().layout(
                &mut tree.children[0],
                renderer,
                &content_limits,
            )
        });

        // Update state with sizes after layout
        let content_height = node
            .children()
            .first()
            .map(|c| c.size().height)
            .unwrap_or(0.0);
        let state = tree.state.downcast_mut::<State>();
        state.content_height = content_height;
        state.viewport_height = viewport_height;

        node
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let cursor_over_scrollable = cursor.position_over(bounds);

        // Get scroll offset for cursor translation
        let translation = {
            let state = tree.state.downcast_ref::<State>();
            Vector::new(0.0, state.scroll_offset + state.bounce_offset)
        };

        // Handle animation ticks
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            if state.tick(*now) {
                shell.request_redraw();
            }
        }

        // Handle scroll wheel - capture this event, don't pass to children
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(bounds) {
                let state = tree.state.downcast_mut::<State>();
                let scroll_delta = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -y * 40.0,
                    mouse::ScrollDelta::Pixels { y, .. } => -y,
                };

                if state.apply_scroll(scroll_delta) {
                    shell.request_redraw();
                }

                // Cancel any ongoing animation
                state.animation = None;

                // Schedule bounce-back at a specific instant
                if state.bounce_offset.abs() > 0.5 {
                    let delay = if state.bounce_offset < 0.0 { 30 } else { 100 };
                    state.bounce_back_at =
                        Some(Instant::now() + std::time::Duration::from_millis(delay));
                    shell.request_redraw();
                } else {
                    state.bounce_back_at = None;
                }

                return; // Don't forward scroll events
            }

        // Forward other events to content with adjusted cursor position
        // (same approach as iced's scrollable)
        let content_layout = layout.children().next().unwrap();

        // Translate cursor position to account for scroll offset
        let translated_cursor = match cursor_over_scrollable {
            Some(cursor_position) => Cursor::Available(cursor_position + translation),
            _ => cursor,
        };

        // Adjusted viewport for children
        let content_viewport = Rectangle {
            x: bounds.x + translation.x,
            y: bounds.y + translation.y,
            ..bounds
        };

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            translated_cursor,
            renderer,
            clipboard,
            shell,
            &content_viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        // Get visible bounds (intersection with viewport)
        let Some(visible_bounds) = bounds.intersection(viewport) else {
            return;
        };

        // Calculate translation for scroll offset
        let translation_y = state.scroll_offset + state.bounce_offset;

        // Clip to visible bounds and apply translation
        renderer.with_layer(visible_bounds, |renderer| {
            renderer.with_translation(Vector::new(0.0, -translation_y), |renderer| {
                let content_layout = layout.children().next().unwrap();

                // Pass adjusted viewport to children
                let content_viewport = Rectangle {
                    x: visible_bounds.x,
                    y: visible_bounds.y + translation_y,
                    ..visible_bounds
                };

                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    content_layout,
                    cursor,
                    &content_viewport,
                );
            });
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        let scroll_translation = Vector::new(0.0, -(state.scroll_offset + state.bounce_offset));

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation + scroll_translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<BounceableScrollable<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(scrollable: BounceableScrollable<'a, Message, Theme, Renderer>) -> Self {
        Element::new(scrollable)
    }
}

/// Creates a new [`BounceableScrollable`] with the given content.
pub fn bounceable_scrollable<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> BounceableScrollable<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    BounceableScrollable::new(content)
}
