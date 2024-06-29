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
use super::{Message, Tab};
use iced::{Alignment, Color, Element, Length, Padding, theme};
use iced::alignment::Vertical;
use iced::widget::{Button, Column, Container, svg, Text};
use iced_aw::{TabLabel};
use iced_native::widget::{Row, scrollable, Svg};
use iced_native::svg::Handle;
use iced_native::widget::scrollable::Properties;
use crate::settings::Setting as AppSettings;
use crate::settings::{AcInstallPath, AutomationUserdataPath, BeamNGModPath, CrateEnginePath, LegacyAutomationUserdataPath};
use crate::ui::{ApplicationData};
use crate::ui::data::PathState;
use crate::ui::image_data::{CIRCLE_CROSS, CIRCLE_TICK};

#[derive(Default)]
pub struct SettingsTab {

}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    #[allow(dead_code)]
    ThingSelected(String)
}

#[derive(Debug, Clone, Copy)]
pub enum Setting {
    AcPath,
    BeamNGModPath,
    CrateEnginePath,
    LegacyAutomationUserdataPath,
    AutomationUserdataPath
}

impl Setting {
    fn create_path_select(&self, app_data: &ApplicationData) -> Column<Message> {
        let value: String;
        let aux_text ;
        let is_valid: bool;
        let title: &'static str;
        match &self {
            Setting::AcPath => {
                (is_valid, value, aux_text) = get_path_data::<AcInstallPath>(app_data, true);
                title = AcInstallPath::friendly_name();
            }
            Setting::BeamNGModPath => {
                (is_valid, value, aux_text) = get_path_data::<BeamNGModPath>(app_data, false);
                title = BeamNGModPath::friendly_name();
            }
            Setting::CrateEnginePath => {
                (is_valid, value, aux_text) = get_path_data::<CrateEnginePath>(app_data, true);
                title = CrateEnginePath::friendly_name();
            }
            Setting::LegacyAutomationUserdataPath => {
                (is_valid, value, aux_text) = get_path_data::<LegacyAutomationUserdataPath>(app_data, false);
                title = LegacyAutomationUserdataPath::friendly_name();
            }
            Setting::AutomationUserdataPath => {
                (is_valid, value, aux_text) = get_path_data::<AutomationUserdataPath>(app_data, false);
                title = AutomationUserdataPath::friendly_name();
            }
        };
        create_path_select(*self, title, value, is_valid, aux_text)
    }
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
        let ac_path_selector =
            Setting::AcPath.create_path_select(app_data)
                .padding(Padding::from([15, 3, 0, 3]));

        let mod_path_selector =
            Setting::BeamNGModPath.create_path_select(app_data)
                .padding(Padding::from([0, 3, 0, 3]));

        let crate_path_selector = Setting::CrateEnginePath.create_path_select(app_data)
            .padding(Padding::from([0, 3, 0, 3]));

        let legacy_auto_path_selector = Setting::LegacyAutomationUserdataPath.create_path_select(app_data)
            .padding(Padding::from([0, 3, 0, 3]));

        let auto_path_selector = Setting::AutomationUserdataPath.create_path_select(app_data)
            .padding(Padding::from([0, 3, 0, 3]));

        let container : Element<'_, Message> = scrollable(Container::new(
            Column::new()
                .push(ac_path_selector)
                .push(mod_path_selector)
                .push(crate_path_selector)
                .push(legacy_auto_path_selector)
                .push(auto_path_selector)
                .spacing(25)
        ).width(Length::Fill)).vertical_scroll(Properties::default()).into();
        container
        //content.map(Message::Edit)
    }
}

fn success_green_colour() -> Color {
    Color::from_rgb8(75, 181, 67)
}

fn fail_red_colour() -> Color {
    Color::from_rgb8(237, 67, 55)
}

fn get_path_data<T: crate::settings::PathSetting>(app_data: &ApplicationData,
                                                  need_write_permission: bool) -> (bool, String, Option<String>)
{
    let mut valid = true;
    let mut invalid_text = None;
    let base_path_str = match &app_data.get_path::<T>() {
        None => {
            valid = false;
            "Not Set".to_string()
        }
        Some(path) => {
            let (read_state, write_state) = app_data.get_permission_data::<T>();
            if read_state == PathState::Ok {
                if need_write_permission && write_state != PathState::Ok {
                    valid = false;
                    invalid_text = Some("Don't have permission to write directory".to_owned())
                }
            } else {
                valid = false;
                invalid_text = Some("Don't have permission to read directory".to_owned())
            }
            format!("{}", path.display())
        }
    };
    (valid, base_path_str.clone(), invalid_text)
}

fn create_path_select(setting: Setting,
                      title: &str,
                      current_val: String,
                      is_valid: bool,
                      aux_text: Option<String>) -> Column<Message> {
    let select =
        Button::new( Text::new("Browse"))
            .on_press(Message::RequestPathSelect(setting));
    let copy =
        Button::new(Text::new("Copy"))
            .on_press(Message::CopySettingToClipboard(setting));
    let default=
        Button::new( Text::new("Revert to default")).style(theme::Button::Destructive)
            .on_press(Message::RevertSettingToDefault(setting));

    let (img, theme) = match is_valid {
        true => {
            (CIRCLE_TICK, theme::Svg::custom_fn(|_| {
                svg::Appearance{color: Some(success_green_colour())}
            }))
        }
        false => {
            (CIRCLE_CROSS, theme::Svg::custom_fn(|_| {
                svg::Appearance{color: Some(fail_red_colour())}
            }))
        }
    };
    let svg = Svg::new(Handle::from_memory(img))
                .style(theme)
                .height(Length::Units(15))
                .width(Length::Units(15));

    let val_row = Row::new()
        .align_items(Alignment::Center)
        .spacing(6)
        .push(Text::new(current_val))
        .push(svg);

    let mut button_row = Row::new()
        .align_items(Alignment::Center)
        .spacing(5)
        .push(select)
        .push(copy)
        .push(default);
    if let Some(t) = aux_text {
        button_row = button_row.push(Text::new(t).size(14).vertical_alignment(Vertical::Center));
    }

    Column::new()
        .align_items(Alignment::Start)
        .spacing(5)
        .push(Text::new(title).size(24))
        .push(val_row)
        .push(button_row)
}
