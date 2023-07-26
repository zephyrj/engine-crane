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

use std::collections::{HashMap, BTreeMap, HashSet, BTreeSet};
use std::path::PathBuf;
use fraction::ToPrimitive;
use iced::{Alignment, ContentFit, Length, Padding, Theme};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, Column, Row, scrollable, text, Text, text_input, TextInput};
use iced::widget::scrollable::Properties;
use iced::widget::Image;
use iced::widget::image::Handle;
use crate::ui::image_data::DELETE_IMAGE;

use tracing::{error, warn};
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::setup::Setup;
use crate::assetto_corsa::car::data::setup::gears::{GearSet, GearConfig, GearData, SingleGear};
use crate::assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};
use crate::ui::edit::EditMessage;
use crate::ui::edit::gears::{GearConfigChoice, GearConfigIdentifier, GearLabel, GearUpdateType, RatioEntry, RatioSet};


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
                        let mut count_vec: RatioSet = gear_vec.iter().map(|pair| RatioEntry::new(pair.0.clone(), pair.1)).collect();
                        new_setup_data.insert(gear.get_index().map_err(|e| { e.to_string()})?.into(),
                                              count_vec);
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
                new_setup_data
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
                    EditMessage::GearUpdate(GearUpdateType::UpdateRatio(GearConfigIdentifier::Fixed(gear_idx), new_value))
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
            GearUpdateType::UpdateRatio(gear_idx, ratio) => {
                match gear_idx {
                    GearConfigIdentifier::Fixed(gear_idx) => {
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
                    EditMessage::GearUpdate(GearUpdateType::UpdateRatio(GearConfigIdentifier::GearSet(gearset_idx, gear_idx), new_value))
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
            GearUpdateType::UpdateRatio(gear_idx, ratio) => {
                match gear_idx {
                    GearConfigIdentifier::GearSet(set_idx, gear_idx) => {
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
    new_setup_data: BTreeMap<GearLabel, RatioSet>
}

impl CustomizableGears {
    // TODO - For Customisable allow deletion of any gear ratio
    fn create_gear_ratio_column(row_vals: Vec<(String, String)>) -> Column<'static, EditMessage>
    {
        let mut gear_list = Column::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
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
                    EditMessage::GearUpdate(GearUpdateType::UpdateRatio(GearConfigIdentifier::Fixed(gear_idx), new_value))
                }
            ).width(Length::Units(84));
            gear_row = gear_row.push(l).push(t);
            let delete_button = iced::widget::button(
                text("X").horizontal_alignment(Horizontal::Center),
            )   .padding(12)
                .width(Length::Units(25))
                .height(Length::Units(25))
                .on_press(EditMessage::GearUpdate(GearUpdateType::RemoveGear()));
            gear_row = gear_row.push(delete_button);
            gear_list = gear_list.push(gear_row);
        }
        let add_ratio_button = iced::widget::button(
            text("Add Ratio").horizontal_alignment(Horizontal::Center),
        )   .padding(12)
            .width(Length::Units(75))
            .height(Length::Units(25))
            .on_press(EditMessage::GearUpdate(GearUpdateType::AddGear()));
        gear_list = gear_list.push(add_ratio_button);
        gear_list
    }
}

impl GearConfiguration for CustomizableGears {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::PerGearConfig
    }

    fn handle_update(&mut self, update_type: GearUpdateType) {
        todo!()
    }

    fn add_editable_gear_list<'a, 'b>(&'a self, layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut gearset_roe = Row::new().spacing(5).padding(Padding::from([0, 10])).width(Length::Shrink);
        for (gear_idx, ratio_set) in &self.new_setup_data {
            let mut col = Column::new().align_items(Alignment::Center).width(Length::Shrink).spacing(5).padding(Padding::from([0, 10, 12, 10]));
            col = col.push(text(gear_idx));
            let name_width = (ratio_set.max_name_length * 10).to_u16().unwrap_or(u16::MAX);
            for ratio_entry in ratio_set.entries() {
                let mut name_label = TextInput::new("", &ratio_entry.name, |e|{ EditMessage::GearUpdate(GearUpdateType::RemoveGear()) }).width(Length::Units(name_width));
                name_label = name_label.size(14);
                let mut ratio_input = TextInput::new("",&ratio_entry.ratio.to_string(), |e|{ EditMessage::GearUpdate(GearUpdateType::RemoveGear()) }).width(Length::Units(56));
                ratio_input = ratio_input.size(14);
                let mut r = Row::new().spacing(5).width(Length::Shrink).align_items(Alignment::Center);
                r = r.push(name_label);
                r = r.push(ratio_input);
                r = r.push(iced::widget::button(Image::new(Handle::from_memory(DELETE_IMAGE)).content_fit(ContentFit::Fill)).height(Length::Units(15)).width(Length::Units(15)).padding(1));
                col = col.push(r);
            }
            let add_ratio_button = iced::widget::button(
                text("Add Ratio").horizontal_alignment(Horizontal::Center).vertical_alignment(Vertical::Center).size(12),
            )   .width(Length::Units(75))
                .height(Length::Units(25))
                .on_press(EditMessage::GearUpdate(GearUpdateType::AddRatio(GearConfigIdentifier::CustomizedGears(0))));
            col = col.push(add_ratio_button);
            gearset_roe = gearset_roe.push(col);
        }
        layout.push(scrollable(gearset_roe).horizontal_scroll(Properties::default()))
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

