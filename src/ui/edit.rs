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

use std::cmp::{max, Ordering};
use std::collections::{HashMap, BTreeMap, HashSet, BTreeSet};
use std::fmt::{Display, Formatter};
use super::{Message, Tab};
use std::path::{PathBuf};
use fraction::ToPrimitive;
use iced::{Alignment, Element, Length, Padding};
use iced::alignment::Vertical;
use iced::widget::{Column, Container, pick_list, Row, Text, radio, horizontal_rule, text_input, vertical_rule, text, row, TextInput};
use iced_aw::{TabLabel};
use tracing::{error, warn};
use zip::write;
use crate::assetto_corsa::Car;
use crate::assetto_corsa::car::data;
use crate::assetto_corsa::car::data::Drivetrain;
use crate::assetto_corsa::car::data::setup::gears::{GearConfig, GearData, GearSet, SingleGear};
use crate::assetto_corsa::car::data::setup::Setup;
use crate::assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};
use crate::automation::car::AttributeValue::{False, True};
use crate::ui::{ApplicationData, ListPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearConfigChoice {
    Fixed,
    GearSets,
    PerGearConfig
}

impl Display for GearConfigChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            GearConfigChoice::Fixed => { write!(f, "Fixed Gearing") }
            GearConfigChoice::GearSets => { write!(f, "Gear Sets") }
            GearConfigChoice::PerGearConfig => { write!(f, "Fully Customizable") }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalDriveChoice {
    Fixed,
    Multiple
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GearUpdateType {
    FixedGearUpdate(usize, String),
    CustomizedGearUpdate
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct GearLabel {
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

struct RatioEntry {
    pub name: String,
    pub ratio: f64
}

impl RatioEntry {
    pub fn new(name: String, ratio: f64) -> RatioEntry {
        RatioEntry {name, ratio}
    }

    pub fn total_cmp(&self, other: &RatioEntry) -> Ordering {
        self.ratio.total_cmp(&other.ratio)
    }
}

impl Eq for RatioEntry {}

impl PartialEq<Self> for RatioEntry {
    fn eq(&self, other: &Self) -> bool {
        self.ratio.eq(&other.ratio)
    }
}

impl PartialOrd<Self> for RatioEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.ratio.partial_cmp(&other.ratio)
    }
}

impl Ord for RatioEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_cmp(other)
    }
}

struct RatioSet {
    entries: BTreeSet<RatioEntry>,
    max_name_length: usize
}

impl RatioSet {
    pub fn new() -> RatioSet {
        RatioSet {
            entries: BTreeSet::new(),
            max_name_length: 0
        }
    }

    pub fn max_name_len(&self) -> usize {
        self.max_name_length
    }

    pub fn entries(&self) -> &BTreeSet<RatioEntry> {
        &self.entries
    }

    pub fn mut_entries(&mut self) -> &mut BTreeSet<RatioEntry> {
        &mut self.entries
    }

    pub fn insert(&mut self, new_entry: RatioEntry) -> bool {
        self.max_name_length = max(self.max_name_length , new_entry.name.len());
        self.entries.insert(new_entry)
    }

    pub fn remove(&mut self, entry: RatioEntry) -> bool {
        if self.entries.remove(&entry) {
            if entry.name.len() == self.max_name_length {
                self.max_name_length = 0;
                for entry in &self.entries {
                    self.max_name_length = max(self.max_name_length, entry.name.len());
                }
            }
            return true;
        }
        return false;
    }
}

impl FromIterator<RatioEntry> for RatioSet {
    fn from_iter<T: IntoIterator<Item=RatioEntry>>(iter: T) -> Self {
        let mut s = RatioSet::new();
        for entry in iter {
            s.insert(entry);
        }
        s
    }
}

fn gear_configuration_builder(ac_car_path: &PathBuf) -> Result<Box<dyn GearConfiguration>, String> {
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
            Ok(Box::new(FixedGears {
                current_drivetrain_data: drivetrain_data,
                updated_drivetrain_data: HashMap::new()
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
            Ok(Box::new(GearSets{
                current_drivetrain_data: drivetrain_data,
                current_setup_data,
                updated_drivetrain_data: HashMap::new()
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
            Ok(Box::new(CustomizableGears{
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

pub struct GearSets {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Vec<GearSet>,
    updated_drivetrain_data: HashMap<String, HashMap<usize, String>>
}

impl GearConfiguration for GearSets {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::GearSets
    }

    fn handle_update(&mut self, update_type: GearUpdateType) {
        todo!()
    }

    fn add_editable_gear_list<'a, 'b>(&'a self, layout: Column<'b, EditMessage>) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let mut gearset_roe = Row::new().width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
        let mut first = true;
        for gearset in self.current_setup_data.iter() {
            let mut col = Column::new().align_items(Alignment::Center).spacing(5).padding(Padding::from([0, 10]));
            let mut displayed_ratios = Vec::new();
            for (gear_idx, ratio) in gearset.ratios().iter().enumerate() {
                let current_val = match self.updated_drivetrain_data.get(gearset.name()) {
                    Some(gear_lookup) => {
                        match gear_lookup.get(&gear_idx)  {
                            Some(val) => {
                                val.clone()
                            }
                            None => "".to_string()
                        }
                    },
                    None => "".to_string()
                };
                displayed_ratios.push((ratio.to_string(), current_val));
            }
            col = col.push(text(gearset.name()));
            col = col.push(create_gear_ratio_column(displayed_ratios));
            gearset_roe = gearset_roe.push(col);
        }
        layout.push(gearset_roe)
    }
}

pub struct CustomizableGears {
    current_drivetrain_data: Vec<f64>,
    current_setup_data: Vec<SingleGear>,
    new_setup_data: BTreeMap<GearLabel, RatioSet>
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
            let mut col = Column::new().align_items(Alignment::Center).width(Length::Shrink).spacing(5).padding(Padding::from([0, 10]));
            col = col.push(text(gear_idx));
            let name_width = (ratio_set.max_name_length * 12).to_u16().unwrap_or(u16::MAX);
            for ratio_entry in ratio_set.entries() {
                let name_label = TextInput::new("",&ratio_entry.name, |e|{ EditMessage::GearUpdate(GearUpdateType::CustomizedGearUpdate) }).width(Length::Units(name_width));
                let ratio_input = TextInput::new("",&ratio_entry.ratio.to_string(), |e|{ EditMessage::GearUpdate(GearUpdateType::CustomizedGearUpdate) }).width(Length::Units(84));
                let mut r = Row::new().spacing(5).width(Length::Shrink);
                r = r.push(name_label);
                r = r.push(ratio_input);
                col = col.push(r);
            }
            gearset_roe = gearset_roe.push(col);
        }
        layout.push(gearset_roe)
    }
}

pub struct FixedGears {
    current_drivetrain_data: Vec<f64>,
    updated_drivetrain_data: HashMap<usize, String>
}

impl GearConfiguration for FixedGears {
    fn get_config_type(&self) -> GearConfigChoice {
        GearConfigChoice::Fixed
    }

    fn handle_update(&mut self, update_type: GearUpdateType) {
        match update_type {
            GearUpdateType::FixedGearUpdate(gear_idx, ratio) => {
                self.updated_drivetrain_data.insert(gear_idx, ratio);
            }
            GearUpdateType::CustomizedGearUpdate => {}
        }
    }

    fn add_editable_gear_list<'a, 'b>(
        &'a self,
        mut layout: Column<'b, EditMessage>
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {

        let mut displayed_ratios = Vec::new();
        for (gear_idx, ratio) in self.current_drivetrain_data.iter().enumerate() {
            let current_val = match self.updated_drivetrain_data.contains_key(&gear_idx) {
                true => self.updated_drivetrain_data[&gear_idx].to_string(),
                false => "".to_string()
            };
            displayed_ratios.push((ratio.to_string(), current_val));
        }
        layout.push(create_gear_ratio_column(displayed_ratios))
    }
}

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
            move |new_value| { EditMessage::GearUpdate(GearUpdateType::FixedGearUpdate(gear_idx, new_value))}
        ).width(Length::Units(84));
        gear_row = gear_row.push(l).push(t);
        gear_list = gear_list.push(gear_row);
    }
    gear_list
}

pub struct EditTab {
    status_message: String,
    current_car_path: Option<PathBuf>,
    gear_configuration: Option<Box<dyn GearConfiguration>>
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    CarSelected(ListPath),
    GearConfigSelected(GearConfigChoice),
    GearUpdate(GearUpdateType)
}

impl EditTab {
    pub(crate) fn new() -> Self {
        EditTab {
            status_message: String::new(),
            current_car_path: None,
            gear_configuration: None
        }
    }

    fn add_gearbox_config_selector_row<'a, 'b>(
        &'a self,
        layout: Column<'b, EditMessage>,
        selected_option: GearConfigChoice
    ) -> Column<'b, EditMessage>
        where 'b: 'a
    {
        let gear_config_row = [GearConfigChoice::Fixed, GearConfigChoice::GearSets, GearConfigChoice::PerGearConfig]
            .iter().fold(
            Row::new().padding(Padding::from([0, 10])).spacing(20).align_items(Alignment::End),
            |row, config_choice| {
                row.push(radio(
                    format!("{config_choice}"),
                    *config_choice,
                    Some(selected_option),
                    EditMessage::GearConfigSelected).spacing(3).size(20).text_size(18))
            });
        layout.push(horizontal_rule(5)).push(gear_config_row)
    }

    pub fn update(&mut self, message: EditMessage, app_data: &ApplicationData) {
        match message {
            EditMessage::CarSelected(path_ref) => {
                self.current_car_path = Some(path_ref.full_path.clone());
                match gear_configuration_builder(&path_ref.full_path) {
                    Ok(config) => { self.gear_configuration = Some(config) }
                    Err(e) => {
                        error!(e)
                    }
                }
            }
            EditMessage::GearConfigSelected(choice) => {
                // TODO convert between types
            }
            EditMessage::GearUpdate(updateType) => {
                if let Some(config) = &mut self.gear_configuration {
                    config.handle_update(updateType);
                }
            }
        }
    }

    pub fn app_data_update(&mut self, app_data: &ApplicationData) {
    }
}

impl Tab for EditTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Edit Car")
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
        let current_car = match &self.current_car_path {
            None => { None }
            Some(path) => {
                Some(ListPath {full_path: path.clone()})
            }
        };
        let car_select_container = Column::new()
            .align_items(Alignment::Start)
            //.padding(10)
            .push(Text::new("Assetto Corsa car"))
            .push(pick_list(
                &app_data.assetto_corsa_data.available_cars,
                current_car,
                EditMessage::CarSelected,
            ));
        let select_container = Row::new()
            //.align_items(Align::)
            .padding(Padding::from([0, 10]))
            .spacing(20)
            .push(car_select_container);

        let mut layout = Column::new().width(Length::Fill)
            .align_items(Alignment::Start)
            //.padding(10)
            .spacing(30)
            .push(select_container);
            //.push(horizontal_rule(3));

        if let Some(gear_config) = &self.gear_configuration {
            layout = self.add_gearbox_config_selector_row(layout, gear_config.get_config_type());
            layout = gear_config.add_editable_gear_list(layout);
        }
        let content : Element<'_, EditMessage> = Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into();
        content.map(Message::Edit)
    }
}
