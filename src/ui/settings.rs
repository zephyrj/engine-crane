/*
 * Copyright (c):
 * 2022 zephyrj
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

use super::{Message, Tab};
use iced::{Alignment, button, Button, Column, Container, Element, Padding, Row, Text, text_input, TextInput};
use iced_aw::{TabLabel};
use crate::ui::{ApplicationData};


#[derive(Default)]
pub struct SettingsTab {
    ac_path_select_button: button::State,
    beamng_mod_path_select_button: button::State,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ThingSelected(String)
}

impl SettingsTab {
    pub(crate) fn new() -> Self {
        SettingsTab {
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: SettingsMessage, app_data: &ApplicationData) {
        match message { SettingsMessage::ThingSelected(_) => {} }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData) {
    }
}

impl Tab for SettingsTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Settings")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content<'a, 'b>(
        &'a mut self,
        app_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let base_path_str = match &app_data.get_ac_install_path() {
            None => format!("Not Set"),
            Some(path) => format!("{}", path.display())
        };
        let path_select_button =
            Button::new(&mut self.ac_path_select_button, Text::new("Browse"))
                .on_press(Message::AcPathSelectPressed);
        let ac_path_select_row = Column::new()
            .align_items(Alignment::Start)
            .push(Text::new("Assetto Corsa install path:").size(24))
            .push(Text::new(base_path_str))
            .push(path_select_button)
            .spacing(5)
            .padding(Padding::from([15, 3, 0, 3]));

        let mod_path_str = match &app_data.get_beam_ng_mod_path() {
            None => format!("Not Set"),
            Some(path) => format!("{}", path.display())
        };
        let mod_path_select_button =
            Button::new(&mut self.beamng_mod_path_select_button, Text::new("Browse"))
                .on_press(Message::BeamNGModPathSelectPressed);
        let mod_path_select_row = Column::new()
            .align_items(Alignment::Start)
            .push(Text::new("BeamNG mod path:").size(24))
            .push(Text::new(mod_path_str))
            .push(mod_path_select_button)
            .spacing(5)
            .padding(Padding::from([0, 3, 0, 3]));
        let container : Container<'_, Message> = Container::new(
            Column::new()
                .push(ac_path_select_row)
                .push(mod_path_select_row)
                .spacing(25)
        );
        container.into()
        //content.map(Message::Edit)
    }
}


