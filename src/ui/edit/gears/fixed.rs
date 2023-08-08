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
use iced::{Alignment, Length, Padding};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Row, Text};
use iced_native::widget::{text, text_input};
use crate::assetto_corsa::car::data::setup::gears::GearConfig;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::GearUpdate;
use crate::ui::edit::gears::final_drive::{FinalDrive, FinalDriveUpdate};
use crate::ui::edit::gears::{GearConfigChoice, GearConfiguration, GearUpdateType};
use crate::ui::edit::gears::GearUpdateType::Fixed;


#[derive(Debug, Clone, PartialEq)]
pub enum FixedGearUpdate {
    AddGear(),
    RemoveGear(),
    UpdateRatio(usize, String)
}

pub struct FixedGears {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Option<GearConfig>,
    updated_drivetrain_data: BTreeMap<usize, Option<String>>,
    final_drive_data: FinalDrive
}

impl FixedGears {
    pub(crate) fn from_gear_data(drivetrain_data: Vec<f64>, final_drive_data: FinalDrive) -> FixedGears {
        let updated_drivetrain_data = drivetrain_data.iter().enumerate().map(|(idx, _)| (idx, None)).collect();
        FixedGears {
            current_drivetrain_data: drivetrain_data,
            current_setup_data: None,
            updated_drivetrain_data,
            final_drive_data
        }
    }

    fn create_gear_ratio_column(row_vals: Vec<(String, String)>) -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
        let mut max_gear_idx = 0;
        for (gear_idx, (placeholder, new_ratio)) in row_vals.iter().enumerate() {
            let mut gear_row = Row::new()
                .width(Length::Shrink)
                .align_items(Alignment::Center)
                .spacing(5);
            let l = Text::new(format!("Gear {}:", gear_idx+1)).vertical_alignment(Vertical::Bottom);
            let t = text_input(
                placeholder,
                new_ratio,
                move |new_value| {
                    GearUpdate(Fixed(FixedGearUpdate::UpdateRatio(gear_idx, new_value)))
                }
            ).width(Length::Units(84));
            gear_row = gear_row.push(l).push(t);
            gear_list = gear_list.push(gear_row);
            max_gear_idx = gear_idx;
        }
        gear_list
    }
}

impl GearConfiguration for FixedGears {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::Fixed
    }

    // TODO return a Result so errors can be passed somewhere for viewing
    fn handle_gear_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            Fixed(update) => { match update {
                FixedGearUpdate::AddGear() => {
                    let gear_idx: usize = match self.updated_drivetrain_data.last_key_value() {
                        None => { 0 }
                        Some((max_gear_idx, _)) => { max_gear_idx+1 }
                    };
                    self.updated_drivetrain_data.insert(gear_idx, None);
                }
                FixedGearUpdate::RemoveGear() => {
                    self.updated_drivetrain_data.pop_last();
                }
                FixedGearUpdate::UpdateRatio(gear_idx, ratio) => {
                    if ratio.is_empty() {
                        self.updated_drivetrain_data.insert(gear_idx, None);
                    } else if is_valid_ratio(&ratio) {
                        self.updated_drivetrain_data.insert(gear_idx, Some(ratio));
                    }
                }
            }}
            _ => {}
        }
    }

    fn handle_final_drive_update(&mut self, update_type: FinalDriveUpdate) {
        self.final_drive_data.handle_update(update_type)
    }

    fn add_editable_gear_list<'a, 'b>(
        &'a self,
        mut layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut displayed_ratios = Vec::new();
        for (gear_idx, ratio) in self.updated_drivetrain_data.iter() {
            let current_val = match ratio {
                None => {
                    ""
                }
                Some(ratio) => {
                    ratio
                }
            };

            let placeholder = match self.current_drivetrain_data.get(*gear_idx) {
                None => { "".to_string() }
                Some(ratio) => { ratio.to_string() }
            };

            displayed_ratios.push((placeholder, current_val.to_string()));
        }
        let mut holder = Row::new().width(Length::Shrink).spacing(10).align_items(Alignment::Start);
        holder = holder.push(Self::create_gear_ratio_column(displayed_ratios));
        holder = holder.push( self.final_drive_data.create_final_drive_column());
        layout = layout.push(holder);
        let mut add_remove_row = Row::new().width(Length::Shrink).spacing(5).padding(Padding::from([10, 0])).align_items(Alignment::Center);
        let add_gear_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(Fixed(FixedGearUpdate::AddGear())));
        add_remove_row = add_remove_row.push(add_gear_button);
        let delete_gear_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(Fixed(FixedGearUpdate::RemoveGear())));
        add_remove_row = add_remove_row.push(delete_gear_button);
        layout.push(add_remove_row)
    }
}

fn is_valid_ratio(val: &str) -> bool {
    if val.is_empty() || val.parse::<f64>().is_ok() {
        return true;
    }
    false
}
