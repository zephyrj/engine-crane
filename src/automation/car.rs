use std::ffi::OsString;
use std::default::Default;
use std::mem;


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

#[derive(Debug)]
struct ByteChunk<'a> {
    byte_stream: &'a[u8],
}

impl<'a> ByteChunk<'a> {
    pub fn hex(&self) {
        format!("{:x?}", self.byte_stream);
    }
}

#[derive(Debug)]
enum AttributeValue {
    Blob(Blob),
    Text(String),
    Number(f64),
    False,
    True
}

#[derive(Debug)]
struct Attribute {
    name: String,
    value: AttributeValue
}

#[derive(Debug)]
struct Section {
    name: String,
    num_children: usize,
    attributes: Vec<Attribute>,
    sections: Vec<Section>,
    stack: Vec<usize>
}

impl Section {
    pub fn is_complete(&self) -> bool {
        if self.stack.is_empty() {
            return self.num_children == (self.attributes.len() + self.sections.len())
        }
        return false;
    }

    fn finalise_sections_if_complete(&mut self) {
        while !self.stack.is_empty() && self.get_current_section().is_complete() {
            self.stack.pop();
        }
    }

    fn add_attribute(&mut self, attr: Attribute) {
        if self.stack.is_empty() {
            self.attributes.push(attr);
        } else {
            self.get_current_section().add_attribute(attr);
        }
        self.finalise_sections_if_complete();
    }

    fn add_section(&mut self, section: Section) {
        if self.stack.is_empty() {
            let section_index = self.sections.len();
            self.sections.push(section);
            self.stack.push(section_index);
        } else {
            self.get_current_section().add_section(section);
        }
        self.finalise_sections_if_complete();
    }

    fn get_current_section(&mut self) -> &mut Section {
        self.sections.get_mut(*self.stack.last().unwrap()).unwrap()
    }
}

#[derive(Debug)]
struct Blob {
    chunk: Vec<u8>,
    name: String,
    sections: Vec<Section>,
    stack: Vec<usize>
}

#[derive(Default, Debug)]
pub struct CarFile {
    byte_stream: Vec<u8>,
    current_pos: usize,
    attributes: Vec<Attribute>,
    sections: Vec<Section>,
    stack: Vec<usize>
}

impl CarFile {
    pub fn from_bytes(byte_stream: Vec<u8>) -> Result<CarFile, String> {
        let mut c = CarFile {
            byte_stream,
            current_pos: 0,
            attributes: Vec::new(),
            sections: Vec::new(),
            stack: Vec::new()
        };
        c.parse_opening_blob_mark()?;
        c.parse()?;
        Ok(c)
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
                let mut attribute_name = String::new();
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
                                let mut section = self.parse_section(attribute_name);
                                self.add_section(section);
                            }
                            TypeIdentifier::Text | TypeIdentifier::BlobMark => {
                                let len = self.parse_length();
                                if self.peek_is_blob() {
                                    self.increment_position(2);
                                    let mut section = self.parse_section(attribute_name);
                                    self.add_section(section);
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
        let section_byte_stream = self.peek_byte_slice(mem::size_of::<u64>());
        let mut num_children = 0;
        if u32::from_le_bytes(section_byte_stream[0..4].try_into().expect("Section header too small")) > 0 {
            num_children = u64::from_le_bytes(section_byte_stream[0..8].try_into().expect("Failed to determine num_children")) as usize;
        } else {
            num_children = u32::from_le_bytes(section_byte_stream[4..8].try_into().expect("Failed to determine num_children")) as usize;
        }
        self.increment_position(mem::size_of::<u64>());
        Section {
            name,
            num_children,
            attributes: Vec::new(),
            sections: Vec::new(),
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
        self.sections.get_mut(*self.stack.last().unwrap()).unwrap()
    }

    fn increment_position(&mut self, len: usize) {
        self.current_pos += len;
    }

    fn add_attribute(&mut self, attr: Attribute) {
        if self.stack.is_empty() {
            self.attributes.push(attr);
        } else {
            self.get_current_section().add_attribute(attr);
        }
    }

    fn add_section(&mut self, section: Section) {
        if self.stack.is_empty() {
            let section_idx = self.sections.len();
            self.sections.push(section);
            self.stack.push(section_idx);
        } else {
            self.get_current_section().add_section(section);
        }
    }
}