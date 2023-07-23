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
use std::collections::{BTreeSet};
use std::fmt::{Display, Formatter};

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
pub enum GearConfigIdentifier {
    Fixed(usize),
    GearSet(usize, usize),
    CustomizedGears(usize)
}

#[derive(Debug, Clone, PartialEq)]
pub enum GearUpdateType {
    Add(),
    Remove(GearConfigIdentifier),
    Update(GearConfigIdentifier, String)
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

