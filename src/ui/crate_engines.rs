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

use std::path::PathBuf;
use iced::{Alignment, Background, Color, Element, Length, Renderer, Theme, theme};
use iced::alignment::{Horizontal, Vertical};
use iced::Length::Fill;
use iced::widget::{Button, Column, Container, Row, Text};
use iced_aw::style::colors::WHITE;
use iced_aw::TabLabel;
use iced_native::widget::{button, container, text, vertical_rule};
use tracing::{error, info};

use crate::data::{CrateEngine, CrateEngineMetadata, FromBeamNGModOptions};

use crate::ui::{ListPath, Message, Tab};
use crate::ui::data::ApplicationData;
use crate::ui::elements::{create_drop_down_list, create_text_with_units};
use crate::ui::elements::modal::Modal;



#[derive(Debug, Clone)]
pub enum CrateTabMessage {
    EngineSelected(String),
    BeamNGModSelected(ListPath),
    VerifyImport,
    ImportCancelled,
    ImportConfirmation,
    DeleteCrateEngineRequest,
    DeleteCancelled
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum ModalState {
    Hidden,
    VerifyImport,
    VerifyDelete,
    ShowActionResult
}

impl Default for ModalState {
    fn default() -> Self {
        ModalState::Hidden
    }
}

#[derive(Default)]
pub struct CrateEngineTab {
    selected_engine: Option<String>,
    pub(crate) selected_beam_ng_mod: Option<ListPath>,
    modal: ModalState,
    action_result_string: Option<String>
}

impl CrateEngineTab {
    pub(crate) fn new(_app_data: &ApplicationData) -> Self {
        CrateEngineTab {
            selected_engine: None,
            selected_beam_ng_mod: None,
            modal: ModalState::Hidden,
            action_result_string: None
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
            CrateTabMessage::VerifyImport => {
                self.modal = ModalState::VerifyImport
            }
            CrateTabMessage::ImportCancelled => {
                self.modal = ModalState::Hidden
            }
            CrateTabMessage::ImportConfirmation => {
                self.import_crate_engine(app_data);
                self.modal = ModalState::ShowActionResult
            }
            CrateTabMessage::DeleteCrateEngineRequest => {
                self.modal = ModalState::VerifyDelete
            }
            CrateTabMessage::DeleteCancelled => {
                self.modal = ModalState::Hidden
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData, update_event: &Message) {
        match update_event {
            Message::RefreshCrateEngines => {
                if self.modal == ModalState::ShowActionResult {
                    self.modal = ModalState::Hidden
                }
            }
            _ => {}
        }
        if let Some(path) = self.selected_beam_ng_mod.as_ref() {
            if !app_data.beam_ng_data.available_mods.contains(path) {
                self.selected_beam_ng_mod = None;
            }
        }
        if let Some(name) = self.selected_engine.as_ref() {
            if !app_data.crate_engine_data.available_engines.contains(name) {
                self.selected_engine = None;
            }
        }
    }

    pub fn notify_action_success(&mut self, action_event: &Message) {
        match action_event {
            Message::DeleteCrateEngine(eng_name) => {
                self.selected_engine = None;
                if self.modal == ModalState::VerifyDelete {
                    self.set_success_status(format!("Successfully deleted {}", eng_name));
                    self.modal = ModalState::ShowActionResult
                }
            }
            _ => {}
        }
    }

    pub fn notify_action_failure(&mut self, action_event: &Message, reason: &str) {
        match action_event {
            Message::DeleteCrateEngine(eng_name) => {
                if self.modal == ModalState::VerifyDelete {
                    self.set_error_status(format!("Failed to delete {}. {}", eng_name, reason));
                    self.modal = ModalState::ShowActionResult;
                }
            }
            _ => {}
        }
    }

    fn create_metadata_container(data: Option<&CrateEngineMetadata>) -> Column<'_, Message> {
        let mut metadata_container = Column::new();
        match data {
            None => {
                metadata_container = metadata_container.push(Text::new("No metadata found"));
            },
            Some(m) => {
                let mut title_col = Column::new();
                let mut value_col = Column::new();

                title_col = title_col.push(Text::new("Name:"));
                value_col = value_col.push(Text::new(format!("{}", m.name())));
                title_col = title_col.push(Text::new("Year:"));
                value_col = value_col.push(Text::new(format!("{}", m.build_year())));
                title_col = title_col.push(Text::new("Capacity:"));
                value_col = value_col.push(create_text_with_units(format!("{}", m.capacity()), "cc"));
                title_col = title_col.push(Text::new("Config:"));
                value_col = value_col.push(Text::new(format!("{} {} {}", m.block_config(), m.head_config(), m.valves())));
                title_col = title_col.push(Text::new("Aspiration:"));
                value_col = value_col.push(Text::new(format!("{}", m.aspiration())));
                title_col = title_col.push(Text::new("Fuel:"));
                value_col = value_col.push(Text::new(format!("{}", m.fuel())));
                title_col = title_col.push(Text::new("Limiter:"));
                value_col = value_col.push(create_text_with_units(format!("{}", m.max_rpm()), "rpm"));

                title_col = title_col.push(Text::new("Power:"));
                let mut power_container = Row::new().align_items(Alignment::End);
                power_container = power_container.push(create_text_with_units(format!("{}", m.peak_power()), "kW"));
                power_container = power_container.push(Text::new("@").size(18));
                power_container = power_container.push(create_text_with_units(format!("{}", m.peak_power_rpm()), "rpm"));
                value_col = value_col.push(power_container);

                title_col = title_col.push(Text::new("Torque:"));
                let mut torque_container = Row::new().align_items(Alignment::End);
                torque_container = torque_container.push(create_text_with_units(format!("{}", m.peak_torque()), "Nm"));
                torque_container = torque_container.push(Text::new("@").size(18));
                torque_container = torque_container.push(create_text_with_units(format!("{}", m.peak_torque_rpm()), "rpm"));
                value_col = value_col.push(torque_container);

                let table_holder =
                    Row::with_children(vec![title_col.into(), value_col.into()]).spacing(10).padding([0, 0, 10, 0]);
                metadata_container = metadata_container.push(table_holder);
                let source_string = m.get_source().source_name();
                metadata_container = metadata_container.push(Text::new(format!("Data Source: {}", source_string)));
                let version_string = m.data_version().to_string();
                metadata_container = metadata_container.push(Text::new(format!("Version: {}", version_string)));
                metadata_container = metadata_container.push(Text::new(format!("Automation Version: {}", m.automation_version())));
                metadata_container = metadata_container.push(
                    Button::new("Delete")
                        .style(theme::Button::Destructive)
                        .on_press(Message::CrateTab(CrateTabMessage::DeleteCrateEngineRequest))
                );
            }
        };
        metadata_container
    }

    fn get_modal_content(&self) -> Option<Element<'_, Message>> {
        match &self.modal {
            ModalState::Hidden => None,
            ModalState::VerifyImport => {
                let f: fn(&Theme) -> container::Appearance = |_theme: &Theme| {
                    container::Appearance{
                        text_color: None,
                        background: Some(Background::Color(WHITE)),
                        border_radius: 1.0,
                        border_width: 1.0,
                        border_color: Color::BLACK,
                    }
                };
                let default_val = ListPath::from_path(PathBuf::from("unknown"));
                let mod_path = self.selected_beam_ng_mod.as_ref().unwrap_or(&default_val);
                let modal_message = format!("This will import a crate engine from the BeamNG mod at:\n{}", mod_path.full_path.display());
                let confirm_content =
                    Column::new()
                        .align_items(Alignment::Center)
                        .width(Length::Units(50))
                        .push(Text::new("Ok").size(20).width(Fill));
                let confirm_button =
                    button(confirm_content)
                        .style(theme::Button::Positive)
                        .on_press(Message::CrateTab(CrateTabMessage::ImportConfirmation));
                let cancel_content =
                    Column::new()
                        .align_items(Alignment::Center)
                        .width(Length::Units(75))
                        .push(Text::new("Cancel").size(20).width(Fill));
                let cancel_button =
                    button(cancel_content)
                        .style(theme::Button::Destructive)
                        .on_press(Message::CrateTab(CrateTabMessage::ImportCancelled));
                let button_rom =
                    Row::with_children(vec![confirm_button.into(), cancel_button.into()])
                        .width(Length::Shrink)
                        .spacing(5);
                let modal_contents = container(
                    Column::new()
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .push(container(text(modal_message)))
                        .push(button_rom)
                ).style(theme::Container::Custom(
                    Box::new(f)
                )).padding(20);
                Some(modal_contents.into())
            }
            ModalState::ShowActionResult => {
                let f: fn(&Theme) -> container::Appearance = |_theme: &Theme| {
                    container::Appearance{
                        text_color: None,
                        background: Some(Background::Color(WHITE)),
                        border_radius: 1.0,
                        border_width: 1.0,
                        border_color: Color::BLACK,
                    }
                };
                let default_message = String::from("Unknown status");
                let modal_message = self.action_result_string.as_ref().unwrap_or(&default_message);
                let modal_contents = container(
                    Column::new()
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .push(container(text(modal_message)))
                        .push(
                            button("Ok")
                                .style(theme::Button::Positive)
                                .on_press(Message::RefreshCrateEngines)
                        )
                ).style(theme::Container::Custom(
                    Box::new(f)
                )).padding(20);
                Some(modal_contents.into())
            }
            ModalState::VerifyDelete => {
                let f: fn(&Theme) -> container::Appearance = |_theme: &Theme| {
                    container::Appearance{
                        text_color: None,
                        background: Some(Background::Color(WHITE)),
                        border_radius: 1.0,
                        border_width: 1.0,
                        border_color: Color::BLACK,
                    }
                };
                let default_val = String::from("unknown");
                let crate_name = self.selected_engine.as_ref().unwrap_or(&default_val);
                let modal_message = format!("This will delete crate engine {}", crate_name);
                let confirm_content =
                    Column::new()
                        .align_items(Alignment::Center)
                        .width(Length::Units(50))
                        .push(Text::new("Ok").size(20).width(Fill));
                let confirm_button =
                    button(confirm_content)
                        .style(theme::Button::Positive)
                        .on_press(Message::DeleteCrateEngine(crate_name.to_string()));
                let cancel_content =
                    Column::new()
                        .align_items(Alignment::Center)
                        .width(Length::Units(75))
                        .push(Text::new("Cancel").size(20).width(Fill));
                let cancel_button =
                    button(cancel_content)
                        .style(theme::Button::Destructive)
                        .on_press(Message::CrateTab(CrateTabMessage::DeleteCancelled));
                let button_rom =
                    Row::with_children(vec![confirm_button.into(), cancel_button.into()])
                        .width(Length::Shrink)
                        .spacing(5);
                let modal_contents = container(
                    Column::new()
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .push(container(text(modal_message)))
                        .push(button_rom)
                ).style(theme::Container::Custom(
                    Box::new(f)
                )).padding(20);
                Some(modal_contents.into())
            }
        }
    }

    fn import_crate_engine(&mut self, app_data: &ApplicationData) {
        if let Some(mod_path) = &self.selected_beam_ng_mod {
            if let Some(crate_engine_path) = app_data.get_crate_engine_path() {
                match CrateEngine::from_beamng_mod_zip(&mod_path.full_path, FromBeamNGModOptions::default()) {
                    Ok(crate_eng) => {
                        match crate_eng.write_to_path(crate_engine_path) {
                            Ok(written_path) => {
                                self.set_success_status(format!("Successfully created crate engine {}", written_path.display()));
                            }
                            Err(e) => {
                                self.set_error_status(format!("Failed to write crate engine. {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.set_error_status(format!("Failed to create crate engine from BeamNG mod {}. {}",mod_path.full_path.display(), e));
                    }
                }
            } else {
                self.set_error_status("Cannot import crate engine as path not set/accessible".to_string());
            }
        } else {
            self.set_error_status("Cannot import crate engine as no BeamNG mod selected".to_string());
        }
    }

    fn set_success_status(&mut self, error_str: String) {
        info!("{}",&error_str);
        self.action_result_string = Some(error_str);
    }

    fn set_error_status(&mut self, error_str: String) {
        error!("{}",&error_str);
        self.action_result_string = Some(error_str);
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
        let mut import_button = Button::new(Text::new("Import")).width(Length::Units(70));
        if self.selected_beam_ng_mod.is_some() {
            import_button = import_button.on_press(Message::CrateTab(CrateTabMessage::VerifyImport))
        }
        drop_down_list = drop_down_list.push(import_button);
        import_layout = import_layout.push(drop_down_list);

        let layout = Row::new()
            .push(crate_layout)
            .push(vertical_rule(4))
            .push(import_layout)
            .spacing(20);
        let content = Container::new(layout)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .padding(20);

        return match self.get_modal_content() {
            None => content.into(),
            Some(modal_content) => {
                Modal::new(content, modal_content).into()
            }
        }
    }
}
