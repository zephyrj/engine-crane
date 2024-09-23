/*
 * Copyright (c):
 * 2024 zephyrj
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
mod fuel_econ;

use std::fmt::{Display, Formatter};
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
use crate::ui::edit::fuel_econ::{consumption_configuration_builder, FuelEfficiencyConfig};
use crate::ui::edit::gears::{gear_configuration_builder, convert_gear_configuration, FinalDriveUpdate, GearConfig, GearConfigType, GearUpdateType, GearConfiguration};
use crate::ui::elements::modal::Modal;
use crate::ui::image_data::ICE_CREAM_SVG;
use crate::ui::settings::Setting;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EditOption {
    Gears,
    FuelEcon
}

impl EditOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            EditOption::Gears => "Gears",
            EditOption::FuelEcon => "Fuel Consumption",
        }
    }

    pub fn all() -> Vec<EditOption> {
        vec![EditOption::Gears, EditOption::FuelEcon,]
    }
}

impl Display for EditOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct EditTab {
    status_message: String,
    edit_types: Vec<EditOption>,
    current_edit_type: EditOption,
    editable_car_paths: Vec<ListPath>,
    current_car_path: Option<PathBuf>,
    gear_configuration: Option<GearConfig>,
    fuel_eff_data: Option<FuelEfficiencyConfig>,
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
    EditTypeSelected(EditOption),
    CarSelected(ListPath),
    GearConfigSelected(GearConfigType),
    GearUpdate(GearUpdateType),
    FinalDriveUpdate(FinalDriveUpdate),
    FuelConsumptionUpdate(i32, String),
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
            edit_types: EditOption::all(),
            current_edit_type: EditOption::Gears,
            editable_car_paths: Vec::new(),
            current_car_path: None,
            gear_configuration: None,
            fuel_eff_data: None,
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
        self.fuel_eff_data = None;
        self.current_car_path = None;
        if self.show_all_cars {
            self.editable_car_paths = app_data.assetto_corsa_data.available_cars.clone();
        } else {
            let mut skip_count :usize = 0;
            for car_path in &app_data.assetto_corsa_data.available_cars {
                match Car::load_from_path(&car_path.full_path) {
                    Ok(mut car) => {
                        if car.is_ac_car_tuner_tune() {
                            self.editable_car_paths.push(car_path.clone());
                        } else {
                            match CarUiData::from_car(&mut car) {
                                Ok(ui_data) => {
                                    if ui_data.ui_info.has_tag(ENGINE_CRANE_CAR_TAG) {
                                        self.editable_car_paths.push(car_path.clone())
                                    }
                                }
                                Err(_) => skip_count += 1
                            } 
                        }
                    }
                    Err(_) => skip_count += 1
                }
            }
            info!("Skipped over {} cars", skip_count);
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

    fn setup_gear_data(&mut self) {
        if let Some(path_ref) = &self.current_car_path {
            match gear_configuration_builder(path_ref) {
                Ok(config) => { self.gear_configuration = Some(config) }
                Err(e) => {
                    error!(e)
                }
            }
        }
    }

    fn setup_fuel_econ_data(&mut self) {
        if let Some(path_ref) = &self.current_car_path {
            match consumption_configuration_builder(path_ref) {
                Ok(config) => { self.fuel_eff_data = Some(config) }
                Err(e) => {
                    error!(e)
                }
            }
        }
    }

    pub fn update(&mut self, message: EditMessage, app_data: &ApplicationData) {
        match message {
            EditMessage::CarSelected(path_ref) => {
                self.current_car_path = Some(path_ref.full_path.clone());
                match self.current_edit_type {
                    EditOption::Gears => self.setup_gear_data(),
                    EditOption::FuelEcon => self.setup_fuel_econ_data(),
                }
            }
            EditMessage::EditTypeSelected(ty) => {
                if self.current_edit_type != ty {
                    self.current_edit_type = ty;
                }
                if self.current_car_path.is_some() {
                    self.reload_selected_car();
                }

                match ty {
                    EditOption::Gears => {
                        if self.fuel_eff_data.is_some() {
                            self.fuel_eff_data = None;
                        }
                        self.setup_gear_data()
                    },
                    EditOption::FuelEcon => {
                        if self.gear_configuration.is_some() {
                            self.gear_configuration = None;
                        }
                        self.setup_fuel_econ_data()
                    }
                }
            },
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
                    convert_gear_configuration(
                        std::mem::take(&mut self.gear_configuration).unwrap(),
                        choice
                    ).unwrap_or_else(|(old_config, error)| {
                        self.status_message = error;
                        old_config
                    })
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
            EditMessage::FuelConsumptionUpdate(rpm, new_value) => {
                if let Some(config) = &mut self.fuel_eff_data {
                    config.update_efficiency_string(rpm, new_value);
                }
            }
            EditMessage::ApplyChanges() => {
                self.status_message = "Updating...".to_string();
                self.modal_state = ModalState::AfterUpdate;
                match self.current_edit_type {
                    EditOption::Gears => {
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
                    EditOption::FuelEcon => {
                        if let Some(config) = &mut self.fuel_eff_data {
                            if let Some(car_path) = &self.current_car_path {
                                match config.update_car(car_path) {
                                    Ok(_) => {
                                        self.update_successful = true;
                                        info!("Successfully updated fuel consumption data for {}", car_path.display())
                                    },
                                    Err(e) => {
                                        self.update_successful = false;
                                        self.status_message = format!("Failed to update fuel consumption data: {}", e);
                                        error!("Failed to update fuel consumption data for {}. {}", car_path.display(), e);
                                    }
                                }
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
                match self.current_edit_type {
                    EditOption::Gears => match gear_configuration_builder(&current_car_path) {
                        Ok(config) => { self.gear_configuration = Some(config) }
                        Err(e) => {
                            error!(e)
                        }
                    }
                    EditOption::FuelEcon => {
                        if self.gear_configuration.is_some() {
                            self.gear_configuration = None;
                        }
                    }
                }
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData, update_event: &Message) {
        match update_event {
            Message::RequestPathSelect(setting) => match setting {
                Setting::AcPath => self.load_car_list(app_data),
                _ => {}
            }
            Message::RevertSettingToDefault(setting) => match setting {
                Setting::AcPath => self.load_car_list(app_data),
                _ => {}
            }
            Message::EngineSwapRequested => self.load_car_list(app_data),
            _ => {}
        }
    }

    pub fn notify_action_success(&mut self, _action_event: &Message) {
    }

    pub fn notify_action_failure(&mut self, _action_event: &Message, _reason: &str) {
    }

    fn get_modal_content(&self) -> Option<Element<'_, EditMessage>> {
        match self.modal_state {
            ModalState::Hidden => None,
            ModalState::AfterUpdate => {
                let f: fn(&Theme) -> container::Appearance = |_theme: &Theme| {
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
                let f: fn(&Theme) -> container::Appearance = |_theme: &Theme| {
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

    fn content<'a, 'b>(&'a self, _app_data: &'b ApplicationData) -> Element<'_, Self::Message>
        where 'b: 'a
    {
        let edit_type_selector = Row::new().padding(0).spacing(8).align_items(Alignment::Center)
            .push(pick_list(
                &self.edit_types,
                Some(self.current_edit_type),
                EditMessage::EditTypeSelected,
            ));
        let edit_select_container = Column::new()
            .align_items(Alignment::Start)
            .padding(Padding::from([0,0,5,0]))
            .push(Text::new("Type"))
            .push(edit_type_selector);

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
        if let Some(_) = current_car {
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
            .push(edit_select_container)
            .push(car_select_container)
            .push(command_row);

        let mut layout = Column::new()
            .align_items(Alignment::Fill)
            //.padding(10)
            .spacing(20)
            .push(select_container);
            //.push(horizontal_rule(3));

        match self.current_edit_type {
            EditOption::Gears => {
                if let Some(gear_config) = &self.gear_configuration {
                    layout = self.add_gearbox_config_selector_row(layout, gear_config.get_config_type());
                    layout = gear_config.add_editable_gear_list(layout);
                }
            }
            EditOption::FuelEcon => {
                if let Some(fuel_econ_data) = &self.fuel_eff_data {
                    layout = fuel_econ_data.add_editable_list(layout);
                }
            }
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
