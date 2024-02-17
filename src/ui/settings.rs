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

use super::{Message, Tab};
use iced::{Alignment,Element, Padding};
use iced::widget::{Button, Column, Container, Text};
use iced_aw::{TabLabel};
use crate::ui::{ApplicationData};


#[derive(Default)]
pub struct SettingsTab {

}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    #[allow(dead_code)]
    ThingSelected(String)
}

impl SettingsTab {
    pub(crate) fn new() -> Self {
        SettingsTab {
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: SettingsMessage, _app_data: &ApplicationData) {
        match message { SettingsMessage::ThingSelected(_) => {} }
    }

    pub fn app_data_update(&mut self, _app_data: &ApplicationData, _update_event: &Message) {
    }

    pub fn notify_action_success(&mut self, _action_event: &Message) {
    }

    pub fn notify_action_failure(&mut self, _action_event: &Message, _reason: &str) {
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
        &'a self,
        app_data: &'b ApplicationData
    ) -> Element<'_, Self::Message>
    where 'b: 'a
    {
        let base_path_str = match &app_data.get_ac_install_path() {
            None => format!("Not Set"),
            Some(path) => format!("{}", path.display())
        };
        let path_select_button =
            Button::new(Text::new("Browse"))
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
            Button::new( Text::new("Browse"))
                .on_press(Message::BeamNGModPathSelectPressed);
        let mod_path_select_row = Column::new()
            .align_items(Alignment::Start)
            .push(Text::new("BeamNG mod path:").size(24))
            .push(Text::new(mod_path_str))
            .push(mod_path_select_button)
            .padding(Padding::from([0, 3, 0, 3]));

        let crate_path_str = match &app_data.get_crate_engine_path() {
            None => "Not Set".to_string(),
            Some(path) => format!("{}", path.display())
        };
        let crate_path_selector =
            create_path_select(Message::CrateEnginePathSelectPressed,
                               "Crate engine path",
                               crate_path_str)
                .padding(Padding::from([0, 3, 0, 3]));

        let legacy_auto_path_str = match &app_data.get_legacy_automation_userdata_path() {
            None => "Not Set".to_string(),
            Some(path) => format!("{}", path.display())
        };
        let legacy_auto_path_selector =
            create_path_select(Message::LegacyAutomationPathSelectPressed,
                               "Legacy automation data path",
                               legacy_auto_path_str)
                .padding(Padding::from([0, 3, 0, 3]));

        let auto_path_str = match &app_data.get_automation_userdata_path() {
            None => "Not Set".to_string(),
            Some(path) => format!("{}", path.display())
        };
        let auto_path_selector =
            create_path_select(Message::AutomationPathSelectPressed,
                               "Automation data path",
                               auto_path_str)
                .padding(Padding::from([0, 3, 0, 3]));


        let container : Container<'_, Message> = Container::new(
            Column::new()
                .push(ac_path_select_row)
                .push(mod_path_select_row)
                .push(crate_path_selector)
                .push(legacy_auto_path_selector)
                .push(auto_path_selector)
                .spacing(25)
        );
        container.into()
        //content.map(Message::Edit)
    }
}

fn create_path_select(on_select: Message, title: &str, current_val: String) -> Column<Message> {
    let select =
        Button::new( Text::new("Browse"))
            .on_press(on_select);
    Column::new()
        .align_items(Alignment::Start)
        .push(Text::new(title).size(24))
        .push(Text::new(current_val))
        .push(select)
        .spacing(5)
}




