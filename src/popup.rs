use crate::{
    node::UINode,
    Control,
    UserInterface,
    widget::{
        Widget,
        WidgetBuilder,
    },
    message::{
        UiMessage,
        UiMessageData,
        PopupMessage,
        WidgetMessage,
        OsEvent,
        ButtonState,
    },
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
    border::BorderBuilder,
    NodeHandleMapping,
};
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Placement {
    LeftTop,
    RightTop,
    Center,
    LeftBottom,
    RightBottom,
    Cursor,
    Position(Vec2),
}

pub struct Popup<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    placement: Placement,
    stays_open: bool,
    is_open: bool,
    content: Handle<UINode<M, C>>,
    body: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Popup<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Popup<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Popup<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Popup(Self {
            widget: self.widget.raw_copy(),
            placement: self.placement,
            stays_open: false,
            is_open: false,
            content: self.content,
            body: self.body,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        if let Some(content) = node_map.get(&self.content) {
            self.content = *content;
        }
        self.body = *node_map.get(&self.body).unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        match &message.data {
            UiMessageData::Popup(msg) if message.destination == self.handle => {
                match msg {
                    PopupMessage::Open => {
                        self.is_open = true;
                        self.set_visibility(true);
                        if !self.stays_open {
                            if ui.top_picking_restriction() != self.handle {
                                ui.push_picking_restriction(self.handle);
                            }
                        }
                        self.send_message(UiMessage {
                            data: UiMessageData::Widget(WidgetMessage::TopMost),
                            destination: self.handle,
                            handled: false
                        });
                        match self.placement {
                            Placement::LeftTop => {
                                self.set_desired_local_position(Vec2::ZERO);
                            }
                            Placement::RightTop => {
                                let width = self.widget.actual_size().x;
                                let screen_width = ui.screen_size().x;
                                self.set_desired_local_position(
                                    Vec2::new(screen_width - width, 0.0));
                            }
                            Placement::Center => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                self.set_desired_local_position(
                                    (screen_size - size).scale(0.5));
                            }
                            Placement::LeftBottom => {
                                let height = self.widget.actual_size().y;
                                let screen_height = ui.screen_size().y;
                                self.set_desired_local_position(
                                    Vec2::new(0.0, screen_height - height));
                            }
                            Placement::RightBottom => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                self.set_desired_local_position(
                                    screen_size - size);
                            }
                            Placement::Cursor => {
                                self.set_desired_local_position(
                                    ui.cursor_position())
                            }
                            Placement::Position(position) => {
                                self
                                    .set_desired_local_position(
                                        position)
                            }
                        }
                    }
                    PopupMessage::Close => {
                        self.is_open = false;
                        self.set_visibility(false);
                        if !self.stays_open {
                            ui.pop_picking_restriction();
                        }
                        if ui.captured_node() == self.handle {
                            ui.release_mouse_capture();
                        }
                    }
                    PopupMessage::Content(content) => {
                        if self.content.is_some() {
                            ui.remove_node(self.content);
                        }
                        self.content = *content;
                        ui.link_nodes(self.content, self.body);
                    }
                    &PopupMessage::Placement(placement) => {
                        self.placement = placement;
                        self.invalidate_layout();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_os_event(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed && ui.top_picking_restriction() == self_handle && self.is_open {
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos.x, pos.y) && !self.stays_open {
                    self.close();
                }
            }
        }
    }
}

impl<M, C: 'static + Control<M, C>> Popup<M, C> {
    pub fn open(&mut self) {
        if !self.is_open {
            self.invalidate_layout();
            self.send_message(UiMessage {
                data: UiMessageData::Popup(PopupMessage::Open),
                destination: self.handle,
                handled: false
            });
        }
    }

    pub fn close(&mut self) {
        if self.is_open {
            self.invalidate_layout();
            self.send_message(UiMessage {
                data: UiMessageData::Popup(PopupMessage::Close),
                destination: self.handle,
                handled: false
            });
        }
    }

    pub fn set_placement(&mut self, placement: Placement) {
        if self.placement != placement {
            self.placement = placement;
            self.invalidate_layout();
            self.send_message(UiMessage {
                data: UiMessageData::Popup(PopupMessage::Placement(placement)),
                destination: self.handle,
                handled: false
            });
        }
    }
}

pub struct PopupBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    placement: Placement,
    stays_open: bool,
    content: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> PopupBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            placement: Placement::Cursor,
            stays_open: false,
            content: Default::default(),
        }
    }

    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn stays_open(mut self, value: bool) -> Self {
        self.stays_open = value;
        self
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> where Self: Sized {
        let body = BorderBuilder::new(WidgetBuilder::new()
            .with_child(self.content))
            .build(ui);

        let popup = Popup {
            widget: self.widget_builder
                .with_child(body)
                .with_visibility(false)
                .build(ui.sender()),
            placement: self.placement,
            stays_open: self.stays_open,
            is_open: false,
            content: self.content,
            body,
        };

        let handle = ui.add_node(UINode::Popup(popup));

        ui.flush_messages();

        handle
    }
}