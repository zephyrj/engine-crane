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

use std::cell::RefCell;
use std::{error, fs, io};
use std::cmp::max;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Weak;

use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::Path;
use indexmap::IndexMap;
use crate::error::{Error, ErrorKind};

pub trait IniUpdater {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String>;
}

pub trait FromIni {
    fn load_from_ini(ini_data: &Ini) -> crate::error::Result<Self> where Self: Sized;
}

#[derive(Debug)]
pub struct FieldTypeError {
    section_name: String,
    field_name: String,
    expected_type: String
}

impl FieldTypeError {
    pub fn new(section_name: &str, field_name: &str, expected_type: &str) -> FieldTypeError {
        FieldTypeError {
            section_name: String::from(section_name),
            field_name: String::from(field_name),
            expected_type: String::from(expected_type)
        }
    }
}

impl Display for FieldTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expected {}-{} to be {} type",
               &self.section_name,
               &self.field_name,
               &self.expected_type)
    }
}

impl error::Error for FieldTypeError {}

impl From<FieldTypeError> for Error {
    fn from(err: FieldTypeError) -> Self {
        Error::new(ErrorKind::InvalidCar, err.to_string() )
    }
}

#[derive(Debug)]
pub struct MissingMandatoryProperty {
    pub section_name: String,
    pub property_name: String
}

impl MissingMandatoryProperty {
    pub fn new(section_name: &str, property_name: &str) -> MissingMandatoryProperty {
        MissingMandatoryProperty {
            section_name: String::from(section_name),
            property_name: String::from(property_name)
        }
    }
}

impl Display for MissingMandatoryProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{} is missing",
               &self.section_name,
               &self.property_name,)
    }
}

impl error::Error for MissingMandatoryProperty {}

impl From<MissingMandatoryProperty> for Error {
    fn from(err: MissingMandatoryProperty) -> Self {
        Error::new(ErrorKind::InvalidCar, err.to_string() )
    }
}

#[derive(Debug)]
pub struct MissingSection {
    pub section_name: String
}

impl MissingSection {
    pub fn new(section_name: String) -> MissingSection {
        MissingSection { section_name }
    }
}

impl Display for MissingSection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Section {} is missing",
               &self.section_name)
    }
}

impl error::Error for MissingSection {}

impl From<MissingSection> for Error {
    fn from(err: MissingSection) -> Self {
        Error::new(ErrorKind::IniParseError, err.to_string() )
    }
}

pub fn get_value_from_weak_ref<T: std::str::FromStr>(ini_data: &Weak<RefCell<Ini>>,
                                                     section: &str,
                                                     key: &str) -> Option<T> {
    let ini = ini_data.upgrade()?;
    let ini_ref = ini.borrow();
    get_value(ini_ref.deref(), section, key)
}

pub fn get_value<T: std::str::FromStr>(ini: &Ini,
                                       section: &str,
                                       key: &str) -> Option<T> {
    let item = ini.get_value(section, key)?;
    match item.parse::<T>() {
        Ok(val) => { Some(val) }
        Err(_) => { None }
    }
}

pub fn get_mandatory_property<T: std::str::FromStr>(ini_data: &Ini, section_name: &str, key: &str) -> Result<T, MissingMandatoryProperty> {
    let res: T = match get_value(ini_data, section_name, key) {
        Some(val) => val,
        None => { return Err(MissingMandatoryProperty::new(section_name, key)); }
    };
    Ok(res)
}

pub fn set_value<T: std::fmt::Display>(ini: &mut Ini,
                                       section: &str,
                                       key: &str,
                                       val: T) -> Option<String> {
    ini.set_value(section, key, val.to_string())
}

pub fn set_float(ini: &mut Ini, section: &str, key: &str, val: f64, precision: usize) -> Option<String> {
    ini.set_value(section,
                  key,
                  format!("{number:.prec$}", number=val, prec=precision))
}

pub fn validate_section_exists(ini: &Ini, section_name: &str) -> Result<(), MissingSection> {
    match ini.contains_section(section_name) {
        true => Ok(()),
        false => Err(MissingSection::new(section_name.to_string()))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Section {
    name: String,
    indentation: Option<String>,
    property_map: IndexMap<String, Property>,
    name_comment: Option<Comment>,
    comments: Vec<Comment>,
    ordering: IndexMap<String, LineType>
}

impl Section {
    pub fn new(name: String) -> Section {
        Section {
            name,
            indentation: None,
            property_map: IndexMap::new(),
            name_comment: None,
            comments: Vec::new(),
            ordering: IndexMap::new()
        }
    }

    pub fn from_line(line: &str, comment_symbols: &HashSet<char>) -> Result<Section, String> {
        return match line.find('[') {
            None => {
                Err(String::from("No opening '[' for section name found"))
            }
            Some(opening_bracket_pos) => {
                match line.find(']') {
                    None => {
                        return Err(String::from("No closing ']' for section name found"));
                    }
                    Some(closing_bracket_pos) => {
                        let name = String::from(&line[opening_bracket_pos + 1..closing_bracket_pos]);
                        let name_comment = Comment::from_line(
                            &line[closing_bracket_pos+1..], comment_symbols
                        );
                        let indentation = if opening_bracket_pos > 0 {
                            Some(String::from(&line[..opening_bracket_pos]))
                        } else {
                            None
                        };
                        Ok(Section {
                            name,
                            indentation,
                            property_map: IndexMap::new(),
                            name_comment,
                            comments: Vec::new(),
                            ordering: IndexMap::new()
                        })
                    }
                }
            }
        }
    }

    pub fn get_property(&self, property_key: &str) -> Option<&Property> {
        self.property_map.get(property_key)
    }

    pub fn get_property_mut(&mut self, property_key: &str) -> Option<&mut Property> {
        self.property_map.get_mut(property_key)
    }

    pub fn contains_property(&self, key: &str) -> bool {
        self.property_map.contains_key(key)
    }

    pub fn add_property(&mut self, property: Property) {
        self.ordering.insert(property.key.clone(), LineType::KeyValue);
        self.property_map.insert(property.key.clone(), property);
    }

    pub fn remove_propery(&mut self, key: &str) -> Option<Property> {
        self.ordering.shift_remove_entry(key);
        let (_, val) = self.property_map.shift_remove_entry(key)?;
        Some(val)
    }

    pub fn add_comment(&mut self, comment: Comment) {
        self.comments.push(comment);
        self.ordering.insert(format!("comment-{}", self.comments.len().to_string()),
                             LineType::Comment);
    }
}

impl ToString for Section {
    fn to_string(&self) -> String {
        let mut out = String::new();
        if !self.name.is_empty() {
            out += &format!("[{}]", &self.name);
            if let Some(comment) = &self.name_comment {
                out += &comment.to_string();
            }
            out += "\n";
        }
        let mut property_iter = self.property_map.values();
        let mut comment_iter = self.comments.iter();

        let kv_strings: Vec<String> = self.ordering.iter().filter_map(|(_, line_type)| {
            return match line_type {
                LineType::KeyValue => {
                    Some(property_iter.next().unwrap().to_string())
                }
                LineType::Comment => {
                    Some(comment_iter.next().unwrap().to_string())
                }
                LineType::Ignore | LineType::SectionName => {
                    None
                }
            }
        }).collect();
        out += &kv_strings.join("\n");
        out
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Property {
    key: String,
    value: String,
    indentation: Option<String>,
    comment: Option<Comment>
}

impl Property {
    pub fn from_line(line: &str, comment_symbols: &HashSet<char>) -> Result<Property, String> {
        return match line.find(char::is_alphanumeric) {
            None => { Err(String::from("Cannot find valid property name")) }
            Some(key_start_pos) => {
                let mut indentation = None;
                if key_start_pos > 0 {
                    indentation = Some(String::from(&line[..key_start_pos]));
                }

                match line.find("=") {
                    None => { Err(String::from("Cannot find valid property value")) }
                    Some(delimiter_pos) => {
                        let key = String::from(&line[key_start_pos..delimiter_pos]);
                        let mut value = String::new();
                        let mut comment = None;
                        match line.find(|c: char| comment_symbols.contains(&c)) {
                            None => {
                                value += &line[delimiter_pos+1..].trim();
                            }
                            Some(comment_start_pos) => {
                                value += &line[delimiter_pos+1..comment_start_pos].trim();
                                comment = Comment::from_line(
                                    &line[delimiter_pos+1+value.len()..],
                                    comment_symbols
                                );
                            }
                        }
                        Ok(Property{ key, value, indentation, comment })
                    }
                }
            }
        }
    }

    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    pub fn set_value(&mut self, val: String) -> String {
        std::mem::replace(&mut self.value, val)
    }

    pub fn has_comment(&self) -> bool {
        self.comment.is_some()
    }

    pub fn add_comment(&mut self, comment: Comment) -> Option<Comment> {
        self.comment.replace(comment)
    }
}

impl ToString for Property {
    fn to_string(&self) -> String {
        let mut out = String::new();
        if let Some(indentation) = &self.indentation {
            out += indentation;
        }
        out += &self.key;
        out += "=";
        out += &self.value;
        if let Some(comment) = &self.comment {
            out += &comment.to_string();
        }
        out
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Comment {
    symbol: String,
    value: String,
    indentation: Option<String>
}

impl Comment {
    pub fn new(value: String, indentation: Option<String>) -> Comment{
        return Comment {
            symbol: "#".to_string(), value, indentation
        }
    }

    pub fn from_line(line: &str, comment_symbols: &HashSet<char>) -> Option<Comment> {
        return match line.match_indices(|c: char| comment_symbols.contains(&c)).next() {
            None => None,
            Some((idx, matched_char)) => {
                let mut indentation = None;
                if idx > 0 { indentation = Some(String::from(&line[..idx])); }
                Some(Comment{
                    symbol: String::from(matched_char),
                    value: String::from(&line[idx+1..]),
                    indentation
                })
            }
        }
    }
}

impl ToString for Comment {
    fn to_string(&self) -> String {
        let mut out = String::new();
        if let Some(indentation) = &self.indentation {
            out += indentation;
        }
        out += &self.symbol;
        out += &self.value;
        out
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub (crate) enum LineType {
    SectionName,
    KeyValue,
    Comment,
    Ignore
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Ini {
    sections: IndexMap<String, Section>,
    comment_symbols: HashSet<char>,
}

impl Ini {
    const TOP_LEVEL: &'static str = "topLevel";

    pub fn new() -> Ini {
        Ini {
            sections: IndexMap::new(),
            comment_symbols: HashSet::from([';', '#'])
        }
    }

    pub fn load_from_string(ini_data: String) -> Ini {
        let mut ini = Ini::new();
        ini.parse(ini_data);
        ini
    }

    pub fn load_from_file(path: &Path) -> io::Result<Ini> {
        Ok(Ini::load_from_string(fs::read_to_string(path)?))
    }

    pub fn parse(&mut self, input: String) {
        let mut current_section= Section::new(String::from(""));
        for (_num, line) in input.lines().enumerate() {
            match self.get_expected_line_type(line) {
                LineType::SectionName => {
                    self.finish_section(current_section);
                    current_section = Section::from_line(line, &self.comment_symbols).unwrap();
                }
                LineType::KeyValue => {
                    current_section.add_property(
                        Property::from_line(line, &self.comment_symbols).unwrap()
                    );
                }
                LineType::Comment => {
                    current_section.add_comment(
                        Comment::from_line(line, &self.comment_symbols).unwrap()
                    );
                }
                LineType::Ignore => {}
            }
        }
        self.finish_section(current_section)
    }

    pub fn extract<T: FromIni>(&self) -> crate::error::Result<T> {
        T::load_from_ini(self)
    }

    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_string())
    }
    
    pub fn write_to_buf(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.write_all(&self.to_bytes())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }

    pub fn get_value(&self, section_name: &str, property_name: &str) -> Option<String> {
        Some(self.sections.get(section_name)?.get_property(property_name)?.get_value())
    }

    pub fn get_mut_section(&mut self, name: &str) -> Option<&mut Section> {
        if !self.sections.contains_key(name) {
            return None;
        }
        Some(self.sections.get_mut(name).unwrap())
    }

    /// Remove a property from a given section.
    /// Returns the previous value of the property as `Some(value)` where value is a `String` or
    /// `None` if this operation didn't remove anything
    pub fn remove_value(&mut self, section_name: &str, property_name: &str) -> Option<String> {
        if !self.sections.contains_key(section_name) {
            return None;
        }
        let section = self.sections.get_mut(section_name).unwrap();
        if let Some(old_prop) = section.remove_propery(property_name) {
            return Some(old_prop.value)
        }
        None
    }

    pub fn remove_section(&mut self, section_name: &str) {
        if self.sections.contains_key(section_name) {
            self.sections.shift_remove(section_name);
        }
    }

    /// Set a ini property value to the provided String.
    /// Returns the previous value of the property as `Some(value)` where value is a `String` or
    /// `None` if this operation added a new property
    pub fn set_value(&mut self,
                     section_name: &str,
                     property_key: &str,
                     property_value: String) -> Option<String> {
        if !self.sections.contains_key(section_name) {
            self.sections.insert(String::from(section_name),
                                 Section::new(String::from(section_name)));
        }
        let section = self.sections.get_mut(section_name).unwrap();
        if !section.contains_property(property_key) {
            section.add_property(Property {
                key: String::from(property_key),
                value: property_value,
                indentation: section.indentation.clone(),
                comment: None
            });
            None
        } else {
            Some(section.get_property_mut(property_key).unwrap().set_value(property_value))
        }
    }
    
    pub fn get_section_names_starting_with(&self, key_prefix: &str) -> Vec<&str>{
        self.sections.keys().filter_map(|key| {
            if key.starts_with(key_prefix) {
                return Some(&key[..])
            }
            None
        }).collect()
    }

    pub fn get_max_idx_for_section_with_prefix(&self, section_prefix: &str) -> Option<usize> {
        let mut current_max  = None;
        for name in self.get_section_names_starting_with(section_prefix) {
            if let Some(idx) = section_name_to_idx(section_prefix, name) {
                current_max = match current_max {
                    None => Some(idx),
                    Some(current) => Some(max(current, idx))
                };
            }
        }
        current_max
    }

    pub fn get_section_names_with_prefix(&self, section_prefix: &str) -> BTreeMap<usize, &str> {
        let mut section_map = BTreeMap::new();
        for name in self.get_section_names_starting_with(section_prefix) {
            if let Some(idx) = section_name_to_idx(section_prefix, name) {
                section_map.insert(idx, name);
            }
        }
        section_map
    }

    pub fn contains_section(&self, name: &str) -> bool {
        self.sections.contains_key(name)
    }

    pub fn section_contains_property(&self, section_name: &str, property_name: &str) -> bool {
        match self.contains_section(section_name) {
            false => { false }
            true => {
                self.sections.get(section_name).unwrap().contains_property(property_name)
            }
        }
    }

    fn finish_section(&mut self, section: Section) {
        let key;
        if section.name.is_empty() {
            key = String::from(Ini::TOP_LEVEL);
        } else {
            key = section.name.clone();
        }
        self.sections.insert(key, section);
    }

    /// Essentially "what delimiting character comes first?"
    fn get_expected_line_type(&self, line: &str) -> LineType {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return LineType::Ignore;
        } else if trimmed.starts_with('=') {
            return LineType::Ignore;
        }

        let comment_opt = self.find_comment_start(trimmed);
        if let Some(section_start_pos) = self.find_section_start(trimmed) {
            if let Some(kv_delimiter_pos) = self.find_key_value_delimiter(trimmed) {
                if kv_delimiter_pos < section_start_pos {
                    if let Some(comment_start_index) = comment_opt {
                        if comment_start_index < kv_delimiter_pos {
                            return LineType::Comment
                        }
                    }
                    return LineType::KeyValue
                }
            }
            if let Some(comment_start_pos) = comment_opt {
                if comment_start_pos < section_start_pos {
                    return LineType::Comment
                }
            }
            return LineType::SectionName
        }
        if let Some(kv_delimiter_pos) = self.find_key_value_delimiter(trimmed) {
            if let Some(comment_start_pos) = comment_opt {
                if comment_start_pos < kv_delimiter_pos {
                    return LineType::Comment
                }
            }
            return LineType::KeyValue
        }
        return match comment_opt {
            None => { LineType::Ignore }
            Some(_) => { LineType::Comment }
        };
    }

    fn find_comment_start(&self, line: &str) -> Option<usize> {
        line.find(|c: char| self.comment_symbols.contains(&c))
    }

    fn find_section_start(&self, line: &str) -> Option<usize> {
        line.find('[')
    }

    fn find_key_value_delimiter(&self, line: &str) -> Option<usize> {
        match line.find("=") {
            None => None,
            Some(idx) => {
                if idx == 0 {
                    // Can't have a empty Key so we didn't match
                    None
                } else {
                    Some(idx)
                }
            }
        }
    }
}

impl Display for Ini {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        let section_strings: Vec<String> = self.sections.values().filter_map(|section| {
            let section_string = section.to_string();
            if !section_string.is_empty() {
                Some(section_string)
            } else {
                None
            }
        }).collect();
        out += &section_strings.join("\n\n");
        out += "\n";
        write!(f, "{}", out)
    }
}

fn section_name_to_idx(section_prefix: &str, name: &str) -> Option<usize> {
    match name.strip_prefix(section_prefix) {
        None => None,
        Some(remaining) => {
            let digits: String = remaining.chars().filter(|c| c.is_digit(10)).collect();
            if digits.is_empty() {
                Some(0)
            } else {
                match digits.parse::<usize>() {
                    Ok(val) => Some(val),
                    Err(_) => None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ini_utils::section_name_to_idx;

    #[test]
    fn section_name_idx_extraction() {
        assert_eq!(section_name_to_idx("FRONT","FRONT"), Some(0));
        assert_eq!(section_name_to_idx("REAR","REAR"), Some(0));
        assert_eq!(section_name_to_idx("GEAR_","GEAR_0"), Some(0));
        assert_eq!(section_name_to_idx("REAR","REAR_1"), Some(1));
        assert_eq!(section_name_to_idx("REAR","REAR_5"), Some(5));
        assert_eq!(section_name_to_idx("REAR","REAR_9"), Some(9));
        assert_eq!(section_name_to_idx("REAR","REAR_10"), Some(10));
        assert_eq!(section_name_to_idx("REAR","REAR_11"), Some(11));
        assert_eq!(section_name_to_idx("REAR","REAR_99"), Some(99));
        assert_eq!(section_name_to_idx("REAR","REAR_100"), Some(100));
        assert_eq!(section_name_to_idx("REAR","REAR_101"), Some(101));
        assert_eq!(section_name_to_idx("REAR","THERMAL_REAR"), None);
        assert_eq!(section_name_to_idx("REAR","THERMAL_REAR_1"), None);
    }
}