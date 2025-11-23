//! A simple wrap layout widget that flows elements left-to-right, wrapping to new lines

use iced::{
    Element, Length, Rectangle, Size,
    advanced::{
        layout::{Layout, Limits, Node},
        renderer,
        widget::{Tree, Widget},
    },
};

pub struct Wrap<'a, Message, Theme, Renderer> {
    elements: Vec<Element<'a, Message, Theme, Renderer>>,
    spacing: f32,
    line_spacing: f32,
}

impl<'a, Message, Theme, Renderer> Wrap<'a, Message, Theme, Renderer> {
    pub fn with_elements(elements: Vec<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            elements,
            spacing: 0.0,
            line_spacing: 0.0,
        }
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn line_spacing(mut self, line_spacing: f32) -> Self {
        self.line_spacing = line_spacing;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Wrap<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let max_width = limits.max().width;
        let mut children = Vec::new();
        let mut current_line_width = 0.0;
        let mut current_line_height = 0.0;
        let mut total_height = 0.0;
        let mut line_nodes = Vec::new();
        let mut x_offset = 0.0;
        let mut y_offset = 0.0;

        for (child, tree) in self.elements.iter_mut().zip(tree.children.iter_mut()) {
            let child_node = child.as_widget_mut().layout(tree, renderer, limits);
            let child_size = child_node.size();

            // Check if we need to wrap to a new line
            if current_line_width + child_size.width > max_width && !line_nodes.is_empty() {
                // Place all nodes in current line
                for node in line_nodes.drain(..) {
                    children.push(node);
                }

                // Move to next line
                y_offset += current_line_height + self.line_spacing;
                total_height = y_offset;
                x_offset = 0.0;
                current_line_width = 0.0;
                current_line_height = 0.0;
            }

            // Add child to current line
            let positioned_node = child_node.move_to(iced::Point::new(x_offset, y_offset));
            line_nodes.push(positioned_node);

            current_line_width += child_size.width + self.spacing;
            current_line_height = current_line_height.max(child_size.height);
            x_offset += child_size.width + self.spacing;
        }

        // Place remaining nodes in last line
        for node in line_nodes.drain(..) {
            children.push(node);
        }
        total_height += current_line_height;

        Node::with_children(Size::new(max_width, total_height), children)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .elements
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.elements);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.container(None, layout.bounds());

        self.elements
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .for_each(|((child, state), layout)| {
                child
                    .as_widget_mut()
                    .operate(state, layout, renderer, operation);
            });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        self.elements
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .elements
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let children = self
            .elements
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .filter_map(|((child, state), layout)| {
                child
                    .as_widget_mut()
                    .overlay(state, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!children.is_empty())
            .then(|| iced::advanced::overlay::Group::with_children(children).overlay())
    }
}

impl<'a, Message, Theme, Renderer> From<Wrap<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(wrap: Wrap<'a, Message, Theme, Renderer>) -> Self {
        Self::new(wrap)
    }
}
