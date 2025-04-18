/*
 * Copyright (c):
 * 2025 zephyrj
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
use iced::alignment::{Vertical};
use iced::widget::{Column, Container, Row, Text};
use iced_native::widget::{text_input, vertical_rule};
use zephyrj_ac_tools as assetto_corsa;
use assetto_corsa::car::model::GearingCalculator;
use assetto_corsa::car;
use assetto_corsa::car::data::Drivetrain;
use assetto_corsa::car::data::setup::gears::{GearConfig, GearData};
use assetto_corsa::traits::{CarDataUpdater, MandatoryDataSection};
use crate::ui::button::{create_add_button, create_delete_button, create_disabled_add_button, create_disabled_delete_button};
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::GearUpdate;
use crate::ui::edit::gears::final_drive::{FinalDrive, FinalDriveUpdate};
use crate::ui::edit::gears::{create_max_ratio_speed_element, GearConfigType, GearUpdateType};
use crate::ui::edit::gears::customizable::CustomizableGears;
use crate::ui::edit::gears::gear_sets::GearSets;
use crate::ui::edit::gears::GearUpdateType::Fixed;


#[derive(Debug, Clone, PartialEq)]
pub enum FixedGearUpdate {
    AddGear(),
    RemoveGear(),
    UpdateRatio(usize, String)
}

pub struct FixedGears {
    original_drivetrain_data: Vec<f64>,
    original_setup_data: Option<GearConfig>,
    updated_drivetrain_data: BTreeMap<usize, Option<String>>,
    final_drive_data: FinalDrive,
    gearing_calculator: Option<GearingCalculator>
}

impl From<GearSets> for FixedGears {
    fn from(mut value: GearSets) -> Self {
        let original_drivetrain_data = value.extract_original_drivetrain_data();
        let default_ratios = value.get_default_ratios();
        let updated_drivetrain_data =
            default_ratios.into_iter().enumerate().map(|(idx, default)| {
                match default {
                    None => (idx, None),
                    Some(ratio) => {
                        match original_drivetrain_data.get(idx) {
                            None => (idx, Some(ratio)),
                            Some(og_ratio) => {
                                if og_ratio.to_string() == ratio {
                                    (idx, None)
                                } else {
                                    (idx, Some(ratio))
                                }
                            }
                        }
                    }
                }
            }).collect();
        let mut config =
            FixedGears::new(original_drivetrain_data,
                            value.extract_original_setup_data(),
                            updated_drivetrain_data,
                            value.extract_final_drive_data());
        if let Some(gear_calc) = value.extract_gearing_calculator() {
            config.set_gearing_calculator(gear_calc);
        }
        config
    }
}

impl From<CustomizableGears> for FixedGears {
    fn from(mut value: CustomizableGears) -> Self {
        let original_drivetrain_data = value.extract_original_drivetrain_data();
        let default_ratios = value.get_default_gear_ratios();
        let updated_drivetrain_data =
            default_ratios.into_iter().enumerate().map(|(idx, default)| {
                match default {
                    None => (idx, None),
                    Some(ratio) => {
                        match original_drivetrain_data.get(idx) {
                            None => (idx, Some(ratio.to_string())),
                            Some(og_ratio) => {
                                if og_ratio.eq(&ratio) {
                                    (idx, None)
                                } else {
                                    (idx, Some(ratio.to_string()))
                                }
                            }
                        }
                    }
                }
            }).collect();
        let mut config =
            FixedGears::new(original_drivetrain_data,
                            value.extract_original_setup_data(),
                            updated_drivetrain_data,
                            value.extract_final_drive_data());
        if let Some(gear_calc) = value.extract_gearing_calculator() {
            config.set_gearing_calculator(gear_calc);
        }
        config
    }
}

impl FixedGears {
    pub fn new(original_drivetrain_data: Vec<f64>,
               original_setup_data: Option<GearConfig>,
               updated_drivetrain_data: BTreeMap<usize, Option<String>>,
               final_drive_data: FinalDrive,) -> FixedGears
    {
        FixedGears {
            original_drivetrain_data,
            original_setup_data,
            updated_drivetrain_data,
            final_drive_data,
            gearing_calculator: None
        }
    }

    pub(crate) fn from_gear_data(drivetrain_data: Vec<f64>,
                                 drivetrain_setup_data: Option<GearConfig>,
                                 final_drive_data: FinalDrive)
        -> FixedGears
    {
        let updated_drivetrain_data =
            drivetrain_data.iter().enumerate().map(|(idx, _)| (idx, None)).collect();
        FixedGears::new(drivetrain_data, drivetrain_setup_data, updated_drivetrain_data, final_drive_data)
    }

    pub(crate) fn set_gearing_calculator(&mut self, mut calculator: GearingCalculator) {
        calculator.set_gear_ratios(self.get_updated_gear_values());
        calculator.set_final_drive(self.final_drive_data.get_default_ratio_val());
        self.gearing_calculator = Some(calculator)
    }

    pub(crate) fn extract_gearing_calculator(&mut self) -> Option<GearingCalculator> {
        self.gearing_calculator.take()
    }

    fn create_gear_ratio_column(&self) -> Column<'static, EditMessage>
    {
        let mut row_vals = Vec::new();
        for (gear_idx, ratio) in self.updated_drivetrain_data.iter() {
            let current_val = match ratio {
                None => {
                    ""
                }
                Some(ratio) => {
                    ratio
                }
            };

            let placeholder = match self.original_drivetrain_data.get(*gear_idx) {
                None => { "".to_string() }
                Some(ratio) => { ratio.to_string() }
            };

            row_vals.push((placeholder, current_val.to_string()));
        }

        let mut gear_list =
            Column::new()
                .align_items(Alignment::Fill)
                .width(Length::Shrink)
                .spacing(5)
                .padding(Padding::from([0, 10, 12, 10]));
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
            if let Some(calc) = &self.gearing_calculator {
                let ratio_str;
                if new_ratio.is_empty() {
                    ratio_str = placeholder;
                } else {
                    ratio_str = new_ratio;
                }
                gear_row = gear_row.push(create_max_ratio_speed_element(ratio_str, calc));
            }
            gear_list = gear_list.push(gear_row);
        }

        let mut add_remove_row =
            Row::new()
                .width(Length::Shrink)
                .spacing(5)
                .padding(Padding::from([10, 0]))
                .align_items(Alignment::Center);

        let mut add_gear_button;
        if row_vals.len() < 10 {
            add_gear_button = create_add_button(GearUpdate(Fixed(FixedGearUpdate::AddGear())));
        } else {
            add_gear_button = create_disabled_add_button();
        }
        add_gear_button = add_gear_button.width(Length::Units(30)).height(Length::Units(30));
        add_remove_row = add_remove_row.push(add_gear_button);

        let mut delete_gear_button;
        if row_vals.len() > 1 {
            delete_gear_button = create_delete_button(GearUpdate(Fixed(FixedGearUpdate::RemoveGear())));
        } else {
            delete_gear_button = create_disabled_delete_button();
        }
        delete_gear_button = delete_gear_button.width(Length::Units(30)).height(Length::Units(30));
        add_remove_row = add_remove_row.push(delete_gear_button);
        gear_list.push(Container::new(add_remove_row).width(Length::Fill).center_x().center_y())
    }

    pub fn get_updated_gear_values(&self) -> Vec<f64> {
        let mut displayed_ratios = Vec::new();
        for (gear_idx, ratio) in self.updated_drivetrain_data.iter() {
            let current_val = match ratio {
                None => {
                    match self.original_drivetrain_data.get(*gear_idx) {
                        None => 0f64,
                        Some(ratio) => *ratio
                    }
                }
                Some(ratio) => {
                    ratio.parse::<f64>().unwrap_or(0f64)
                }
            };
            displayed_ratios.push(current_val);
        }
        displayed_ratios
    }

    pub(crate) fn extract_original_drivetrain_data(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.original_drivetrain_data)
    }

    pub(crate) fn extract_original_setup_data(&mut self) -> Option<GearConfig> {
        std::mem::take(&mut self.original_setup_data)
    }

    pub(crate) fn extract_final_drive_data(&mut self) -> FinalDrive {
        std::mem::take(&mut self.final_drive_data)
    }
    
    pub(crate) fn get_updated_ratios(&self) -> Vec<Option<f64>> {
        self.updated_drivetrain_data.values().map(|opt|{
            return if let Some(val) = opt {
                match val.parse::<f64>() {
                    Ok(ratio) => Some(ratio),
                    Err(_) => None
                }
            } else {
                None
            }
        }).collect()
    }

    pub(crate) fn get_config_type(&self) -> GearConfigType {
        GearConfigType::Fixed
    }

    // TODO return a Result so errors can be passed somewhere for viewing
    pub(crate) fn handle_gear_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            Fixed(update) => {
                match update {
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
                }
                if self.gearing_calculator.is_some() {
                    let updated_ratios = self.get_updated_gear_values();
                    self.gearing_calculator.as_mut().unwrap().set_gear_ratios(updated_ratios);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_final_drive_update(&mut self, update_type: FinalDriveUpdate) {
        self.final_drive_data.handle_update(update_type);
        if self.gearing_calculator.is_some() {
            let updated_final_drive = self.final_drive_data.get_default_ratio_val();
            self.gearing_calculator.as_mut().unwrap().set_final_drive(updated_final_drive);
        }
    }

    pub(crate) fn add_editable_gear_list<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut holder = Row::new().width(Length::Shrink).spacing(10).align_items(Alignment::Fill);
        holder = holder.push(self.create_gear_ratio_column());
        holder = holder.push(vertical_rule(5));
        holder = holder.push(
            self.final_drive_data.create_final_drive_column().padding(Padding::from([0, 10, 12, 10]))
        );
        layout.push(holder)
    }

    pub(crate) fn apply_drivetrain_changes(&self, drivetrain: &mut Drivetrain) -> Result<(), String> {
        let mut gearbox_data =
            car::data::drivetrain::Gearbox::load_from_parent(drivetrain)
                .map_err(|e| format!("{}", e.to_string()))?;
        let _ = gearbox_data.update_gears(self.get_updated_gear_values());
        gearbox_data.update_car_data(drivetrain).map_err(|e| e.to_string())?;
        self.final_drive_data.apply_drivetrain_changes(drivetrain)
    }

    pub(crate) fn apply_setup_changes(&self, gear_data: &mut GearData) -> Result<(), String> {
        gear_data.clear_gear_config();
        self.final_drive_data.apply_setup_changes(gear_data)?;
        Ok(())
    }
}

fn is_valid_ratio(val: &str) -> bool {
    if val.is_empty() || val.parse::<f64>().is_ok() {
        return true;
    }
    false
}
