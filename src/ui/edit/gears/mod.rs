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

pub mod gear_config;

pub use gear_config::{gear_configuration_builder, GearConfiguration};

use std::cmp::{max, Ordering};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use iced::widget::TextInput;
use itertools::Itertools;

use crate::assetto_corsa::car::data::setup::gears::SingleGear;


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
pub enum GearIdentifier {
    Fixed(usize),
    GearSet(usize, usize),
    CustomizedGears(GearLabel, usize),
    FinalDrive(usize)
}

#[derive(Debug, Clone, PartialEq)]
pub enum GearUpdateType {
    Fixed(FixedGearUpdate),
    Gearset(GearsetUpdate),
    CustomizedGear(CustomizedGearUpdate)
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum FixedGearUpdate {
    AddGear(),
    RemoveGear(),
    UpdateRatio(usize, String)
}

#[derive(Debug, Clone, PartialEq)]
pub enum GearsetUpdate {
    AddGear(),
    RemoveGear(),
    UpdateRatio(GearsetLabel, usize, String),
    DefaultGearsetSelected(GearsetLabel)
}

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

struct RatioEntry {
    pub idx: usize,
    pub name: String,
    pub ratio: f64
}

impl RatioEntry {
    fn new(idx: usize, name: String, ratio: f64) -> RatioEntry {
        RatioEntry {idx, name, ratio}
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
    entries: BTreeMap<usize, RatioEntry>,
    max_name_length: usize,
    next_idx: usize,
    default_idx: Option<usize>
}

impl RatioSet {
    pub fn new() -> RatioSet {
        RatioSet {
            entries: BTreeMap::new(),
            max_name_length: 0,
            next_idx: 0,
            default_idx: None
        }
    }

    pub fn max_name_len(&self) -> usize {
        self.max_name_length
    }

    pub fn entries(&self) -> Vec<&RatioEntry> {
        let mut v = Vec::from_iter(self.entries.values());
        v.sort();
        v
    }

    pub fn mut_entries(&mut self) -> Vec<&mut RatioEntry> {
        let mut v = Vec::from_iter(self.entries.values_mut());
        v.sort();
        v
    }

    pub fn insert(&mut self, ratio_name: String, ratio: f64) -> usize {
        self.max_name_length = max(self.max_name_length , ratio_name.len());
        let idx = self.next_idx;
        self.next_idx += 1;
        self.entries.insert(idx, RatioEntry::new(idx, ratio_name, ratio));
        idx
    }

    pub fn remove(&mut self, idx: usize) -> bool {
        return match self.entries.remove(&idx) {
            None => { false }
            Some(removed) => {
                if let Some(default_idx) = self.default_idx {
                    if idx == default_idx {
                        self.default_idx = None;
                    }
                }
                if removed.name.len() == self.max_name_length {
                    self.max_name_length = 0;
                    for entry in self.entries.values() {
                        self.max_name_length = max(self.max_name_length, entry.name.len());
                    }
                }
                true
            }
        }
    }

    pub fn remove_entry(&mut self, entry: &RatioEntry) -> bool {
        self.remove(entry.idx)
    }

    pub fn update_ratio_name(&mut self, idx: usize, new_name: String) {
        match self.entries.get_mut(&idx) {
            None => {}
            Some(entry) => { entry.name = new_name }
        }
    }

    pub fn update_ratio_value(&mut self, idx: usize, new_value: f64) {
        match self.entries.get_mut(&idx) {
            None => {}
            Some(entry) => { entry.ratio = new_value }
        }
    }

    pub fn default(&self) -> Option<usize> {
        self.default_idx
    }

    pub fn set_default(&mut self, idx: usize) -> Result<(), String> {
        if !self.entries.contains_key(&idx) {
            return Err(format!("Index {} doesn't exist", idx));
        }
        self.default_idx = Some(idx);
        Ok(())
    }


}

