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

use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::{Display, Formatter};
use std::mem;
use utils::numeric::round_float_to;


#[derive(Copy, Clone, Debug)]
enum TypeIdentifier {
    BlobMark = 1,
    False = 48,
    True = 49,
    Number = 78,
    Text = 83,
    Section = 84
}

impl TypeIdentifier {
    fn from_u8(byte: u8) -> Option<TypeIdentifier> {
        match byte {
            1 => Some(TypeIdentifier::BlobMark),
            48 => Some(TypeIdentifier::False),
            49 => Some(TypeIdentifier::True),
            78 => Some(TypeIdentifier::Number),
            83 => Some(TypeIdentifier::Text),
            84 => Some(TypeIdentifier::Section),
            _ => None
        }
    }
}
#[allow(dead_code)]
#[derive(Debug)]
struct ByteChunk<'a> {
    byte_stream: &'a[u8],
}

impl<'a> ByteChunk<'a> {
    #[allow(dead_code)]
    pub fn hex(&self) {
        format!("{:x?}", self.byte_stream);
    }
}

#[derive(Debug)]
pub enum AttributeValue {
    Blob(Vec<u8>),
    Text(String),
    Number(f64),
    False,
    True
}

impl AttributeValue {
    pub fn as_str(&self) -> &str {
        return match self {
            AttributeValue::Blob(_) => { "BLOB" }
            AttributeValue::Text(t) => { t.as_str() }
            AttributeValue::Number(_num) => { "" }
            AttributeValue::False => { "false" }
            AttributeValue::True => { "true" }
        }
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        return match self {
            AttributeValue::Blob(blob) => { blob.clone() }
            AttributeValue::Text(t) => { Vec::from(t.as_bytes()) }
            AttributeValue::Number(num) => {
                Vec::from(round_float_to(*num, 10).to_string().as_bytes())
            }
            AttributeValue::False => { Vec::from("false".as_bytes()) }
            AttributeValue::True => { Vec::from("true".as_bytes()) }
        }
    }
    
    pub fn as_num(&self) -> Result<f64, String> {
        return match self {
            AttributeValue::Number(num) => { Ok(*num) },
            _ => { Err(String::from("Not a number")) }
        }
    }
}

impl Display for AttributeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AttributeValue::Blob(_) => { String::from("BLOB") }
            AttributeValue::Text(t) => { t.clone() }
            AttributeValue::Number(num) => { num.to_string() }
            AttributeValue::False => { String::from("false") }
            AttributeValue::True => { String::from("true") }
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug)]
pub struct Attribute {
    pub name: String,
    pub value: AttributeValue
}

impl Display for Attribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name.as_str(), self.value)
    }
}

#[derive(Debug)]
pub struct Section {
    name: String,
    section_type: u32,
    num_children: usize,
    attributes: HashMap<String, Attribute>,
    sections: HashMap<String, Section>,
    stack: Vec<String>
}

impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]\n", self.name.as_str())?;
        for (_, attr) in &self.attributes {
            write!(f, "{}\n", attr)?;
        }
        for (_, section) in &self.sections {
            write!(f, "  {}\n", section)?;
        }
        Ok(())
    }
}

impl Section {
    pub fn is_complete(&self) -> bool {
        if self.stack.is_empty() {
            return self.num_children == (self.attributes.len() + self.sections.len())
        }
        return false;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_section(&self, section_name: &str) -> Option<&Section> {
        self.sections.get(section_name)
    }

    pub fn section_keys(&self) -> Keys<'_, String, Section> {
        self.sections.keys()
    }

    pub fn get_attribute(&self, attribute_name: &str) -> Option<&Attribute> {
        self.attributes.get(attribute_name)
    }

    pub fn attribute_keys(&self) -> Keys<'_, String, Attribute> {
        self.attributes.keys()
    }

    fn finalise_sections_if_complete(&mut self) {
        while !self.stack.is_empty() && self.get_current_section().is_complete() {
            self.stack.pop();
        }
    }

    fn add_attribute(&mut self, attr: Attribute) {
        if self.stack.is_empty() {
            self.attributes.insert(attr.name.clone(), attr);
        } else {
            self.get_current_section().add_attribute(attr);
        }
        self.finalise_sections_if_complete();
    }

    fn add_section(&mut self, section: Section) {
        if self.stack.is_empty() {
            let section_name = section.name.clone();
            self.sections.insert(section.name.clone(), section);
            self.stack.push(section_name);
        } else {
            self.get_current_section().add_section(section);
        }
        self.finalise_sections_if_complete();
    }

    fn get_current_section(&mut self) -> &mut Section {
        self.sections.get_mut(self.stack.last().unwrap()).unwrap()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Blob {
    chunk: Vec<u8>,
    name: String,
    sections: Vec<Section>,
    stack: Vec<usize>
}

#[derive(Default, Debug)]
pub struct CarFile {
    byte_stream: Vec<u8>,
    current_pos: usize,
    attributes: HashMap<String, Attribute>,
    sections: HashMap<String, Section>,
    stack: Vec<String>
}

impl Display for CarFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (_, attr) in &self.attributes {
            write!(f, "{}\n", attr)?;
        }
        for (_, section) in &self.sections {
            write!(f, "{}\n", section)?;
        }
        Ok(())
    }
}

impl CarFile {
    pub fn from_bytes(byte_stream: Vec<u8>) -> Result<CarFile, String> {
        let mut c = CarFile {
            byte_stream,
            current_pos: 0,
            attributes: HashMap::new(),
            sections: HashMap::new(),
            stack: Vec::new()
        };
        c.parse_opening_blob_mark()?;
        c.parse()?;
        Ok(c)
    }

    pub fn get_section(&self, section_name: &str) -> Option<&Section> {
        self.sections.get(section_name)
    }

    pub fn section_keys(&self) -> Keys<'_, String, Section> {
        self.sections.keys()
    }

    fn parse_opening_blob_mark(&mut self) -> Result<(), String> {
        if self.byte_stream[self.current_pos] != TypeIdentifier::BlobMark as u8 {
            return Err(String::from("Stream doesn't open with expected blob mark - is it valid .car data?"))
        }
        self.increment_position(2);
        let section = self.parse_section(String::from("Car"));
        self.add_section(section);
        Ok(())
    }

    fn parse(&mut self) -> Result<(), String> {
        let mut parsing_attribute = false;
        let mut parsing_int_pair = false;
        loop {
            if self.current_pos >= self.byte_stream.len() {
                break;
            }

            if parsing_attribute || parsing_int_pair {
                let attribute_name :String;
                if !parsing_int_pair {
                    let len: usize = self.parse_length();
                    if self.peek_is_blob() {
                        self.increment_position(len);
                        continue
                    }
                    attribute_name = self.parse_text(len);
                } else {
                    attribute_name = self.parse_number().to_string();
                }

                match TypeIdentifier::from_u8(self.parse_byte()) {
                    Some(type_id) => {
                        match type_id {
                            TypeIdentifier::False => {
                                self.add_attribute(Attribute{ name: attribute_name,
                                                                   value: AttributeValue::False });
                            }
                            TypeIdentifier::True => {
                                self.add_attribute(Attribute{ name: attribute_name,
                                                                   value: AttributeValue::True });
                            }
                            TypeIdentifier::Number => {
                                let attr = Attribute{ name: attribute_name, value: AttributeValue::Number(self.parse_number()) };
                                self.add_attribute(attr);
                            }
                            TypeIdentifier::Section => {
                                let section = self.parse_section(attribute_name);
                                self.add_section(section);
                            }
                            TypeIdentifier::Text | TypeIdentifier::BlobMark => {
                                let len = self.parse_length();
                                if self.peek_is_blob() {
                                    let attr = Attribute { name: attribute_name,
                                        value: AttributeValue::Blob(self.parse_blob(len))
                                    };
                                    self.add_attribute(attr);
                                } else {
                                    let attr = Attribute { name: attribute_name,
                                        value: AttributeValue::Text(self.parse_text(len)) };
                                    self.add_attribute(attr);
                                }
                            }
                        }
                    },
                    None => {
                        return Err(String::from("Unexpected type found in stream"));
                    }
                }
                self.finalise_sections_if_complete();
                parsing_attribute = false;
                parsing_int_pair = false;
            }
            else {
                match TypeIdentifier::from_u8(self.parse_byte()) {
                    Some(type_id) => {
                        match type_id {
                            TypeIdentifier::Text => {
                                parsing_attribute = true;
                            },
                            TypeIdentifier::Number => {
                                parsing_int_pair = true;
                            }
                            _ => {}
                        }
                    }
                    None => {}
                }
            }
        }
        Ok(())
    }

    fn parse_section(&mut self, name: String) -> Section {
        let section_type_len = mem::size_of::<u32>();
        let section_type = u32::from_le_bytes(self.peek_byte_slice(section_type_len).try_into().expect("Failed to determine num_children"));
        self.increment_position(section_type_len);

        let num_children_len  = mem::size_of::<u32>();
        let num_children = u32::from_le_bytes(self.peek_byte_slice(num_children_len).try_into().expect("Failed to determine num_children")) as usize;
        self.increment_position(num_children_len);

        Section {
            name,
            section_type,
            num_children,
            attributes: HashMap::new(),
            sections: HashMap::new(),
            stack: Vec::new()
        }
    }

    fn parse_byte(&mut self) -> u8 {
        let len = mem::size_of::<u8>();
        let t= u8::from_le_bytes(self.peek_byte_slice(len).try_into().expect("Cannot parse byte"));
        self.increment_position(len);
        return t;
    }

    fn parse_length(&mut self) -> usize {
        let len = u32::from_le_bytes(self.peek_byte_slice(mem::size_of::<u32>()).try_into().expect("Failed to read length"));
        self.increment_position(4);
        return len as usize;
    }

    fn parse_text(&mut self, len: usize) -> String {
        let str = String::from_utf8(Vec::from(self.peek_byte_slice(len))).expect("Failed to parse UTF-8 text");
        self.increment_position(len);
        return str;
    }

    fn parse_blob(&mut self, len: usize) -> Vec<u8> {
        let vec = Vec::from(self.peek_byte_slice(len));
        self.increment_position(len);
        return vec;
    }

    fn parse_number(&mut self) -> f64 {
        let num = f64::from_le_bytes(self.peek_byte_slice(mem::size_of::<f64>()).try_into().expect("Failed to parse number"));
        self.increment_position(mem::size_of::<f64>());
        return num;
    }

    fn peek_is_blob(&self) -> bool {
        if self.byte_stream[self.current_pos] == TypeIdentifier::BlobMark as u8 {
            return true;
        }
        false
    }

    fn peek_byte_slice(&self, len: usize) -> &[u8] {
        &self.byte_stream[self.current_pos..(self.current_pos+len)]
    }

    fn finalise_sections_if_complete(&mut self) {
        while !self.stack.is_empty() && self.get_current_section().is_complete() {
            self.stack.pop();
        }
    }

    fn get_current_section(&mut self) -> &mut Section {
        self.sections.get_mut(self.stack.last().unwrap()).unwrap()
    }

    fn increment_position(&mut self, len: usize) {
        self.current_pos += len;
    }

    fn add_attribute(&mut self, attr: Attribute) {
        if self.stack.is_empty() {
            self.attributes.insert(attr.name.clone(), attr);
        } else {
            self.get_current_section().add_attribute(attr);
        }
    }

    fn add_section(&mut self, section: Section) {
        if self.stack.is_empty() {
            let section_name = section.name.clone();
            self.sections.insert(section.name.clone(), section);
            self.stack.push(section_name);
        } else {
            self.get_current_section().add_section(section);
        }
    }
}