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

use std::collections::BTreeMap;
use iced::{Alignment, Element, Length, Padding, Renderer};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, Column, Container, PickList, Row, Text, TextInput};
use iced_aw::TabLabel;
use iced_native::widget::vertical_rule;
use tracing::metadata;
use crate::data::CrateEngineMetadata;

use crate::ui::{ListPath, Message, Tab};
use crate::ui::data::ApplicationData;
use crate::ui::edit::EditMessage;
use crate::ui::elements::create_drop_down_list;

#[derive(Debug, Clone)]
pub enum CrateTabMessage {
    EngineSelected(String),
    BeamNGModSelected(ListPath)
}

#[derive(Default)]
pub struct CrateEngineTab {
    selected_engine: Option<String>,
    pub(crate) selected_beam_ng_mod: Option<ListPath>
}

impl CrateEngineTab {
    pub(crate) fn new(app_data: &ApplicationData) -> Self {
        CrateEngineTab {
            selected_engine: None,
            selected_beam_ng_mod: None
        }
    }

    pub fn update(&mut self, message: CrateTabMessage, app_data: &ApplicationData) {
        match message {
            CrateTabMessage::EngineSelected(name) => {
                self.selected_engine = Some(name)
            },
            CrateTabMessage::BeamNGModSelected(name) => {
                self.selected_beam_ng_mod = Some(name)
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData, update_event: &Message) {
        // match update_event {
        //     Message::CrateEnginePathSelectPressed => {
        //         self.load_crate_engine_list(app_data)
        //     },
        //     Message::ImportCrateEngineRequested => {
        //
        //     }
        //     _ => {}
        // }
    }

    fn create_metadata_container(data: Option<&CrateEngineMetadata>) -> Column<'_, Message> {
        let mut metadata_container = Column::new();
        match data {
            None => {
                metadata_container = metadata_container.push(Text::new("No metadata found"));
            },
            Some(m) => {
                metadata_container = metadata_container.push(Text::new(format!("Name: {}", m.name())));
                let version_string = match m.data_version() {
                    Ok(v) => v.to_string(),
                    Err(_) => "Unknown".to_string()
                };
                metadata_container = metadata_container.push(Text::new(format!("Version: {}", version_string)));
            }
        };
        metadata_container
    }
}

impl Tab for CrateEngineTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("My Engines")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content<'a, 'b>(&'a self, app_data: &'b ApplicationData ) -> Element<'_, Self::Message, Renderer>
        where 'b: 'a
    {
        let mut crate_layout = Column::new()
            .width(Length::FillPortion(2))
            .spacing(20);
        let list = create_drop_down_list(
            "Crate Engines",
            &app_data.crate_engine_data.available_engines,
            self.selected_engine.clone(),
            move |new_val| Message::CrateTab(CrateTabMessage::EngineSelected(new_val))
        );
        crate_layout = crate_layout.push(list);
        if let Some(name) = &self.selected_engine {
            crate_layout = crate_layout.push(Self::create_metadata_container(app_data.crate_engine_data.get_metadata_for(name)))
        }

        let mut import_layout = Column::new().width(Length::FillPortion(1)).align_items(Alignment::Center);
        let mut drop_down_list = create_drop_down_list(
            "Import from BeamNG mod",
            &app_data.beam_ng_data.available_mods,
            self.selected_beam_ng_mod.clone(),
            move |new_val| Message::CrateTab(CrateTabMessage::BeamNGModSelected(new_val))
        );
        let mut import_button = Button::new(Text::new("Import")).width(Length::Units(60));
        if self.selected_beam_ng_mod.is_some() {
            import_button = import_button.on_press(Message::ImportCrateEngineRequested)
        }
        drop_down_list = drop_down_list.push(import_button);
        import_layout = import_layout.push(drop_down_list);

        let layout = Row::new()
            .push(crate_layout)
            .push(vertical_rule(4))
            .push(import_layout)
            .spacing(20);
        Container::new(layout)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .padding(20).into()
    }
}
