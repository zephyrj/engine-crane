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
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use fraction::ToPrimitive;
use iced::{Alignment, Length, Padding};
use iced::alignment::{Horizontal, Vertical};
use iced::theme::Button;
use iced::widget::{Column, Container, Radio, Row, Text};
use iced_native::widget::{scrollable, text, text_input};
use iced_native::widget::scrollable::Properties;
use crate::assetto_corsa::car::data::setup::gears::{GearConfig, SingleGear};
use crate::ui::button::{create_add_button, create_delete_button, create_disabled_add_button};
use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::GearUpdate;
use crate::ui::edit::gears::{FinalDriveUpdate, GearConfigType, GearConfiguration, GearUpdateType};
use crate::ui::edit::gears::final_drive::FinalDrive;
use crate::ui::edit::gears::fixed::FixedGears;
use crate::ui::edit::gears::GearUpdateType::CustomizedGear;
use crate::ui::edit::gears::ratio_set::{RatioEntry, RatioSet};


#[derive(Debug, Clone, PartialEq)]
pub enum CustomizedGearUpdate {
    AddGear(),
    RemoveGear(),
    AddRatio(GearLabel),
    DefaultRatioSelected(GearLabel, usize),
    RemoveRatio(GearLabel, usize),
    UpdateRatioName(String),
    UpdateRatioValue(String),
    ConfirmNewRatio(),
    DiscardNewRatio(),
}

pub struct CustomizableGears {
    original_drivetrain_data: Vec<f64>,
    original_setup_data: Option<GearConfig>,
    new_setup_data: BTreeMap<GearLabel, RatioSet>,
    new_ratio_data: Option<(GearLabel, String, String)>,
    final_drive_data: FinalDrive
}

impl From<FixedGears> for CustomizableGears {
    fn from(mut fixed_gears: FixedGears) -> Self {
        let original_drivetrain_data = fixed_gears.extract_original_drivetrain_data();
        let original_setup_data = fixed_gears.extract_original_setup_data();
        let mut new_setup_data =
            Self::load_from_setup_data(&original_drivetrain_data, &original_setup_data);
        if new_setup_data.is_empty() {
            for (idx, ratio) in original_drivetrain_data.iter().enumerate() {
                let mut ratio_set = RatioSet::new();
                let ratio_idx = ratio_set.insert(ratio.to_string(), *ratio);
                let _ = ratio_set.set_default(ratio_idx);
                new_setup_data.insert((idx+1).into(), ratio_set);
            }
        }
        CustomizableGears {
            original_drivetrain_data,
            original_setup_data,
            new_setup_data,
            new_ratio_data: None,
            final_drive_data: fixed_gears.extract_final_drive_data()
        }
    }
}

impl CustomizableGears {
    pub(crate) fn from_gear_data(drivetrain_data: Vec<f64>, drivetrain_setup_data: Option<GearConfig>, final_drive_data: FinalDrive) -> CustomizableGears {
        let new_setup_data = Self::load_from_setup_data(&drivetrain_data, &drivetrain_setup_data);
        CustomizableGears {
            original_drivetrain_data: drivetrain_data,
            original_setup_data: drivetrain_setup_data,
            new_setup_data,
            new_ratio_data: None,
            final_drive_data
        }
    }

    fn load_from_setup_data(drivetrain_data: &Vec<f64>, drivetrain_setup_data: &Option<GearConfig>) -> BTreeMap<GearLabel, RatioSet> {
        let current_setup_data =  match &drivetrain_setup_data {
            None => Vec::new(),
            Some(gear_config) => { match gear_config {
                GearConfig::GearSets(_) => Vec::new(),
                GearConfig::PerGear(gear_vec) => gear_vec.clone()
            }}
        };
        let mut new_setup_data= BTreeMap::new();
        for (idx, gear) in current_setup_data.into_iter().enumerate() {
            let gear_vec = gear.ratios_lut.to_vec();
            let mut ratio_set = RatioSet::new();
            let default_opt = drivetrain_data.get(idx);
            gear_vec.into_iter().for_each(|pair| {
                let ratio_idx = ratio_set.insert(pair.0, pair.1);
                if let Some(default_ratio) = default_opt {
                    if pair.1 == *default_ratio {
                        let _ = ratio_set.set_default(ratio_idx);
                    }
                }
            });
            new_setup_data.insert(gear.get_index().unwrap_or(idx).into(),
                                  ratio_set);
        }
        new_setup_data
    }

    fn create_gear_ratio_column(gear_idx: &GearLabel, ratio_set: &RatioSet ) -> Column<'static, EditMessage>
    {
        let mut col = Column::new().align_items(Alignment::Center).width(Length::Shrink).spacing(5).padding(Padding::from([0, 10, 12, 10]));
        col = col.push(text(gear_idx));
        let default_idx = ratio_set.default_idx();
        let name_width = (ratio_set.max_name_len() * 10).to_u16().unwrap_or(u16::MAX);
        for ratio_entry in ratio_set.entries() {
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
                    move |idx| { GearUpdate(CustomizedGear(CustomizedGearUpdate::DefaultRatioSelected(gear_idx.clone(), idx))) }
                ).size(10)
            );
            r = r.push(
                create_delete_button(
                    GearUpdate(CustomizedGear(CustomizedGearUpdate::RemoveRatio(gear_idx.clone(), ratio_entry.idx)))
                )
                    .height(Length::Units(15))
                    .width(Length::Units(15))
            );
            col = col.push(r);
        }
        col
    }

    fn add_gear_ratio_button(label: GearLabel) -> iced::widget::Button<'static, EditMessage> {
        iced::widget::button(
            text("Add Ratio").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(CustomizedGear(CustomizedGearUpdate::AddRatio(label))))
    }

    fn add_gear_ratio_entry_row(new_ratio_data: (GearLabel, String, String), name_max_width: u16) -> Row<'static, EditMessage>
    {
        let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
        let (label, name, ratio) = new_ratio_data;
        r = r.push(text_input(
            "",
            &name,
            move |new_val| {
                GearUpdate(CustomizedGear(CustomizedGearUpdate::UpdateRatioName(new_val)))})
            .width(Length::Units(name_max_width))
            .size(14)
        );
        r = r.push(text_input(
            "",
            &ratio,
            move |new_val| {
                GearUpdate(CustomizedGear(CustomizedGearUpdate::UpdateRatioValue(new_val)))})
            .width(Length::Units(56))
            .size(14)
        );
        let mut confirm;
        if !ratio.is_empty() {
            confirm = create_add_button(GearUpdate(CustomizedGear(CustomizedGearUpdate::ConfirmNewRatio())));
        } else {
            confirm = create_disabled_add_button().height(Length::Units(20)).width(Length::Units(20));
        }
        r = r.push(confirm.height(Length::Units(20)).width(Length::Units(20)));
        r = r.push(
            create_delete_button(GearUpdate(CustomizedGear(CustomizedGearUpdate::DiscardNewRatio())))
                .height(Length::Units(20))
                .width(Length::Units(20))
        );
        r
    }

    pub(crate) fn get_default_gear_ratios(&self) -> Vec<Option<f64>> {
        self.new_setup_data.values().map(|ratio_set|{
            match ratio_set.default_ratio() {
                None => None,
                Some(ratio_entry) => Some(ratio_entry.ratio)
            }
        }).collect()
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
}

impl GearConfiguration for CustomizableGears {
    fn get_config_type(&self) -> GearConfigType {
        GearConfigType::PerGearConfig
    }

    fn handle_gear_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            CustomizedGear(update) => { match update {
                CustomizedGearUpdate::AddGear() => {
                    let next_idx: usize;
                    if let Some((l, _)) = self.new_setup_data.last_key_value() {
                        next_idx = l.idx + 1;
                    } else {
                        next_idx = 1;
                    }
                    if next_idx <= 10 {
                        self.new_setup_data.insert(GearLabel{idx: next_idx}, RatioSet::new());
                    }
                }
                CustomizedGearUpdate::DefaultRatioSelected(gear, ratio_idx) => {
                    if let Some(ratio_set) = self.new_setup_data.get_mut(&gear) {
                        let _ = ratio_set.set_default(ratio_idx);
                    }
                }
                CustomizedGearUpdate::RemoveGear() => {
                    if !self.new_setup_data.is_empty() {
                        self.new_setup_data.pop_last();
                    }
                }
                CustomizedGearUpdate::AddRatio(gear_idx) => {
                    self.new_ratio_data = Some((gear_idx, String::new(), String::new()));
                }
                CustomizedGearUpdate::RemoveRatio(label, ratio_idx) => {
                    if let Some(ratio_set) = self.new_setup_data.get_mut(&label) {
                        ratio_set.remove(ratio_idx);
                    }
                }
                CustomizedGearUpdate::UpdateRatioName(new_val) => {
                    if let Some((_, name, _)) = &mut self.new_ratio_data {
                        *name = new_val;
                    }
                }
                CustomizedGearUpdate::UpdateRatioValue(new_val) => {
                    if let Some((_, _, ratio)) = &mut self.new_ratio_data {
                        if is_valid_ratio(&new_val) {
                            *ratio = new_val;
                        }
                    }
                }
                CustomizedGearUpdate::ConfirmNewRatio() => {
                    if let Some((label , name, ratio)) = &self.new_ratio_data {
                        match self.new_setup_data.get_mut(label) {
                            None => {}
                            Some(ratio_set) => {
                                match ratio.parse::<f64>() {
                                    Ok(ratio_f) => {
                                        let gear_name = match name.is_empty() {
                                            true => ratio.clone(),
                                            false => name.clone()
                                        };
                                        ratio_set.insert(gear_name, ratio_f);
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        self.new_ratio_data = None;
                    }
                }
                CustomizedGearUpdate::DiscardNewRatio() => {
                    self.new_ratio_data = None;
                }
            }}
            _ => {}
        }
    }

    fn handle_final_drive_update(&mut self, update_type: FinalDriveUpdate) {
        self.final_drive_data.handle_update(update_type)
    }

    fn add_editable_gear_list<'a, 'b>(&'a self, mut layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut gearset_roe = Row::new().spacing(5).padding(Padding::from([0, 10])).width(Length::Shrink);
        for (gear_idx, ratio_set) in &self.new_setup_data {
            let mut gear_col = Self::create_gear_ratio_column(gear_idx, ratio_set);
            if let Some((adding_gear_label, ratio_name, ratio)) = &self.new_ratio_data {
                if adding_gear_label == gear_idx {
                    let max_len = max(ratio_set.max_name_len(), ratio_name.len());
                    let name_width = (max_len * 10).to_u16().unwrap_or(100);
                    gear_col = gear_col.push(Self::add_gear_ratio_entry_row((adding_gear_label.clone(), ratio_name.clone(), ratio.clone()), name_width))
                } else {
                    gear_col = gear_col.push(Self::add_gear_ratio_button(gear_idx.clone()));
                }
            } else {
                gear_col = gear_col.push(Self::add_gear_ratio_button(gear_idx.clone()));
            }
            gearset_roe = gearset_roe.push(gear_col);
        }
        gearset_roe = gearset_roe.push(self.final_drive_data.create_final_drive_column());
        let s = scrollable(gearset_roe).horizontal_scroll(Properties::default()).height(Length::FillPortion(6));
        layout = layout.push(s);
        let mut add_remove_row = Row::new().height(Length::Shrink).width(Length::Shrink).spacing(5);
        let add_gear_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(CustomizedGear(CustomizedGearUpdate::AddGear())));
        add_remove_row = add_remove_row.push(add_gear_button);
        let delete_gear_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(GearUpdate(CustomizedGear(CustomizedGearUpdate::RemoveGear())))
            .style(Button::Destructive);
        add_remove_row = add_remove_row.push(delete_gear_button);
        layout.push(Container::new(add_remove_row).height(Length::FillPortion(1)).align_y(Vertical::Top).padding(0))
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GearLabel {
    idx: usize
}

impl Display for GearLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} gear", SingleGear::create_gear_name(self.idx))
    }
}

impl From<GearLabel> for usize {
    fn from(label: GearLabel) -> usize {
        label.idx
    }
}

impl From<usize> for GearLabel {
    fn from(value: usize) -> Self {
        GearLabel { idx: value }
    }
}

fn is_valid_ratio(val: &str) -> bool {
    if val.is_empty() || val.parse::<f64>().is_ok() {
        return true;
    }
    false
}
