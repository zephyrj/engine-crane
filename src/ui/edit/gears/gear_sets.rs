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
use std::fmt::{Display, Formatter};
use iced::{Alignment, Length, Padding, theme};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, Radio, Row, Text};
use iced_native::widget::{text, text_input, vertical_rule};
use assetto_corsa::car::model::GearingCalculator;
use crate::assetto_corsa::car;

use crate::assetto_corsa::car::data::{Drivetrain, setup};
use crate::assetto_corsa::traits::{CarDataUpdater, MandatoryDataSection};
use crate::ui::button::{create_add_button, create_delete_button, create_disabled_delete_button};

use crate::ui::edit::EditMessage;
use crate::ui::edit::EditMessage::GearUpdate;
use crate::ui::edit::gears::final_drive::FinalDrive;
use crate::ui::edit::gears::{create_max_ratio_speed_element, FinalDriveUpdate, GearConfigType, GearUpdateType};
use crate::ui::edit::gears::customizable::CustomizableGears;
use crate::ui::edit::gears::fixed::FixedGears;
use crate::ui::edit::gears::GearsetUpdate::DefaultGearsetSelected;
use crate::ui::edit::gears::GearUpdateType::Gearset;


#[derive(Debug, Clone, PartialEq)]
pub enum GearsetUpdate {
    AddGear(),
    RemoveGear(),
    AddGearset(),
    RemoveGearset(GearsetLabel),
    UpdateRatio(GearsetLabel, usize, String),
    DefaultGearsetSelected(GearsetLabel)
}

pub struct GearSetContainer {
    next_idx: usize,
    num_gears: usize,
    default: Option<GearsetLabel>,
    entries: BTreeMap<GearsetLabel, BTreeMap<usize, Option<String>>>,
    original_values: BTreeMap<GearsetLabel, BTreeMap<usize, String>>,
    gearing_calculator: Option<GearingCalculator>
}

impl GearSetContainer {
    #[allow(dead_code)]
    fn new(num_gears: usize,
           default: Option<GearsetLabel>,
           entries: BTreeMap<GearsetLabel, BTreeMap<usize, Option<String>>>,
           original_values: BTreeMap<GearsetLabel, BTreeMap<usize, String>>,) -> GearSetContainer {
        let next_idx = match entries.last_key_value() {
            None => 0,
            Some((label, _)) => label.idx + 1
        };
        GearSetContainer {
            next_idx, num_gears, default, entries, original_values, gearing_calculator: None
        }
    }

    fn from_setup_data(drivetrain_data: &Vec<f64>,
                       drivetrain_setup_data: &Option<setup::gears::GearConfig>) -> Self
    {
        let mut num_gears: usize = drivetrain_data.len();
        let mut default = None;
        let mut original_values = BTreeMap::new();
        let mut entries = BTreeMap::new();
        match drivetrain_setup_data {
            None => {},
            Some(gear_config) => match gear_config {
                setup::gears::GearConfig::GearSets(gear_sets) => {
                    if !gear_sets.is_empty() {
                        num_gears = gear_sets[0].num_gears()
                    }
                    for (set_idx, set) in gear_sets.iter().enumerate() {
                        let label = GearsetLabel::new(set_idx, set.name().to_string());
                        let mut original_val_map = BTreeMap::new();
                        let mut ratio_map: BTreeMap<usize, Option<String>> = BTreeMap::new();
                        let mut is_default_set = true;
                        for (gear_idx, gear_ratio) in set.ratios().iter().enumerate() {
                            if gear_idx >= num_gears {
                                break;
                            }
                            if is_default_set {
                                if let Some(drivetrain_ratio) = drivetrain_data.get(gear_idx) {
                                    if *drivetrain_ratio != *gear_ratio {
                                        is_default_set = false;
                                    }
                                } else {
                                    is_default_set = false;
                                }
                            }
                            original_val_map.insert(gear_idx, gear_ratio.to_string());
                            ratio_map.insert(gear_idx, None);
                        }
                        original_values.insert(label.clone(), original_val_map);
                        if is_default_set {
                            default = Some(label.clone());
                        }
                        entries.insert(label, ratio_map);
                    }
                }
                _ => {}
            }
        };
        let next_idx = match entries.last_key_value() {
            None => 0,
            Some((label, _)) => label.idx + 1
        };
        GearSetContainer {
            next_idx, num_gears, default, entries, original_values, gearing_calculator: None
        }
    }

    pub fn final_drive_updated(&mut self, ratio: f64) {
        if let Some(gear_calc) = &mut self.gearing_calculator {
            gear_calc.set_final_drive(ratio)
        }
    }

    pub(crate) fn set_gearing_calculator(&mut self, calculator: GearingCalculator) {
        self.gearing_calculator = Some(calculator)
    }

    pub(crate) fn extract_gearing_calculator(&mut self) -> Option<GearingCalculator> {
        self.gearing_calculator.take()
    }

    fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }


    fn set_default_gearset(&mut self, label: &GearsetLabel) {
        if self.entries.contains_key(label) {
            self.default = Some(label.clone())
        }
    }

    fn default_ratios(&self) -> Vec<Option<String>> {
        let mut no_default = Vec::new();
        for _ in 0..self.num_gears {
            no_default.push(None);
        }
        if self.entries.is_empty() {
            return no_default;
        }
        let default_label = match &self.default {
            None => match self.entries.first_key_value() {
                None => return no_default,
                Some((label, _)) => label
            }
            Some(label) => label
        };
        self.entries.get(default_label).unwrap().values().enumerate().map(|(idx,ratio_opt)| {
            match ratio_opt {
                None => self.get_og_ratio(default_label, idx),
                Some(_) => ratio_opt.clone()
            }
        }).collect()
    }

    fn update_ratio(&mut self, gearset: &GearsetLabel, gear_idx: usize, ratio: Option<String>) {
        match self.entries.get_mut(gearset) {
            None => {}
            Some(set) => {
                set.insert(gear_idx, ratio);
            }
        }
    }

    fn add_gearset(&mut self) -> GearsetLabel {
        let mut new_ratio_map = BTreeMap::new();
        for idx in 0..self.num_gears {
            new_ratio_map.insert(idx, None);
        }
        let new_label =
            GearsetLabel::new(self.next_idx, format!("GEARSET_{}", self.next_idx));
        self.entries.insert(new_label.clone(), new_ratio_map);
        self.next_idx += 1;
        new_label
    }

    fn remove_gearset(&mut self, label: &GearsetLabel) {
        match self.entries.remove(label) {
            None => {}
            Some(_) => if let Some(default) = &self.default {
                if default == label {
                    self.default = None;
                }
            }
        }
    }

    fn add_gear(&mut self) {
        for gear_set in self.entries.values_mut() {
            gear_set.insert(self.num_gears, None);
        }
        self.num_gears += 1;
    }

    fn remove_gear(&mut self) {
        for gear_set in self.entries.values_mut() {
            gear_set.pop_last();
        }
        self.num_gears -= 1;
    }

    fn get_all_ratios(&self) -> Vec<Vec<String>> {
        let mut gear_ratios_vec = Vec::new();
        for _ in 0..self.num_gears {
            gear_ratios_vec.push(Vec::new())
        }
        for (label, gearset) in &self.entries {
            for (ratio_idx, ratio_opt) in gearset {
                match ratio_opt {
                    None => match self.original_values.get(&label) {
                        None => {}
                        Some(og_set) => match og_set.get(&ratio_idx) {
                            None => {}
                            Some(ratio) => match gear_ratios_vec.get_mut(*ratio_idx) {
                                None => {}
                                Some(gear_vec) => gear_vec.push(ratio.clone())
                            }
                        }
                    }
                    Some(ratio) => match gear_ratios_vec.get_mut(*ratio_idx) {
                        None => {}
                        Some(gear_vec) => gear_vec.push(ratio.clone())
                    }
                }
            }
        }
        gear_ratios_vec
    }

    fn get_og_ratio(&self, label: &GearsetLabel, gear_idx: usize) -> Option<String> {
        Some(self.original_values.get(&label)?.get(&gear_idx)?.clone())
    }

    fn to_setup_map(&self) -> BTreeMap<String, Vec<f64>> {
        self.entries.iter().map(
            |(label, ratio_map)| {
                let ratio_vec: Vec<f64> = ratio_map.iter().map(
                    |(idx, ratio_opt)| {
                        match ratio_opt {
                            None => {
                                let s = self.get_og_ratio(&label, *idx).unwrap_or("1".to_string());
                                s.parse().unwrap_or(1f64)
                            },
                            Some(ratio) => ratio.parse::<f64>().unwrap_or(1f64)
                        }
                    }
                ).collect();
                (label.name.clone(), ratio_vec)
            }
        ).collect()
    }

    fn create_gearset_lists(&self) -> Row<'static, EditMessage>
    {
        let mut gearset_row = Row::new()
            .height(Length::Shrink)
            .width(Length::Shrink)
            .spacing(5)
            .padding(Padding::from([0, 10]))
            .align_items(Alignment::Fill);
        for (gearset_label, gearset_map) in self.entries.iter() {
            let mut col = Column::new()
                .align_items(Alignment::Center)
                .spacing(5)
                .padding(Padding::from([0, 10]))
                .height(Length::Shrink);
            let mut displayed_ratios = Vec::new();
            for (gear_idx, ratio) in gearset_map.iter() {
                let placeholder = match self.original_values.get(gearset_label) {
                    None => String::new(),
                    Some(set) => match set.get(gear_idx) {
                        None => String::new(),
                        Some(current_ratio) =>  current_ratio.clone()
                    }
                };
                let displayed_ratio = match ratio {
                    None => String::new(),
                    Some(ratio) => ratio.clone()
                };
                displayed_ratios.push((placeholder, displayed_ratio));
            }
            col = col.push(text(format!("{}", gearset_label)));
            col = col.push(self.create_gear_ratio_column(gearset_label.clone(), displayed_ratios));
            gearset_row = gearset_row.push(col);
        }
        gearset_row.push(
            Container::new(
                create_add_button(GearUpdate(Gearset(GearsetUpdate::AddGearset())))
                    .width(Length::Units(30))
                    .height(Length::Units(30))
            ).align_y(Vertical::Center).height(Length::Fill)
        )
    }

    fn create_gear_ratio_column(&self,
                                gearset_label: GearsetLabel,
                                row_vals: Vec<(String, String)>)
        -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new()
            .width(Length::Shrink)
            .spacing(5)
            .align_items(Alignment::Center);
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
        let selected = match &self.default {
            None => None,
            Some(label) => Some(label.idx)
        };

        let delete_button = match self.entries.len() > 1 {
            true => create_delete_button(GearUpdate(Gearset(GearsetUpdate::RemoveGearset(gearset_label.clone())))),
            false => create_disabled_delete_button()
        };

        let aux_element = Container::new( Radio::new(
            gearset_label.idx(),
            "Default",
            selected,
            move |_| { GearUpdate(Gearset(DefaultGearsetSelected(gearset_label.clone()))) }
        ).size(10).text_size(14).spacing(5)).align_x(Horizontal::Center);
        gear_list = gear_list.push(aux_element);
        gear_list.push(delete_button)
    }
}

pub struct GearSets {
    original_drivetrain_data: Vec<f64>,
    original_setup_data: Option<setup::gears::GearConfig>,
    updated_gearsets: GearSetContainer,
    final_drive_data: FinalDrive
}

impl From<FixedGears> for GearSets {
    fn from(mut value: FixedGears) -> Self {
        let original_drivetrain_data = value.extract_original_drivetrain_data();
        let original_setup_data = value.extract_original_setup_data();
        let mut updated_gearsets =
            GearSetContainer::from_setup_data(&original_drivetrain_data, &original_setup_data);

        let mut new_gearset_label: Option<GearsetLabel> = None;
        let mut missed_ratio_idx = Vec::new();
        for (ratio_idx, opt) in value.get_updated_ratios().iter().enumerate() {
            match opt {
                None => missed_ratio_idx.push(ratio_idx),
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
        } else if new_gearset_label.is_some() {
            for idx in missed_ratio_idx {
                match original_drivetrain_data.get(idx) {
                    None => continue,
                    Some(og_ratio) => {
                        updated_gearsets.update_ratio(new_gearset_label.as_ref().unwrap(),
                                                      idx,
                                                      Some(og_ratio.to_string()));
                    }
                }
            }
        }
        let mut config =
            GearSets::new(original_drivetrain_data,
                          original_setup_data,
                          updated_gearsets,
                          value.extract_final_drive_data());
        if let Some(gear_calc) = value.extract_gearing_calculator() {
            config.set_gearing_calculator(gear_calc);
        }
        config
    }
}

impl From<CustomizableGears> for GearSets {
    fn from(mut value: CustomizableGears) -> Self {
        let original_drivetrain_data = value.extract_original_drivetrain_data();
        let original_setup_data = value.extract_original_setup_data();
        let mut updated_gearsets =
            GearSetContainer::from_setup_data(&original_drivetrain_data, &original_setup_data);
        let mut new_gearset_label: Option<GearsetLabel> = None;
        for (ratio_idx, opt) in value.get_default_gear_ratios().iter().enumerate() {
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
        let mut config =
            GearSets::new(original_drivetrain_data,
                          original_setup_data,
                          updated_gearsets,
                          value.extract_final_drive_data());
        if let Some(gear_calc) = value.extract_gearing_calculator() {
            config.set_gearing_calculator(gear_calc);
        }
        config
    }
}

impl GearSets {
    pub fn new(original_drivetrain_data: Vec<f64>,
               original_setup_data: Option<setup::gears::GearConfig>,
               updated_gearsets: GearSetContainer,
               final_drive_data: FinalDrive,) -> GearSets {
        GearSets {
            original_drivetrain_data,
            original_setup_data,
            updated_gearsets,
            final_drive_data
        }
    }

    pub(crate) fn from_gear_data(drivetrain_data: Vec<f64>,
                                 setup_data: Option<setup::gears::GearConfig>,
                                 final_drive_data: FinalDrive) -> GearSets
    {
        let updated_gearsets = GearSetContainer::from_setup_data(&drivetrain_data, &setup_data);
        GearSets::new(drivetrain_data, setup_data, updated_gearsets, final_drive_data)
    }

    pub(crate) fn set_gearing_calculator(&mut self, mut calculator: GearingCalculator) {
        calculator.set_final_drive(self.final_drive_data.get_default_ratio_val());
        self.updated_gearsets.set_gearing_calculator(calculator);
    }

    pub(crate) fn extract_gearing_calculator(&mut self) -> Option<GearingCalculator> {
        self.updated_gearsets.extract_gearing_calculator()
    }

    pub(crate) fn extract_original_drivetrain_data(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.original_drivetrain_data)
    }

    pub(crate) fn extract_original_setup_data(&mut self) -> Option<setup::gears::GearConfig> {
        std::mem::take(&mut self.original_setup_data)
    }

    pub(crate) fn extract_final_drive_data(&mut self) -> FinalDrive {
        std::mem::take(&mut self.final_drive_data)
    }

    pub(crate) fn get_default_ratios(&self) -> Vec<Option<String>> {
        self.updated_gearsets.default_ratios()
    }

    pub(crate) fn get_all_ratios(&self) -> Vec<Vec<String>> {
        self.updated_gearsets.get_all_ratios()
    }

    pub(crate) fn get_config_type(&self) -> GearConfigType {
        GearConfigType::GearSets
    }

    pub(crate) fn handle_gear_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            Gearset(update) => { match update {
                GearsetUpdate::AddGear() => {
                    self.updated_gearsets.add_gear();
                }
                GearsetUpdate::RemoveGear() => {
                    self.updated_gearsets.remove_gear();
                }
                GearsetUpdate::UpdateRatio(set_idx, gear_idx, ratio) => {
                    if ratio.is_empty() {
                        self.updated_gearsets.update_ratio(&set_idx, gear_idx, None);
                    } else if is_valid_ratio(&ratio) {
                        self.updated_gearsets.update_ratio(&set_idx, gear_idx, Some(ratio));
                    }
                }
                GearsetUpdate::DefaultGearsetSelected(label) => {
                    self.updated_gearsets.set_default_gearset(&label);
                }
                GearsetUpdate::AddGearset() => {
                    self.updated_gearsets.add_gearset();
                }
                GearsetUpdate::RemoveGearset(label) => {
                    self.updated_gearsets.remove_gearset(&label);
                }
            }}
            _ => {}
        }
    }

    pub(crate) fn handle_final_drive_update(&mut self, update_type: FinalDriveUpdate) {
        self.final_drive_data.handle_update(update_type);
        self.updated_gearsets.final_drive_updated(self.final_drive_data.get_default_ratio_val())
    }

    pub(crate) fn add_editable_gear_list<'a, 'b>(
        &'a self,
        mut layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut layout_row = Row::new().height(Length::Shrink).spacing(7).align_items(Alignment::Fill);
        let mut layout_col = Column::new().height(Length::Shrink);
        layout_col = layout_col.push(self.updated_gearsets.create_gearset_lists()).height(Length::Shrink);
        layout_row = layout_row.push(layout_col);
        layout_row = layout_row.push(vertical_rule(5));
        layout_row = layout_row.push(
            Column::new()
                .height(Length::Shrink)
                .padding(Padding::from([0,5]))
                .push(self.final_drive_data.create_final_drive_column())
        );
        layout = layout.push(layout_row);

        let mut add_ratio_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .style(theme::Button::Positive);
        if self.updated_gearsets.num_gears < 10 {
            add_ratio_button = add_ratio_button.on_press(GearUpdate(Gearset(GearsetUpdate::AddGear())));
        }

        let mut delete_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .style(theme::Button::Destructive);
        if self.updated_gearsets.num_gears > 1 {
            delete_button = delete_button.on_press(GearUpdate(Gearset(GearsetUpdate::RemoveGear())));
        }

        let mut add_remove_row = Row::new().width(Length::Shrink).spacing(5);
        add_remove_row = add_remove_row.push(add_ratio_button);
        add_remove_row = add_remove_row.push(delete_button);
        layout.push(add_remove_row)
    }

    pub(crate) fn apply_drivetrain_changes(&self, drivetrain: &mut Drivetrain) -> Result<(), String> {
        let mut gearbox_data =
            car::data::drivetrain::Gearbox::load_from_parent(drivetrain)
                .map_err(|e| format!("{}", e.to_string()))?;
        let default_ratios = self.updated_gearsets.default_ratios();
        let ratio_vec: Vec<f64> = default_ratios.iter().enumerate().map(
            |(idx, ratio_opt)| {
                match ratio_opt {
                    None => *self.original_drivetrain_data.get(idx).unwrap_or(&1f64),
                    Some(ratio) => {
                        ratio.parse::<f64>().unwrap_or(1f64)
                    }
                }
            }
        ).collect();
        let _ = gearbox_data.update_gears(ratio_vec);
        gearbox_data.update_car_data(drivetrain).map_err(|e| e.to_string())?;
        self.final_drive_data.apply_drivetrain_changes(drivetrain)?;
        Ok(())
    }

    pub(crate) fn apply_setup_changes(&self, gear_data: &mut setup::gears::GearData) -> Result<(), String> {
        gear_data.set_gear_config(
            Some(
                setup::gears::GearConfig::new_gearset_config_from_btree_map(self.updated_gearsets.to_setup_map())
            )
        );
        self.final_drive_data.apply_setup_changes(gear_data)?;
        Ok(())
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
