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

use std::cmp::max;
use fraction::ToPrimitive;
use iced::{Alignment, Length};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Radio, Row, Text};
use iced_native::widget::{text, text_input};
use crate::assetto_corsa::car;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::setup::gears::{GearData, SingleGear};
use crate::assetto_corsa::traits::{CarDataUpdater, MandatoryDataSection};
use crate::ui::button::{create_add_button, create_delete_button, create_disabled_add_button, create_disabled_delete_button};
use crate::ui::edit::EditMessage;
use crate::ui::edit::gears::final_drive::FinalDriveUpdate::RemoveRatioPressed;
use crate::ui::edit::gears::ratio_set::{RatioSet};


#[derive(Debug, Clone)]
pub enum FinalDriveUpdate {
    AddRatioPressed(),
    RemoveRatioPressed(usize),
    UpdateFinalRatioName(String),
    UpdateFinalRatioVal(String),
    ConfirmNewFinalRatio(),
    DiscardNewFinalRatio(),
    DefaultSelected(usize)
}

#[derive(Debug, Default)]
pub struct FinalDrive {
    current_final_drive: f64,
    setup_data: Option<SingleGear>,
    new_setup_data: RatioSet,
    new_ratio_data: Option<(String, String)>
}

impl FinalDrive {
    pub fn from_gear_data(final_drive: f64, setup_data: Option<SingleGear>) -> FinalDrive {
        let current_final_drive = final_drive;
        let mut new_setup_data = RatioSet::new();
        match &setup_data {
            None => {
                new_setup_data.insert(String::from("DEFAULT"), current_final_drive);
            }
            Some(gear_data) => {
                gear_data.ratios_lut.to_vec().into_iter().for_each(|pair| {
                    let idx = new_setup_data.insert(pair.0, pair.1);
                    if pair.1 == current_final_drive {
                        let _ = new_setup_data.set_default(idx);
                    }
                });
            }
        };
        FinalDrive { current_final_drive, setup_data, new_setup_data, new_ratio_data: None }
    }

    pub fn handle_update(&mut self, update: FinalDriveUpdate) {
        match update {
            FinalDriveUpdate::AddRatioPressed() => {
                self.new_ratio_data = Some((String::new(), String::new()));
            }
            FinalDriveUpdate::RemoveRatioPressed(idx) => {
                self.new_setup_data.remove(idx);
            }
            FinalDriveUpdate::UpdateFinalRatioName(new_val) => {
                if let Some((name,_)) = &mut self.new_ratio_data {
                    *name = new_val;
                }
            }
            FinalDriveUpdate::UpdateFinalRatioVal(new_val) => {
                if let Some((_, ratio)) = &mut self.new_ratio_data {
                    if is_valid_ratio(&new_val) {
                        *ratio = new_val;
                    }
                }
            }
            FinalDriveUpdate::ConfirmNewFinalRatio() => {
                if let Some((name, ratio)) = &self.new_ratio_data {
                    match ratio.parse::<f64>() {
                        Ok(ratio_f) => {
                            let gear_name = match name.is_empty() {
                                true => ratio.clone(),
                                false => name.clone()
                            };
                            self.new_setup_data.insert(gear_name, ratio_f);
                        }
                        Err(_) => {}
                    }
                    self.new_ratio_data = None;
                }
            }
            FinalDriveUpdate::DiscardNewFinalRatio() => {
                self.new_ratio_data = None;
            }
            FinalDriveUpdate::DefaultSelected(idx) => {
                let _ = self.new_setup_data.set_default(idx);
            }
        }
    }

    pub fn create_final_drive_column(&self) -> Column<'static, EditMessage> {
        let mut col =
            Column::new()
                .align_items(Alignment::Center)
                .width(Length::Shrink)
                .spacing(5);
        col = col.push(text("Final Drive"));
        let name_width = (self.new_setup_data.max_name_len() * 10).to_u16().unwrap_or(u16::MAX);
        let default_idx = self.new_setup_data.default_idx();
        for ratio_entry in self.new_setup_data.entries() {
            let mut name_label = Text::new(ratio_entry.name.clone()).width(Length::Units(name_width));
            name_label = name_label.size(14);
            let ratio_string = ratio_entry.ratio.to_string();
            let mut ratio_input = Text::new(ratio_string).width(Length::Units(56));
            ratio_input = ratio_input.size(14);
            let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
            r = r.push(name_label);
            r = r.push(ratio_input);
            r = r.push(
                Radio::new(
                    ratio_entry.idx,
                    "",
                    default_idx,
                    move |idx| { EditMessage::FinalDriveUpdate(FinalDriveUpdate::DefaultSelected(idx)) }
                ).size(10)
            );
            let del_but = match self.new_setup_data.len() > 1 {
                true => create_delete_button(EditMessage::FinalDriveUpdate(RemoveRatioPressed(ratio_entry.idx))),
                false => create_disabled_delete_button()
            };
            r = r.push(del_but.height(Length::Units(15)).width(Length::Units(15)));
            col = col.push(r);
        }
        if let Some(_) = &self.new_ratio_data {
            col = col.push(self.add_gear_ratio_entry_row());
        } else {
            col = col.push(self.add_gear_ratio_button());
        }
        col
    }

    fn add_gear_ratio_button(&self) -> iced::widget::Button<'static, EditMessage> {
        iced::widget::button(
            text("Add Ratio").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::FinalDriveUpdate(FinalDriveUpdate::AddRatioPressed()))
    }

    fn add_gear_ratio_entry_row(&self) -> Row<'static, EditMessage>
    {
        let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
        let (name, ratio) = self.new_ratio_data.as_ref().unwrap();
        let name_max_width = (max(self.new_setup_data.max_name_len(), name.len()) * 10).to_u16().unwrap_or(u16::MAX);
        r = r.push(
            text_input(
                "",
                &name.clone(),
                move |new_val| {
                    EditMessage::FinalDriveUpdate(FinalDriveUpdate::UpdateFinalRatioName(new_val))
                })
                .width(Length::Units(name_max_width))
                .size(14)
        );
        r = r.push(
            text_input(
                "",
                &ratio.clone(),
                move |new_val| {
                    EditMessage::FinalDriveUpdate(FinalDriveUpdate::UpdateFinalRatioVal(new_val))
                })
                .width(Length::Units(56))
                .size(14));
        let mut confirm;
        if !ratio.is_empty() {
            confirm = create_add_button(EditMessage::FinalDriveUpdate(FinalDriveUpdate::ConfirmNewFinalRatio()));
        } else {
            confirm = create_disabled_add_button().height(Length::Units(20)).width(Length::Units(20));
        }
        r = r.push(confirm.height(Length::Units(20)).width(Length::Units(20)));
        r = r.push(
            create_delete_button(
                EditMessage::FinalDriveUpdate(FinalDriveUpdate::DiscardNewFinalRatio())
            )
            .height(Length::Units(20))
            .width(Length::Units(20))
        );
        r
    }

    pub(crate) fn apply_drivetrain_changes(&self, drivetrain: &mut Drivetrain) -> Result<(), String> {
        let ratio = match self.new_setup_data.default_ratio() {
            None => match self.new_setup_data.entries().first() {
                None => 3.0f64,
                Some(entry) => entry.ratio
            }
            Some(entry) => entry.ratio
        };
        let mut gearbox_data =
            car::data::drivetrain::Gearbox::load_from_parent(drivetrain)
                .map_err(|e| format!("{}", e.to_string()))?;
        gearbox_data.update_final_drive(ratio);
        gearbox_data.update_car_data(drivetrain).map_err(|e| format!("{}", e.to_string()))?;
        Ok(())
    }

    pub(crate) fn apply_setup_changes(&self, gear_data: &mut GearData) -> Result<(), String> {
        if self.new_setup_data.len() <= 1 {
            gear_data.clear_final_drive();
        } else {
            let final_ratios = self.new_setup_data.entries().iter().map(
                |entry| {
                    (entry.name.clone(), entry.ratio)
                }
            ).collect();
            gear_data.set_final_drive(Some(SingleGear::new_final_drive(final_ratios)));
        }
        Ok(())
    }
}

fn is_valid_ratio(val: &str) -> bool {
    if val.is_empty() || val.parse::<f64>().is_ok() {
        return true;
    }
    false
}
