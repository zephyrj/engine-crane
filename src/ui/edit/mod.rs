/*
 * Copyright (c):
 * 2023 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

mod gears;

use super::{Message, Tab};
use std::path::{PathBuf};

use iced::{Alignment, Background, ContentFit, Element, Length, Padding, theme, Theme};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, pick_list, Row, Text, radio, horizontal_rule, Button, scrollable};
use iced_aw::{TabLabel};
use iced_aw::style::colors::WHITE;
use iced_native::{Color};
use iced_native::widget::scrollable::Properties;
use iced_native::widget::{button, checkbox, container, Svg, text};
use iced_native::svg::Handle;
use tracing::{error, info};
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::ENGINE_CRANE_CAR_TAG;
use crate::assetto_corsa::car::ui::CarUiData;

use crate::ui::{ApplicationData, ListPath};
use crate::ui::edit::gears::{gear_configuration_builder, convert_gear_configuration, FinalDriveUpdate, GearConfig, GearConfigType, GearUpdateType, GearConfiguration};
use crate::ui::edit::modal::Modal;
use crate::ui::image_data::ICE_CREAM_SVG;


pub struct EditTab {
    status_message: String,
    modal_message: String,
    editable_car_paths: Vec<ListPath>,
    current_car_path: Option<PathBuf>,
    gear_configuration: Option<GearConfig>,
    update_successful: bool,
    modal_state: ModalState,
    show_all_cars: bool
}

#[derive(Debug, Copy, Clone)]
enum ModalState {
    Hidden,
    AfterUpdate,
    AllCarsSelected
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    CarSelected(ListPath),
    GearConfigSelected(GearConfigType),
    GearUpdate(GearUpdateType),
    FinalDriveUpdate(FinalDriveUpdate),
    ApplyChanges(),
    ResetChanges(),
    ChangeConfirmation(),
    ShowAllCarsSelected(bool),
    ConfirmAllCars(),
    DeclineAllCars()
}

impl EditTab {
    pub(crate) fn new(app_data: &ApplicationData) -> Self {
        let mut e = EditTab {
            status_message: String::new(),
            modal_message: "Updating...".to_string(),
            editable_car_paths: Vec::new(),
            current_car_path: None,
            gear_configuration: None,
            update_successful: true,
            modal_state: ModalState::Hidden,
            show_all_cars: false
        };
        e.load_car_list(&app_data);
        e
    }

    fn load_car_list(&mut self, app_data: &ApplicationData) {
        self.editable_car_paths.clear();
        self.gear_configuration = None;
        self.current_car_path = None;
        if self.show_all_cars {
            self.editable_car_paths = app_data.assetto_corsa_data.available_cars.clone();
        } else {
            let mut skip_count :usize = 0;
            for car_path in &app_data.assetto_corsa_data.available_cars {
                match Car::load_from_path(&car_path.full_path) {
                    Ok(mut car) => {
                        match CarUiData::from_car(&mut car) {
                            Ok(ui_data) => {
                                if ui_data.ui_info.has_tag(ENGINE_CRANE_CAR_TAG) {
                                    self.editable_car_paths.push(car_path.clone())
                                }
                            }
                            Err(_) => skip_count += 1
                        }
                    }
                    Err(_) => skip_count += 1
                }
            }
        }
    }

    fn add_gearbox_config_selector_row<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>,
        selected_option: GearConfigType
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let gear_config_row = [GearConfigType::Fixed, GearConfigType::GearSets, GearConfigType::PerGearConfig]
            .iter().fold(
            Row::new().padding(Padding::from([0, 10])).spacing(20).align_items(Alignment::End),
            |row, config_choice| {
                row.push(radio(
                    format!("{config_choice}"),
                    *config_choice,
                    Some(selected_option),
                    EditMessage::GearConfigSelected).spacing(3).size(20).text_size(18))
            });
        layout.push(horizontal_rule(5)).push(gear_config_row)
    }

    pub fn update(&mut self, message: EditMessage, app_data: &ApplicationData) {
        match message {
            EditMessage::CarSelected(path_ref) => {
                self.current_car_path = Some(path_ref.full_path.clone());
                match gear_configuration_builder(&path_ref.full_path) {
                    Ok(config) => { self.gear_configuration = Some(config) }
                    Err(e) => {
                        error!(e)
                    }
                }
            }
            EditMessage::GearConfigSelected(choice) => {
                let current_config_type =
                    if let Some(config) = &self.gear_configuration {
                        Some(config.get_config_type())
                    } else {
                        None
                    };
                match current_config_type {
                    None => { return; }
                    Some(config_type) => if config_type == choice {
                        return;
                    }
                }

                self.gear_configuration = Some(
                    match convert_gear_configuration(
                        std::mem::take(&mut self.gear_configuration).unwrap(),
                        choice
                    ) {
                        Ok(new_config) => new_config,
                        Err((old_config, error)) => {
                            self.status_message = error;
                            old_config
                        }
                    }
                )
            }
            EditMessage::GearUpdate(update_type) => {
                if let Some(config) = &mut self.gear_configuration {
                    config.handle_gear_update(update_type);
                }
            }
            EditMessage::FinalDriveUpdate(update_type) => {
                if let Some(config) = &mut self.gear_configuration {
                    config.handle_final_drive_update(update_type);
                }
            }
            EditMessage::ApplyChanges() => {
                self.status_message = "Updating...".to_string();
                self.modal_state = ModalState::AfterUpdate;
                if let Some(config) = &mut self.gear_configuration {
                    if let Some(car_path) = &self.current_car_path {
                        match config.write_to_car(car_path) {
                            Ok(_) => {
                                self.update_successful = true;
                                info!("Successfully updated gear data for {}", car_path.display())
                            },
                            Err(e) => {
                                self.update_successful = false;
                                self.status_message = format!("Failed to update gear data: {}", e);
                                error!("Failed to update gear data for {}. {}", car_path.display(), e);
                            }
                        }
                    }
                }
            }
            EditMessage::ResetChanges() => {
                self.reload_selected_car();
            }
            EditMessage::ChangeConfirmation() => {
                self.status_message.clear();
                self.modal_state = ModalState::Hidden;
                if self.update_successful {
                    self.reload_selected_car();
                }
            }
            EditMessage::ShowAllCarsSelected(is_selected) => {
                match is_selected {
                    true => self.modal_state = ModalState::AllCarsSelected,
                    false => {
                        self.show_all_cars = false;
                        self.load_car_list(&app_data);
                    }
                }
            }
            EditMessage::ConfirmAllCars() => {
                self.show_all_cars = true;
                self.modal_state = ModalState::Hidden;
                self.load_car_list(&app_data);
            }
            EditMessage::DeclineAllCars() => {
                self.show_all_cars = false;
                self.modal_state = ModalState::Hidden;
            }
        }
    }

    fn reload_selected_car(&mut self) {
        match &self.current_car_path {
            None => error!("Reload requested when no car selected"),
            Some(current_car_path) => {
                match gear_configuration_builder(&current_car_path) {
                    Ok(config) => { self.gear_configuration = Some(config) }
                    Err(e) => {
                        error!(e)
                    }
                }
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData, update_event: &Message) {
        match update_event {
            Message::AcPathSelectPressed | Message::EngineSwapRequested => {
                self.load_car_list(app_data)
            }
            _ => {}
        }
    }

    fn get_modal_content(&self) -> Option<Element<'_, EditMessage>> {
        match self.modal_state {
            ModalState::Hidden => None,
            ModalState::AfterUpdate => {
                let f: fn(&Theme) -> container::Appearance = |theme: &Theme| {
                    container::Appearance{
                        text_color: None,
                        background: Some(Background::Color(WHITE)),
                        border_radius: 1.0,
                        border_width: 1.0,
                        border_color: Color::BLACK,
                    }
                };
                let modal_message = match self.update_successful {
                    true => "Update Successful!".to_string(),
                    false => format!("Update Failed. {}", &self.status_message)
                };
                let modal_contents = container(
                    Column::new()
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .push(container(text(modal_message)))
                        .push(button("Ok").style(theme::Button::Positive).on_press(EditMessage::ChangeConfirmation()))
                ).style(theme::Container::Custom(
                    Box::new(f)
                )).padding(20);
                Some(modal_contents.into())
            }
            ModalState::AllCarsSelected => {
                let f: fn(&Theme) -> container::Appearance = |theme: &Theme| {
                    container::Appearance{
                        text_color: None,
                        background: Some(Background::Color(WHITE)),
                        border_radius: 1.0,
                        border_width: 1.0,
                        border_color: Color::BLACK,
                    }
                };
                let modal_message = String::from("Warning: If you edit base AC cars they will not work online");
                let confirm = button(
                    Row::new()
                        .padding(0)
                        .spacing(3)
                        .align_items(Alignment::Center)
                        .push(
                            Svg::new(Handle::from_memory(ICE_CREAM_SVG))
                                //.style(theme::Svg::custom_fn(|_| { svg::Appearance{color: Some(Color::WHITE)} }))
                                .content_fit(ContentFit::Fill)
                                .height(Length::Units(15))
                                .width(Length::Units(15))
                        )
                        .push(text("Leave me alone, I know what I'm doing").size(14))
                ).style(theme::Button::Destructive).on_press(EditMessage::ConfirmAllCars());
                let decline =
                    button(text("I've changed my mind").size(20))
                        .style(theme::Button::Positive)
                        .on_press(EditMessage::DeclineAllCars());
                let modal_contents = container(
                    Column::new()
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .push(container(text(modal_message)))
                        .push(confirm)
                        .push(decline)
                ).style(theme::Container::Custom(
                    Box::new(f)
                )).padding(20);
                Some(modal_contents.into())
            }
        }
    }
}

impl Tab for EditTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Edit Car")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content<'a, 'b>(&'a self, app_data: &'b ApplicationData) -> Element<'_, Self::Message>
        where 'b: 'a
    {
        let current_car = match &self.current_car_path {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let mut command_row = Row::new().spacing(5);
        let mut apply_but = Button::new("Apply")
            .style(theme::Button::Positive);
        let mut reset_but =
            Button::new("Undo")
            .style(theme::Button::Destructive);
        if let Some(_) =current_car {
            apply_but = apply_but.on_press(EditMessage::ApplyChanges());
            reset_but = reset_but.on_press(EditMessage::ResetChanges());
        }
        command_row = command_row.push(apply_but).push(reset_but);

        let car_select_row = Row::new().padding(0).spacing(8).align_items(Alignment::Center)
            .push(pick_list(
                &self.editable_car_paths,
                current_car,
                EditMessage::CarSelected,
            ))
            .push(checkbox(
                "Show all cars",
                self.show_all_cars,
                |new_val| EditMessage::ShowAllCarsSelected(new_val)
            ).spacing(3));

        let car_select_container = Column::new()
            .align_items(Alignment::Start)
            .push(Text::new("Assetto Corsa car"))
            .push(car_select_row);

        let select_container = Column::new()
            .padding(Padding::from([0, 10]))
            .spacing(5)
            .push(car_select_container)
            .push(command_row);

        let mut layout = Column::new()
            .align_items(Alignment::Fill)
            //.padding(10)
            .spacing(30)
            .push(select_container);
            //.push(horizontal_rule(3));

        if let Some(gear_config) = &self.gear_configuration {
            layout = self.add_gearbox_config_selector_row(layout, gear_config.get_config_type());
            layout = gear_config.add_editable_gear_list(layout);
        }
        let content : Element<'_, EditMessage> =
            scrollable(
                Container::new(layout)
                    .align_x(Horizontal::Left)
                    .align_y(Vertical::Top)
                    .padding(20)
            ).horizontal_scroll(Properties::default()).into();

        return match self.get_modal_content() {
            None => content.map(Message::Edit),
            Some(modal_content) => {
                let r : Element<'_, EditMessage> =
                    Modal::new(content, modal_content).into();
                r.map(Message::Edit)
            }
        }
    }
}

mod modal {
    use iced_native::alignment::Alignment;
    use iced_native::widget::{self, Tree};
    use iced_native::{
        event, layout, mouse, overlay, renderer, Clipboard, Color, Element,
        Event, Layout, Length, Point, Rectangle, Shell, Size, Widget,
    };

    /// A widget that centers a modal element over some base element
    pub struct Modal<'a, Message, Renderer> {
        base: Element<'a, Message, Renderer>,
        modal: Element<'a, Message, Renderer>,
        on_blur: Option<Message>,
    }

    impl<'a, Message, Renderer> Modal<'a, Message, Renderer> {
        /// Returns a new [`Modal`]
        pub fn new(
            base: impl Into<Element<'a, Message, Renderer>>,
            modal: impl Into<Element<'a, Message, Renderer>>,
        ) -> Self {
            Self {
                base: base.into(),
                modal: modal.into(),
                on_blur: None,
            }
        }

        /// Sets the message that will be produces when the background
        /// of the [`Modal`] is pressed
        pub fn on_blur(self, on_blur: Message) -> Self {
            Self {
                on_blur: Some(on_blur),
                ..self
            }
        }
    }

    impl<'a, Message, Renderer> Widget<Message, Renderer>
    for Modal<'a, Message, Renderer>
        where
            Renderer: iced_native::Renderer,
            Message: Clone,
    {
        fn children(&self) -> Vec<Tree> {
            vec![Tree::new(&self.base), Tree::new(&self.modal)]
        }

        fn diff(&self, tree: &mut Tree) {
            tree.diff_children(&[&self.base, &self.modal]);
        }

        fn width(&self) -> Length {
            self.base.as_widget().width()
        }

        fn height(&self) -> Length {
            self.base.as_widget().height()
        }

        fn layout(
            &self,
            renderer: &Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            self.base.as_widget().layout(renderer, limits)
        }

        fn on_event(
            &mut self,
            state: &mut Tree,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            renderer: &Renderer,
            clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, Message>,
        ) -> event::Status {
            self.base.as_widget_mut().on_event(
                &mut state.children[0],
                event,
                layout,
                cursor_position,
                renderer,
                clipboard,
                shell,
            )
        }

        fn draw(
            &self,
            state: &Tree,
            renderer: &mut Renderer,
            theme: &<Renderer as iced_native::Renderer>::Theme,
            style: &renderer::Style,
            layout: Layout<'_>,
            cursor_position: Point,
            viewport: &Rectangle,
        ) {
            self.base.as_widget().draw(
                &state.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor_position,
                viewport,
            );
        }

        fn overlay<'b>(
            &'b mut self,
            state: &'b mut Tree,
            layout: Layout<'_>,
            _renderer: &Renderer,
        ) -> Option<overlay::Element<'b, Message, Renderer>> {
            Some(overlay::Element::new(
                layout.position(),
                Box::new(Overlay {
                    content: &mut self.modal,
                    tree: &mut state.children[1],
                    size: layout.bounds().size(),
                    on_blur: self.on_blur.clone(),
                }),
            ))
        }

        fn mouse_interaction(
            &self,
            state: &Tree,
            layout: Layout<'_>,
            cursor_position: Point,
            viewport: &Rectangle,
            renderer: &Renderer,
        ) -> mouse::Interaction {
            self.base.as_widget().mouse_interaction(
                &state.children[0],
                layout,
                cursor_position,
                viewport,
                renderer,
            )
        }

        fn operate(
            &self,
            state: &mut Tree,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn widget::Operation<Message>,
        ) {
            self.base.as_widget().operate(
                &mut state.children[0],
                layout,
                renderer,
                operation,
            );
        }
    }

    struct Overlay<'a, 'b, Message, Renderer> {
        content: &'b mut Element<'a, Message, Renderer>,
        tree: &'b mut Tree,
        size: Size,
        on_blur: Option<Message>,
    }

    impl<'a, 'b, Message, Renderer> overlay::Overlay<Message, Renderer>
    for Overlay<'a, 'b, Message, Renderer>
        where
            Renderer: iced_native::Renderer,
            Message: Clone,
    {
        fn layout(
            &self,
            renderer: &Renderer,
            _bounds: Size,
            position: Point,
        ) -> layout::Node {
            let limits = layout::Limits::new(Size::ZERO, self.size)
                .width(Length::Fill)
                .height(Length::Fill);

            let mut child = self.content.as_widget().layout(renderer, &limits);
            child.align(Alignment::Center, Alignment::Center, limits.max());

            let mut node = layout::Node::with_children(self.size, vec![child]);
            node.move_to(position);

            node
        }

        fn on_event(
            &mut self,
            event: Event,
            layout: Layout<'_>,
            cursor_position: Point,
            renderer: &Renderer,
            clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, Message>,
        ) -> event::Status {
            let content_bounds = layout.children().next().unwrap().bounds();

            if let Some(message) = self.on_blur.as_ref() {
                if let Event::Mouse(mouse::Event::ButtonPressed(
                                        mouse::Button::Left,
                                    )) = &event
                {
                    if !content_bounds.contains(cursor_position) {
                        shell.publish(message.clone());
                        return event::Status::Captured;
                    }
                }
            }

            self.content.as_widget_mut().on_event(
                self.tree,
                event,
                layout.children().next().unwrap(),
                cursor_position,
                renderer,
                clipboard,
                shell,
            )
        }

        fn draw(
            &self,
            renderer: &mut Renderer,
            theme: &Renderer::Theme,
            style: &renderer::Style,
            layout: Layout<'_>,
            cursor_position: Point,
        ) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border_radius: renderer::BorderRadius::from(0.0),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
                Color {
                    a: 0.80,
                    ..Color::BLACK
                },
            );

            self.content.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor_position,
                &layout.bounds(),
            );
        }

        fn operate(
            &mut self,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn widget::Operation<Message>,
        ) {
            self.content.as_widget().operate(
                self.tree,
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        }

        fn mouse_interaction(
            &self,
            layout: Layout<'_>,
            cursor_position: Point,
            viewport: &Rectangle,
            renderer: &Renderer,
        ) -> mouse::Interaction {
            self.content.as_widget().mouse_interaction(
                self.tree,
                layout.children().next().unwrap(),
                cursor_position,
                viewport,
                renderer,
            )
        }
    }

    impl<'a, Message, Renderer> From<Modal<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
        where
            Renderer: 'a + iced_native::Renderer,
            Message: 'a + Clone,
    {
        fn from(modal: Modal<'a, Message, Renderer>) -> Self {
            Element::new(modal)
        }
    }
}
