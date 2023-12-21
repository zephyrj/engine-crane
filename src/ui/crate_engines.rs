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
use iced::alignment::Horizontal;
use iced::widget::{Button, Column, Container, PickList, Row, Text, TextInput};
use iced_aw::TabLabel;
use tracing::metadata;
use crate::data::CrateEngineMetadata;

use crate::ui::{ListPath, Message, Tab};
use crate::ui::data::ApplicationData;
use crate::ui::edit::EditMessage;
use crate::ui::elements::create_drop_down_list;

#[derive(Debug, Clone)]
pub enum CrateTabMessage {
    EngineSelected(String)
}

#[derive(Default)]
pub struct CrateEngineTab {
    crate_engine_list: Vec<String>,
    current_eng_selection: Option<String>,
}

impl CrateEngineTab {
    pub(crate) fn new(app_data: &ApplicationData) -> Self {
        let mut c = CrateEngineTab {
            crate_engine_list: Vec::new(),
            current_eng_selection: None
        };
        c.load_crate_engine_list(app_data);
        c
    }

    fn load_crate_engine_list(&mut self, application_data: &ApplicationData) {
        self.crate_engine_list.clear();
        self.current_eng_selection = None;
        self.crate_engine_list =
            application_data.crate_engine_data.available_engines.iter().map(|e| e.clone()).collect()
    }

    pub fn update(&mut self, message: CrateTabMessage, app_data: &ApplicationData) {
        match message {
            CrateTabMessage::EngineSelected(name) => {
                self.current_eng_selection = Some(name)
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData, update_event: &Message) {
        match update_event {
            Message::CrateEnginePathSelectPressed => {
                self.load_crate_engine_list(app_data)
            }
            _ => {}
        }
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
        let mut layout = Column::new()
            .align_items(Alignment::Fill)
            .spacing(20);
        let list = create_drop_down_list(
            "Crate Engines",
            &self.crate_engine_list,
            self.current_eng_selection.clone(),
            move |new_val| Message::CrateTab(CrateTabMessage::EngineSelected(new_val))
        );
        layout = layout.push(list);

        if let Some(name) = &self.current_eng_selection {
            layout = match app_data.crate_engine_data.get_metadata_for(name) {
                None => layout.push(Text::new("No metadata found")),
                Some(m) => {
                    layout = layout.push(Text::new(format!("Name: {}", m.name())));
                    let version_string = match m.data_version() {
                        Ok(v) => v.to_string(),
                        Err(_) => "Unknown".to_string()
                    };
                    layout.push(Text::new(format!("Version: {}", version_string)))
                }
            }
        }
        layout.into()
    }
}
