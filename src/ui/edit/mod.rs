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

use iced::{Alignment, Element, Length, Padding};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, pick_list, Row, Text, radio, horizontal_rule};
use iced_aw::{TabLabel};
use tracing::{error};

use crate::ui::{ApplicationData, ListPath};
use crate::ui::edit::gears::{gear_configuration_builder, convert_gear_configuration, FinalDriveUpdate, GearConfig, GearConfigType, GearUpdateType, GearConfiguration};


pub struct EditTab {
    status_message: String,
    current_car_path: Option<PathBuf>,
    gear_configuration: Option<GearConfig>
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    CarSelected(ListPath),
    GearConfigSelected(GearConfigType),
    GearUpdate(GearUpdateType),
    FinalDriveUpdate(FinalDriveUpdate)
}

impl EditTab {
    pub(crate) fn new() -> Self {
        EditTab {
            status_message: String::new(),
            current_car_path: None,
            gear_configuration: None
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
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData) {
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

    fn content<'a, 'b>(
        &'a self,
        app_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let current_car = match &self.current_car_path {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let car_select_container = Column::new()
            .align_items(Alignment::Start)
            //.padding(10)
            .push(Text::new("Assetto Corsa car"))
            .push(pick_list(
                &app_data.assetto_corsa_data.available_cars,
                current_car,
                EditMessage::CarSelected,
            ));
        let select_container = Row::new()
            //.align_items(Align::)
            .padding(Padding::from([0, 10]))
            .spacing(20)
            .push(car_select_container);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Alignment::Start)
            //.padding(10)
            .spacing(30)
            .push(select_container);
            //.push(horizontal_rule(3));

        if let Some(gear_config) = &self.gear_configuration {
            layout = self.add_gearbox_config_selector_row(layout, gear_config.get_config_type());
            layout = gear_config.add_editable_gear_list(layout);
        }
        let content : Element<'_, EditMessage> = Container::new(layout)
            //.width(Length::Fill)
            .align_x(Horizontal::Left)
            //.height(Length::Fill)
            .align_y(Vertical::Top)
            .padding(20)
            .into();
        content.map(Message::Edit)
    }
}
