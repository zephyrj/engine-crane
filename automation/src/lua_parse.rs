use std::collections::HashMap;
use std::fmt::{Display, Pointer};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Value {
    String(Vec<u8>),     
    Number(f64),
    Boolean(bool),
    Map(HashMap<String, Value>),
    Array(Vec<Value>),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", String::from_utf8_lossy(s)),
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Map(m) => {
                for (k, v) in m {
                    write!(f, "\"{}\": {}", k, v)?;
                }
                Ok(())
            },
            Value::Array(a) => write!(f, "{:?}", a),
            Value::Nil => write!(f, "nil"),
        }
    }
}

pub struct Parser {
    input: Vec<char>,
    pos: usize
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Parser {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let contents = String::from_utf8(bytes).unwrap_or_else(|e| {
            String::from_utf8_lossy(e.as_bytes()).into_owned()
        });

        Ok(Parser {
            input: contents.chars().collect(),
            pos: 0
        })
    }

    pub fn parse(&mut self) -> Result<Value, String> {
        // Find "do local _= " and position just before the opening brace
        self.skip_until_map_start()?;
        self.skip_whitespace();

        if self.pos >= self.input.len() || self.input[self.pos] != '{' {
            return Err("Expected '{' after 'do local _= '".to_string());
        }
        let result = self.parse_map()?;

        // Skip until "return _ and end"
        self.skip_until("return")?;
        self.skip_whitespace();
        self.skip_until("end")?;

        Ok(result)
    }

    fn skip_until_map_start(&mut self) -> Result<(), String> {
        let pattern = "do local _=";
        let mut pattern_pos = 0;

        while self.pos < self.input.len() && pattern_pos < pattern.len() {
            if self.input[self.pos] == pattern.chars().nth(pattern_pos).unwrap() {
                pattern_pos += 1;
            } else {
                pattern_pos = 0;
            }
            self.pos += 1;
        }

        if pattern_pos == pattern.len() {
            Ok(())
        } else {
            Err("Could not find start of map definition".to_string())
        }
    }

    fn skip_until(&mut self, target: &str) -> Result<(), String> {
        let chars: Vec<char> = target.chars().collect();
        let mut match_pos = 0;

        while self.pos < self.input.len() {
            if self.input[self.pos] == chars[match_pos] {
                match_pos += 1;
                if match_pos == chars.len() {
                    self.pos += 1; // Move past the last character
                    return Ok(());
                }
            } else {
                match_pos = 0;
            }
            self.pos += 1;
        }

        Err(format!("Could not find '{}'", target))
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Err("Unexpected end of input".to_string());
        }

        match self.input[self.pos] {
            '{' => {
                self.pos += 1; // Consume the opening brace
                self.parse_container()
            },
            '"' => self.parse_string(),
            't' if self.check_keyword("true") => {
                self.pos += 4;
                Ok(Value::Boolean(true))
            },
            'f' if self.check_keyword("false") => {
                self.pos += 5;
                Ok(Value::Boolean(false))
            },
            'n' if self.check_keyword("nil") => {
                self.pos += 3;
                Ok(Value::Nil)
            },
            c if c.is_digit(10) || c == '-' || c == '+' || c == '.' => self.parse_number(),
            _ => Err(format!("Unexpected character '{}' at position {}", self.input[self.pos], self.pos)),
        }
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        if self.pos + keyword.len() > self.input.len() {
            return false;
        }

        let slice: String = self.input[self.pos..self.pos + keyword.len()].iter().collect();
        slice == keyword
    }

    fn parse_string(&mut self) -> Result<Value, String> {
        self.pos += 1; // Skip the opening quote
        let mut result = Vec::new();
        let mut escaped = false;

        // First pass: determine the string boundaries and handle escapes
        let string_start = self.pos;
        let mut string_end = self.pos;

        while self.pos < self.input.len() {
            let c = self.input[self.pos];

            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                string_end = self.pos;
                self.pos += 1; // Skip the closing quote
                break;
            }

            self.pos += 1;
        }
        
        // empty string
        if (string_end == string_start) && !escaped {
            self.pos = string_end + 1;
            return Ok(Value::String(result));
        }

        if escaped || string_end == string_start {
            return Err("Unterminated string".to_string());
        }
        
        self.pos = string_start; // Reset position to start of string content

        while self.pos < string_end {
            // Handle escape sequences
            if self.input[self.pos] == '\\' && self.pos + 1 < string_end {
                self.pos += 1;
                match self.input[self.pos] {
                    'n' => result.push(b'\n'),
                    'r' => result.push(b'\r'),
                    't' => result.push(b'\t'),
                    'b' => result.push(b'\x08'),
                    'f' => result.push(b'\x0C'),
                    '\\' => result.push(b'\\'),
                    '"' => result.push(b'"'),
                    _ => {
                        let c = self.input[self.pos];
                        // Handle multi-byte UTF-8 characters
                        let mut buf = [0; 4];
                        let s = c.encode_utf8(&mut buf);
                        result.extend_from_slice(s.as_bytes());
                    },
                }
            } else {
                // Regular character
                let c = self.input[self.pos];
                // Handle multi-byte UTF-8 characters
                let mut buf = [0; 4];
                let s = c.encode_utf8(&mut buf);
                result.extend_from_slice(s.as_bytes());
            }
            self.pos += 1;
        }

        self.pos = string_end + 1; // Position after closing quote
        Ok(Value::String(result))
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;

        // Skip the sign if present
        if self.input[self.pos] == '-' || self.input[self.pos] == '+' {
            self.pos += 1;
        }

        // Skip digits before decimal point
        while self.pos < self.input.len() && self.input[self.pos].is_digit(10) {
            self.pos += 1;
        }

        // Skip decimal point and digits after it
        if self.pos < self.input.len() && self.input[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.input.len() && self.input[self.pos].is_digit(10) {
                self.pos += 1;
            }
        }

        // Handle scientific notation
        if self.pos < self.input.len() && (self.input[self.pos] == 'e' || self.input[self.pos] == 'E') {
            self.pos += 1;

            // Skip sign of exponent if present
            if self.pos < self.input.len() && (self.input[self.pos] == '-' || self.input[self.pos] == '+') {
                self.pos += 1;
            }

            // Skip exponent digits
            while self.pos < self.input.len() && self.input[self.pos].is_digit(10) {
                self.pos += 1;
            }
        }

        let num_str: String = self.input[start..self.pos].iter().collect();
        match num_str.parse::<f64>() {
            Ok(num) => Ok(Value::Number(num)),
            Err(_) => Err(format!("Invalid number: {}", num_str)),
        }
    }

    fn parse_container(&mut self) -> Result<Value, String> {
        self.skip_whitespace();

        // Empty container
        if self.pos < self.input.len() && self.input[self.pos] == '}' {
            self.pos += 1;
            return Ok(Value::Map(HashMap::new()));
        }

        // Check if it's an array or a map by looking ahead
        let mut temp_pos = self.pos;
        let mut is_array = true;
        let mut depth = 0;

        while temp_pos < self.input.len() {
            if self.input[temp_pos] == '{' {
                depth += 1;
            } else if self.input[temp_pos] == '}' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if self.input[temp_pos] == '=' && depth == 0 {
                is_array = false;
                break;
            } else if self.input[temp_pos] == ',' && depth == 0 {
                // If we hit a comma before an equals sign, it's likely an array
                break;
            }
            temp_pos += 1;
        }

        if is_array {
            self.parse_array()
        } else {
            self.parse_map_content()
        }
    }

    fn parse_map(&mut self) -> Result<Value, String> {
        self.pos += 1; // Consume the opening brace
        let result = self.parse_map_content()?;
        Ok(result)
    }

    fn parse_map_content(&mut self) -> Result<Value, String> {
        let mut result = HashMap::new();

        loop {
            self.skip_whitespace();

            // Check for end of map
            if self.pos < self.input.len() && self.input[self.pos] == '}' {
                self.pos += 1;
                break;
            }

            // Parse key
            let key = if self.pos < self.input.len() && self.input[self.pos] == '"' {
                // Quoted key
                match self.parse_string()? {
                    Value::String(bytes) => {
                        String::from_utf8(bytes).unwrap_or_else(|e| {
                            // Use lossy UTF-8 conversion for keys
                            String::from_utf8_lossy(e.as_bytes()).into_owned()
                        })
                    },
                    _ => unreachable!(),
                }
            } else {
                // Identifier key
                let start = self.pos;
                while self.pos < self.input.len() &&
                    (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
                    self.pos += 1;
                }

                if start == self.pos {
                    return Err(format!("Expected map key at position {}", self.pos));
                }

                self.input[start..self.pos].iter().collect()
            };

            self.skip_whitespace();
            if self.pos >= self.input.len() || self.input[self.pos] != '=' {
                return Err(format!("Expected '=' after map key '{}' at position {}", key, self.pos));
            }
            self.pos += 1;

            let value = self.parse_value()?;
            result.insert(key, value);

            // Skip comma or end of map
            self.skip_whitespace();
            if self.pos < self.input.len() && self.input[self.pos] == ',' {
                self.pos += 1;
            } else if self.pos < self.input.len() && self.input[self.pos] != '}' {
                // If not a comma and not closing brace, that's an error
                return Err(format!("Expected ',' or '}}' after map value at position {}", self.pos));
            }
        }

        Ok(Value::Map(result))
    }

    // Parse an array (assuming opening brace is already consumed)
    fn parse_array(&mut self) -> Result<Value, String> {
        let mut result = Vec::new();

        loop {
            self.skip_whitespace();

            // Check for end of array
            if self.pos < self.input.len() && self.input[self.pos] == '}' {
                self.pos += 1;
                break;
            }

            let value = self.parse_value()?;
            result.push(value);

            // Skip comma or end of array
            self.skip_whitespace();
            if self.pos < self.input.len() && self.input[self.pos] == ',' {
                self.pos += 1;
            } else if self.pos < self.input.len() && self.input[self.pos] != '}' {
                // If not a comma and not closing brace, that's an error
                return Err(format!("Expected ',' or '}}' after array value at position {}", self.pos));
            }
        }

        Ok(Value::Array(result))
    }

    // Skip whitespace and comments
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                ' ' | '\t' | '\r' | '\n' => self.pos += 1,
                '-' if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '-' => {
                    // Skip line comment
                    self.pos += 2;
                    while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                        self.pos += 1;
                    }
                },
                _ => break,
            }
        }
    }
}

impl Value {
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&Vec<u8>> {
        match self {
            Value::String(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(bytes) => String::from_utf8(bytes.clone()).ok(),
            _ => None,
        }
    }
    
    pub fn as_string_lossy(&self) -> Option<String> {
        match self {
            Value::String(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
            _ => None,
        }
    }
}

pub fn parse_lua_like_file(path: &str) -> Result<Value, String> {
    let mut parser = match Parser::from_file(path) {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to read file: {}", e)),
    };

    parser.parse()
}

pub fn parse_lua_like_string(input: &str) -> Result<Value, String> {
    let mut parser = Parser::new(input);
    parser.parse()
}

pub fn parse_lua_like_bytes(bytes: Vec<u8>) -> Result<Value, String> {
    match String::from_utf8(bytes.clone()) {
        Ok(s) => {
            let mut parser = Parser::new(&s);
            parser.parse()
        },
        Err(_) => {
            let s = String::from_utf8_lossy(&bytes);
            let mut parser = Parser::new(&s);
            parser.parse()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_map() {
        let input = r#"do local _= {
            key="string",
            num=42.5,
            bool=true
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();

        assert_eq!(map.get("key").unwrap().as_string().unwrap(), "string");
        assert_eq!(map.get("num").unwrap().as_number().unwrap(), 42.5);
        assert_eq!(map.get("bool").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_string() {
        let input = r#"do local _= {         
            first="string",
            empty_string="",
            escape_string="hello\"dave\""
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();

        assert_eq!(map.get("first").unwrap().as_string().unwrap(), "string");
        assert_eq!(map.get("empty_string").unwrap().as_string().unwrap(), "");
        assert_eq!(map.get("escape_string").unwrap().as_string().unwrap(), "hello\"dave\"");
    }
    
    #[test]
    fn test_numbers() {
        let input = r#"do local _= {
            first=0,
            second=42.5,
            third=-1
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();

        assert_eq!(map.get("first").unwrap().as_number().unwrap(), 0f64);
        assert_eq!(map.get("second").unwrap().as_number().unwrap(), 42.5);
        assert_eq!(map.get("third").unwrap().as_number().unwrap(), -1f64);
    }
    

    #[test]
    fn test_nested_map() {
        let input = r#"do local _= {
            nested={
                inner_key="value",
                inner_num=123
            }
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();
        let nested = map.get("nested").unwrap().as_map().unwrap();

        assert_eq!(nested.get("inner_key").unwrap().as_string().unwrap(), "value");
        assert_eq!(nested.get("inner_num").unwrap().as_number().unwrap(), 123.0);
    }

    #[test]
    fn test_array() {
        let input = r#"do local _= {
            array={1, 2, 3, 4}
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();
        let array = map.get("array").unwrap().as_array().unwrap();

        assert_eq!(array.len(), 4);
        assert_eq!(array[0].as_number().unwrap(), 1.0);
        assert_eq!(array[3].as_number().unwrap(), 4.0);
    }

    #[test]
    fn test_escaped_string() {
        let input = r#"do local _= {
            escaped="string\"with\"escapes"
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();

        assert_eq!(map.get("escaped").unwrap().as_string().unwrap(), "string\"with\"escapes");
    }

    #[test]
    fn test_invalid_utf8_string() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"do local _= { key=\"");

        bytes.push(0xFF);
        bytes.push(0xFE);
        bytes.extend_from_slice(b"\" } return _ end");

        let result = parse_lua_like_bytes(bytes).unwrap();
        let map = result.as_map().unwrap();
        
        assert!(map.get("key").unwrap().as_string().is_none()); // Valid UTF-8 conversion fails
        assert!(map.get("key").unwrap().as_bytes().is_some());  // But bytes are available
        assert!(map.get("key").unwrap().as_string_lossy().is_some()); // Lossy conversion works
    }

    #[test]
    fn test_example_input() {
        let input = r#"do local _= {
            key="string",
            key_with_map={
              inside_map_key=5,
              another_string_items_with_escapes="string\"escaped\"",
              nested_map = {
                 number=5.0,
                 string="hello"
              }
            },
            true_bool=true,
            false_bool=false,
            EnginePoint={
                0,
                132.20915122857144,
                8,
                -90
            },
            SomeKey = {
                0,
                132.2,
                8,
                -90
            },
            EngineBayBounds={
                {
                    44.160003662109375,
                    -183.5,
                    80.93353271484375
                },
                {
                    44.160003662109375,
                    -99.75,
                    100
                },
                {
                    44.160003662109375,
                    -99.75,
                    0
                },
                {
                    44.160003662109375,
                    -183.5,
                    0
                }
            }
        }
        return _
        end"#;

        let result = parse_lua_like_string(input).unwrap();
        let map = result.as_map().unwrap();
        
        assert_eq!(map.get("key").unwrap().as_string().unwrap(), "string");
        assert!(map.get("true_bool").unwrap().as_bool().unwrap());
        assert!(!map.get("false_bool").unwrap().as_bool().unwrap());
        
        let nested = map.get("key_with_map").unwrap().as_map().unwrap();
        assert_eq!(nested.get("inside_map_key").unwrap().as_number().unwrap(), 5.0);
        
        let engine_point = map.get("EnginePoint").unwrap().as_array().unwrap();
        assert_eq!(engine_point.len(), 4);
        assert_eq!(engine_point[1].as_number().unwrap(), 132.20915122857144);
        
        let engine_bay = map.get("EngineBayBounds").unwrap().as_array().unwrap();
        assert_eq!(engine_bay.len(), 4);
        let first_bounds = engine_bay[0].as_array().unwrap();
        assert_eq!(first_bounds[0].as_number().unwrap(), 44.160003662109375);
    }

    #[test]
    fn test_example_file() {
        let file_path = "/home/josykes/.steam/debian-installation/steamapps/compatdata/293760/pfx/drive_c/users/steamuser/AppData/Local/BeamNG.drive/mods/cc3/vehicles/cc3/ccc3_zephyj_clone___family_mk1_clone_clone_clone.car";
        let other_path = "/home/josykes/.steam/debian-installation/steamapps/compatdata/293760/pfx/drive_c/users/steamuser/AppData/Local/BeamNG.drive/mods/second/cc3/vehicles/cc3/ccc3_zephyj_clone___family_mk1_clone_clone_clone.car";
        let result = parse_lua_like_file(other_path).unwrap();
        println!("{}", result);
    }
}