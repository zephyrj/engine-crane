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
use std::collections::{HashMap, BTreeMap, HashSet, BTreeSet};
use std::num::ParseFloatError;
use std::path::PathBuf;
use fraction::ToPrimitive;
use iced::{Alignment, Length, Padding, Theme};
use iced::alignment::{Horizontal, Vertical};
use iced::theme::Button;
use iced::widget::{button, Column, Container, Row, scrollable, text, Text, text_input};
use iced::widget::scrollable::Properties;

use tracing::{error, warn};
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::setup::Setup;
use crate::assetto_corsa::car::data::setup::gears::{GearSet, GearConfig, GearData, SingleGear};
use crate::assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};
use crate::ui::button::{create_add_button, create_delete_button, create_disabled_add_button};
use crate::ui::edit::EditMessage;
use crate::ui::edit::gears::{GearConfigChoice, GearIdentifier, GearLabel, GearUpdateType, RatioSet};


pub fn gear_configuration_builder(ac_car_path: &PathBuf) -> Result<Box<dyn GearConfiguration>, String> {
    let mut car = match Car::load_from_path(ac_car_path) {
        Ok(c) => { c }
        Err(err) => {
            let err_str = format!("Failed to load {}. {}", ac_car_path.display(), err.to_string());
            error!("{}", &err_str);
            return Err(err_str);
        }
    };
    let drivetrain_data: Vec<f64>;
    match Drivetrain::from_car(&mut car) {
        Ok(drivetrain) => {
            match extract_mandatory_section::<data::drivetrain::Gearbox>(&drivetrain) {
                Ok(gearbox) => {
                    drivetrain_data = gearbox.clone_gear_ratios();
                }
                Err(err) => {
                    return Err(format!("Failed to load Gearbox data from {}. {}", ac_car_path.display(), err.to_string()));
                }
            }
        },
        Err(err) => {
            return Err(format!("Failed to load drivetrain from {}. {}", ac_car_path.display(), err.to_string()));
        }
    };

    let setup_data = match Setup::from_car(&mut car) {
        Ok(opt) => {
            match opt {
                Some(setup_data) => {
                    match GearData::load_from_parent(&setup_data) {
                        Ok(gear_data) => {
                            Some(gear_data)
                        }
                        Err(err) => {
                            return Err(format!("Failed to load gear data from {}. {}", ac_car_path.display(), err.to_string()));
                        }
                    }
                }
                None => None
            }
        }
        Err(err) => {
            warn!("Failed to load {}.{}", ac_car_path.display(), err.to_string());
            None
        }
    };

    let config_type = match &setup_data {
        None => GearConfigChoice::Fixed,
        Some(gear_data) => {
            match &gear_data.gear_config {
                None => GearConfigChoice::Fixed,
                Some(config) => {
                    match config {
                        GearConfig::GearSets(_) => GearConfigChoice::GearSets,
                        GearConfig::PerGear(_) => GearConfigChoice::PerGearConfig
                    }
                }
            }
        }
    };
    return match config_type {
        GearConfigChoice::Fixed => {
            let updated_drivetrain_data = drivetrain_data.iter().enumerate().map(|(idx, _)| (idx, None)).collect();
            Ok(Box::new(FixedGears {
                current_drivetrain_data: drivetrain_data,
                updated_drivetrain_data
            }))
        }
        GearConfigChoice::GearSets => {
            let current_setup_data;
            match setup_data.unwrap().gear_config.unwrap() {
                GearConfig::GearSets(sets) => {
                    current_setup_data = sets
                }
                _ => {
                    current_setup_data = Vec::new();
                }
            }
            //updated_drivetrain_data: BTreeMap<String, BTreeMap<usize, Option<String>>>
            let updated_drivetrain_data = current_setup_data.iter().enumerate().map(|(idx, gear_set)| {
                let ratio_map: BTreeMap<usize, Option<String>> = gear_set.ratios().iter().enumerate().map(|(idx, _)| (idx, None)).collect();
                (idx, ratio_map)
            }).collect();
            Ok(Box::new(GearSets {
                current_drivetrain_data: drivetrain_data,
                current_setup_data,
                updated_drivetrain_data
            }))
        }
        GearConfigChoice::PerGearConfig => {
            let mut current_setup_data = Vec::new();
            let mut new_setup_data= BTreeMap::new();
            match setup_data.unwrap().gear_config.unwrap() {
                GearConfig::PerGear(gears) => {
                    current_setup_data = gears;
                    for gear in &current_setup_data {
                        let gear_vec = gear.ratios_lut.to_vec();
                        let mut ratio_set = RatioSet::new();
                        gear_vec.iter().for_each(|pair| { ratio_set.insert(pair.0.clone(), pair.1); });
                        new_setup_data.insert(gear.get_index().map_err(|e| { e.to_string()})?.into(),
                                              ratio_set);
                    }
                }
                _ => {
                    current_setup_data = Vec::new();
                    new_setup_data = BTreeMap::new();
                }
            }
            Ok(Box::new(CustomizableGears {
                current_drivetrain_data: drivetrain_data,
                current_setup_data,
                new_setup_data,
                new_ratio_data: None
            }))
        }
    }
}


pub trait GearConfiguration {
    fn get_config_type(&self) -> GearConfigChoice;
    fn handle_update(&mut self, update_type: GearUpdateType);
    fn add_editable_gear_list<'a, 'b>(&'a self, layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        layout
    }
}

pub struct FixedGears {
    current_drivetrain_data: Vec<f64>,
    updated_drivetrain_data: BTreeMap<usize, Option<String>>
}

impl FixedGears {
    fn create_gear_ratio_column(row_vals: Vec<(String, String)>) -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
        let mut max_gear_idx = 0;
        for (gear_idx, (placeholder, new_ratio)) in row_vals.iter().enumerate() {
            let mut gear_row = Row::new()
                .width(Length::Shrink)
                .align_items(Alignment::Center)
                .spacing(5);
            let l = Text::new(format!("Gear {}", gear_idx+1)).vertical_alignment(Vertical::Bottom);
            let t = text_input(
                placeholder,
                new_ratio,
                move |new_value| {
                    EditMessage::GearUpdate(GearUpdateType::UpdateRatioValue(GearIdentifier::Fixed(gear_idx), new_value))
                }
            ).width(Length::Units(84));
            gear_row = gear_row.push(l).push(t);
            gear_list = gear_list.push(gear_row);
            max_gear_idx = gear_idx;
        }
        let mut add_remove_row = Row::new().width(Length::Shrink).spacing(5);
        let add_gear_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::AddGear()));
        add_remove_row = add_remove_row.push(add_gear_button);
        let delete_gear_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::RemoveGear()));
        add_remove_row = add_remove_row.push(delete_gear_button);
        gear_list = gear_list.push(add_remove_row);
        gear_list
    }
}

impl GearConfiguration for FixedGears {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::Fixed
    }

    // TODO return a Result so errors can be passed somewhere for viewing
    fn handle_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            GearUpdateType::UpdateRatioValue(gear_idx, ratio) => {
                match gear_idx {
                    GearIdentifier::Fixed(gear_idx) => {
                        if ratio.is_empty() {
                            self.updated_drivetrain_data.insert(gear_idx, None);
                        } else {
                            self.updated_drivetrain_data.insert(gear_idx, Some(ratio));
                        }
                    },
                    _ => {}
                }
            },
            GearUpdateType::AddGear() => {
                let gear_idx: usize = match self.updated_drivetrain_data.last_key_value() {
                    None => { 0 }
                    Some((max_gear_idx, _)) => { max_gear_idx+1 }
                };
                self.updated_drivetrain_data.insert(gear_idx, None);
            },
            GearUpdateType::RemoveGear() => {
                self.updated_drivetrain_data.pop_last();
            }
            _ => {}
        }
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
        layout.push(Self::create_gear_ratio_column(displayed_ratios))
    }
}

pub struct GearSets {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Vec<GearSet>,
    updated_drivetrain_data: BTreeMap<usize, BTreeMap<usize, Option<String>>>
}

impl GearSets {
    fn create_gear_ratio_column(gearset_idx: usize, row_vals: Vec<(String, String)>) -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
        let mut max_gear_idx = 0;
        for (gear_idx, (placeholder, new_ratio)) in row_vals.iter().enumerate() {
            let mut gear_row = Row::new()
                .width(Length::Shrink)
                .align_items(Alignment::Center)
                .spacing(5);
            let l = Text::new(format!("Gear {}", gear_idx+1)).vertical_alignment(Vertical::Bottom);
            let t = text_input(
                placeholder,
                new_ratio,
                move |new_value| {
                    EditMessage::GearUpdate(GearUpdateType::UpdateRatioValue(GearIdentifier::GearSet(gearset_idx, gear_idx), new_value))
                }
            ).width(Length::Units(84));
            gear_row = gear_row.push(l).push(t);
            gear_list = gear_list.push(gear_row);
            max_gear_idx = gear_idx;
        }
        gear_list
    }
}

impl GearConfiguration for GearSets {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::GearSets
    }

    fn handle_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            GearUpdateType::UpdateRatioValue(gear_idx, ratio) => {
                match gear_idx {
                    GearIdentifier::GearSet(set_idx, gear_idx) => {
                        if let Some(gear_set) = self.updated_drivetrain_data.get_mut(&set_idx) {
                            if ratio.is_empty() {
                                gear_set.insert(gear_idx, None);
                            } else {
                                gear_set.insert(gear_idx, Some(ratio));
                            }
                        }
                    },
                    _ => {}
                }
            },
            GearUpdateType::AddGear() => {
                for gear_set in self.updated_drivetrain_data.values_mut() {
                    let gear_idx: usize = match gear_set.last_key_value() {
                        None => { 0 }
                        Some((max_gear_idx, _)) => { max_gear_idx+1 }
                    };
                    gear_set.insert(gear_idx, None);
                }
            },
            GearUpdateType::RemoveGear() => {
                for gear_set in self.updated_drivetrain_data.values_mut() {
                    gear_set.pop_last();
                }
            }
            _ => {}
        }
    }

    fn add_editable_gear_list<'a, 'b>(&'a self, mut layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut gearset_row = Row::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
        for (set_idx, gearset_map) in self.updated_drivetrain_data.iter() {
            let mut col = Column::new().align_items(Alignment::Center).spacing(5).padding(Padding::from([0, 10]));
            let mut displayed_ratios = Vec::new();
            for (gear_idx, ratio) in gearset_map.iter() {
                let mut placeholder = String::new();
                let displayed_ratio = match ratio {
                    None => {
                        if let Some(current_set) = self.current_setup_data.get(*set_idx) {
                            if let Some(current_ratio) = current_set.ratios().get(*gear_idx) {
                                placeholder = current_ratio.to_string();
                            }
                        }
                        String::new()
                    }
                    Some(ratio) => ratio.clone()
                };
                displayed_ratios.push((placeholder, displayed_ratio));
            }
            col = col.push(text(format!("GEARSET_{}", set_idx)));
            col = col.push(Self::create_gear_ratio_column(*set_idx, displayed_ratios));
            gearset_row = gearset_row.push(col);
        }
        layout = layout.push(gearset_row);
        let mut add_remove_row = Row::new().width(Length::Shrink).spacing(5);
        let add_ratio_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::AddGear()));
        add_remove_row = add_remove_row.push(add_ratio_button);
        let delete_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::RemoveGear()));
        add_remove_row = add_remove_row.push(delete_button);
        layout.push(add_remove_row)
    }
}

pub struct CustomizableGears {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Vec<SingleGear>,
    new_setup_data: BTreeMap<GearLabel, RatioSet>,
    new_ratio_data: Option<(GearLabel, String, String)>
}

impl CustomizableGears {
    fn create_gear_ratio_column(gear_idx: &GearLabel, ratio_set: &RatioSet ) -> Column<'static, EditMessage>
    {
        let mut col = Column::new().align_items(Alignment::Center).width(Length::Shrink).spacing(5).padding(Padding::from([0, 10, 12, 10]));
        col = col.push(text(gear_idx));
        let name_width = (ratio_set.max_name_length * 10).to_u16().unwrap_or(u16::MAX);
        for ratio_entry in ratio_set.entries() {
            let mut name_label = Text::new(ratio_entry.name.clone()).width(Length::Units(name_width));
            name_label = name_label.size(14);
            let ratio_string = ratio_entry.ratio.to_string();
            let mut ratio_input = Text::new(ratio_string).width(Length::Units(56));
            ratio_input = ratio_input.size(14);
            let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
            r = r.push(name_label);
            r = r.push(ratio_input);
            r = r.push(create_delete_button(EditMessage::GearUpdate(GearUpdateType::RemoveRatio(GearIdentifier::CustomizedGears(gear_idx.clone(), ratio_entry.idx)))).height(Length::Units(20)).width(Length::Units(20)));
            col = col.push(r);
        }
        col
    }

    fn add_gear_ratio_button(label: GearLabel) -> iced::widget::Button<'static, EditMessage> {
        iced::widget::button(
            text("Add Ratio").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::AddRatio(GearIdentifier::CustomizedGears(label, 0))))
    }

    fn add_gear_ratio_entry_row(new_ratio_data: (GearLabel, String, String), name_max_width: u16) -> Row<'static, EditMessage>
    {
        let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
        let (label, name, ratio) = new_ratio_data;
        r = r.push(text_input("", &name, move |new_val| { EditMessage::GearUpdate(GearUpdateType::UpdateRatioName(GearIdentifier::CustomizedGears(label.clone(), 0), new_val))}).width(Length::Units(name_max_width)).size(14));
        r = r.push(text_input("", &ratio, move |new_val| { EditMessage::GearUpdate(GearUpdateType::UpdateRatioValue(GearIdentifier::CustomizedGears(label.clone(), 0), new_val))}).width(Length::Units(56)).size(14));
        let mut confirm;
        if !ratio.is_empty() {
            confirm = create_add_button(EditMessage::GearUpdate(GearUpdateType::ConfirmNewRatio()));
        } else {
            confirm = create_disabled_add_button().height(Length::Units(20)).width(Length::Units(20));
        }
        r = r.push(confirm.height(Length::Units(20)).width(Length::Units(20)));
        r = r.push(create_delete_button(EditMessage::GearUpdate(GearUpdateType::DiscardNewRatio())).height(Length::Units(20)).width(Length::Units(20)));
        r
    }
}

impl GearConfiguration for CustomizableGears {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::PerGearConfig
    }

    fn handle_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            GearUpdateType::AddRatio(gear_idx) => {
                match gear_idx {
                    GearIdentifier::CustomizedGears(gear_idx, ratio_idx) => {
                        self.new_ratio_data = Some((gear_idx, String::new(), String::new()));
                    },
                    _ => {}
                }
            },
            GearUpdateType::UpdateRatioName(gear_idx, new_val) => {
                match gear_idx {
                    GearIdentifier::CustomizedGears(_, _) => {
                        if let Some((_, name, _)) = &mut self.new_ratio_data {
                            *name = new_val;
                        }
                    },
                    _ => {}
                }
            },
            GearUpdateType::UpdateRatioValue(gear_idx, new_val) => {
                match gear_idx {
                    GearIdentifier::CustomizedGears(_, _) => {
                        if let Some((_, _, ratio)) = &mut self.new_ratio_data {
                            if new_val.is_empty() || new_val.parse::<f64>().is_ok() {
                                *ratio = new_val;
                            }
                        }
                    },
                    _ => {}
                }
            },
            GearUpdateType::ConfirmNewRatio() => {
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

            },
            GearUpdateType::DiscardNewRatio() => {
                self.new_ratio_data = None;
            },
            GearUpdateType::RemoveRatio(gear_idx) => {
                match gear_idx {
                    GearIdentifier::CustomizedGears(label, ratio_idx) => {
                        if let Some(ratio_set) = self.new_setup_data.get_mut(&label) {
                            ratio_set.remove(ratio_idx);
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn add_editable_gear_list<'a, 'b>(&'a self, mut layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut gearset_roe = Row::new().spacing(5).padding(Padding::from([0, 10])).width(Length::Shrink);
        for (gear_idx, ratio_set) in &self.new_setup_data {
            let mut gear_col = Self::create_gear_ratio_column(gear_idx, ratio_set);
            if let Some((adding_gear_label, ratio_name, ratio)) = &self.new_ratio_data {
                if adding_gear_label == gear_idx {
                    let max_len = max(ratio_set.max_name_length, ratio_name.len());
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
        let s = scrollable(gearset_roe).horizontal_scroll(Properties::default()).height(Length::FillPortion(6));
        layout = layout.push(s);
        let mut add_remove_row = Row::new().height(Length::Shrink).width(Length::Shrink).spacing(5);
        let add_gear_button = iced::widget::button(
            text("Add Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::AddGear()));
        add_remove_row = add_remove_row.push(add_gear_button);
        let delete_gear_button = iced::widget::button(
            text("Delete Gear").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
        )   .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::RemoveGear()))
            .style(Button::Destructive);
        add_remove_row = add_remove_row.push(delete_gear_button);
        layout.push(Container::new(add_remove_row).height(Length::FillPortion(1)).align_y(Vertical::Top).padding(0))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DeleteButtonStyle;

impl button::StyleSheet for DeleteButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.89,0.15,0.21))),
            text_color: iced::Color::BLACK,
            ..Default::default()
        }
    }
    // other methods in Stylesheet have a default impl
}

// #[derive(Clone, Copy, Debug)]
// pub struct GearStyle;
//
// impl scrollable::StyleSheet for GearStyle {
//     type Style = Theme;
//
//     fn active(&self, _style: &Self::Style) -> scrollable::Appearance {
//         button::Appearance {
//             background: Some(iced::Background::Color(iced::Color::from_rgb(0.89,0.15,0.21))),
//             text_color: iced::Color::BLACK,
//             ..Default::default()
//         }
//     }
//     // other methods in Stylesheet have a default impl
// }
