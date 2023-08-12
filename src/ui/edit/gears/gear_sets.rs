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
use std::fmt::{Display, Formatter};
use iced::{Alignment, Length, Padding};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Radio, Row, Text};
use iced_native::widget::{text, text_input};
use crate::assetto_corsa::car::data::setup::gears::GearSet;
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::GearUpdate;
use crate::ui::edit::gears::final_drive::FinalDrive;
use crate::ui::edit::gears::{FinalDriveUpdate, GearConfigType, GearConfiguration, GearUpdateType};
use crate::ui::edit::gears::fixed::FixedGears;
use crate::ui::edit::gears::GearsetUpdate::DefaultGearsetSelected;
use crate::ui::edit::gears::GearUpdateType::Gearset;


#[derive(Debug, Clone, PartialEq)]
pub enum GearsetUpdate {
    AddGear(),
    RemoveGear(),
    UpdateRatio(GearsetLabel, usize, String),
    DefaultGearsetSelected(GearsetLabel)
}

pub struct GearSets {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Vec<GearSet>,
    updated_drivetrain_data: BTreeMap<GearsetLabel, BTreeMap<usize, Option<String>>>,
    final_drive_data: FinalDrive,
    default_gearset: Option<GearsetLabel>
}

impl From<FixedGears> for GearSets {
    fn from(mut value: FixedGears) -> Self {
        let original_drivetrain_data = value.extract_original_drivetrain_data();
        let original_setup_data = value.extract_original_setup_data();
        let mut updated_gearsets =
            GearSetContainer::from_setup_data(&original_drivetrain_data, &original_setup_data);
        let mut new_gearset_label: Option<GearsetLabel> = None;

        for (ratio_idx, opt) in value.get_updated_ratios().iter().enumerate() {
            match opt {
                None => continue,
                Some(ratio) => {
                    if new_gearset_label.is_none() {
                        new_gearset_label = Some(updated_gearsets.add_gearset());
                    }
                    updated_gearsets.update_ratio(new_gearset_label.as_ref().unwrap(),
                                                  ratio_idx,
                                                  Some(ratio.to_string()));
                }
            }
        }
        if updated_gearsets.is_empty() {
            let label = updated_gearsets.add_gearset();
            for (idx, ratio) in original_drivetrain_data.iter().enumerate() {
                updated_gearsets.update_ratio(&label, idx, Some(ratio.to_string()));
            }
            updated_gearsets.set_default_gearset(&label);
        }

        GearSets {
            original_drivetrain_data,
            original_setup_data,
            updated_gearsets,
            final_drive_data: value.extract_final_drive_data(),
        }
    }
}

impl GearSets {
    pub(crate) fn from_gear_data(drivetrain_data: Vec<f64>,
                                 setup_data: Option<setup::gears::GearConfig>,
                                 final_drive_data: FinalDrive) -> GearSets
    {
        let updated_gearsets = GearSetContainer::from_setup_data(&drivetrain_data, &setup_data);
        GearSets {
            current_drivetrain_data: drivetrain_data,
            current_setup_data: setup_data,
            updated_drivetrain_data,
            final_drive_data,
            default_gearset
        }
    }
    fn create_gear_ratio_column(&self, gearset_label: GearsetLabel, row_vals: Vec<(String, String)>) -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10])).align_items(Alignment::Center);
        for (gear_idx, (placeholder, new_ratio)) in row_vals.iter().enumerate() {
            let mut gear_row = Row::new()
                .width(Length::Shrink)
                .align_items(Alignment::Center)
                .spacing(5);
            let l = Text::new(format!("Gear {}", gear_idx+1)).vertical_alignment(Vertical::Bottom);
            let label = gearset_label.clone();
            let t = text_input(
                placeholder,
                new_ratio,
                move |new_value| {
                    GearUpdate(Gearset(GearsetUpdate::UpdateRatio(label.clone(), gear_idx, new_value)))
                }
            ).width(Length::Units(84));
            gear_row = gear_row.push(l).push(t);
            gear_list = gear_list.push(gear_row);
        }
        let selected = match &self.default_gearset {
            None => None,
            Some(label) => Some(label.idx())
        };
        gear_list = gear_list.push(
            Radio::new(
                gearset_label.idx(),
                "Default",
                selected,
                move |_| { GearUpdate(Gearset(DefaultGearsetSelected(gearset_label.clone()))) }
            ).size(10).text_size(14).spacing(5)
        );
        gear_list
    }
}

impl GearConfiguration for GearSets {
    fn get_config_type(&self) -> GearConfigType {
        GearConfigType::GearSets
    }

    fn handle_gear_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            Gearset(update) => { match update {
                GearsetUpdate::AddGear() => {
                    for gear_set in self.updated_drivetrain_data.values_mut() {
                        let gear_idx: usize = match gear_set.last_key_value() {
                            None => { 0 }
                            Some((max_gear_idx, _)) => { max_gear_idx+1 }
                        };
                        gear_set.insert(gear_idx, None);
                    }
                }
                GearsetUpdate::RemoveGear() => {
                    for gear_set in self.updated_drivetrain_data.values_mut() {
                        gear_set.pop_last();
                    }
                }
                GearsetUpdate::UpdateRatio(set_idx, gear_idx, ratio) => {
                    if let Some(gear_set) = self.updated_drivetrain_data.get_mut(&set_idx) {
                        if ratio.is_empty() {
                            gear_set.insert(gear_idx, None);
                        } else if is_valid_ratio(&ratio) {
                            gear_set.insert(gear_idx, Some(ratio));
                        }
                    }
                }
                GearsetUpdate::DefaultGearsetSelected(label) => {
                    self.default_gearset = Some(label);
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
        let mut layout_row = Row::new().height(Length::Shrink).spacing(7).align_items(Alignment::Fill);
        let mut layout_col = Column::new().height(Length::Shrink);
        layout_col = layout_col.push(self.updated_gearsets.create_gearset_lists()).height(Length::Shrink);
        layout_row = layout_row.push(layout_col);
        layout_row = layout_row.push(vertical_rule(3));
        layout_row = layout_row.push(
            Column::new()
                .height(Length::Shrink)
                .push(self.final_drive_data.create_final_drive_column())
        );
        layout = layout.push(layout_row);
        let mut add_remove_row = Row::new().width(Length::Shrink).spacing(5);
        let add_ratio_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(Gearset(GearsetUpdate::AddGear())));
        add_remove_row = add_remove_row.push(add_ratio_button);
        let delete_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(Gearset(GearsetUpdate::RemoveGear())));
        add_remove_row = add_remove_row.push(delete_button);
        layout.push(add_remove_row)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GearsetLabel {
    idx: usize,
    name: String
}

impl GearsetLabel {
    pub fn new(idx: usize, name: String) -> GearsetLabel {
        GearsetLabel { idx, name }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }
}

impl Display for GearsetLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn is_valid_ratio(val: &str) -> bool {
    if val.is_empty() || val.parse::<f64>().is_ok() {
        return true;
    }
    false
}
