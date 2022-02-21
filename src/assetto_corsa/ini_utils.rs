use std::cell::RefCell;
use std::{error, fs, io};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Weak;

use std::collections::HashSet;
use std::path::Path;
use indexmap::IndexMap;
use crate::assetto_corsa::error::{Error, ErrorKind};
//use std::collections::HashMap as IndexMap;

pub trait IniUpdater {
    fn update_ini(&self, ini_data: &mut Ini) -> std::result::Result<(), String>;
}

pub trait FromIni {
    fn load_from_ini(ini_data: &Ini) -> crate::assetto_corsa::error::Result<Self> where Self: Sized;
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
    section_name: String,
    property_name: String
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

pub fn get_mandatory_property<T: std::str::FromStr>(ini_data: &Ini, section_name: &str, key: &str) -> std::result::Result<T, MissingMandatoryProperty> {
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

#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct Section {
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
                        let mut name_comment = Comment::from_line(
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
struct Property {
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
struct Comment {
    symbol: String,
    value: String,
    indentation: Option<String>
}

impl Comment {
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
        for (num, line) in input.lines().enumerate() {
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

    pub fn extract<T: FromIni>(&self) -> crate::assetto_corsa::error::Result<T> {
        T::load_from_ini(self)
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_string())
    }

    pub fn get_value(&self, section_name: &str, property_name: &str) -> Option<String> {
        Some(self.sections.get(section_name)?.get_property(property_name)?.get_value())
    }

    pub fn remove_value(&mut self, section_name: &str, property_name: &str) -> Option<String> {
        if !self.sections.contains_key(section_name) {
            return None;
        }
        let section = self.sections.get_mut(section_name).unwrap();

        None
    }

    /// Set a ini property value to the provided String. You may also provide a comment to set
    /// If the section or property don't exist prior to this operation then they will be
    /// created.
    ///
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

    pub fn contains_section(&self, name: &str) -> bool {
        self.sections.contains_key(name)
    }

    fn finish_section(&mut self, section: Section) {
        let mut key = String::new();
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
        line.find("=")
    }
}

impl ToString for Ini {
    fn to_string(&self) -> String {
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
        out
    }
}
