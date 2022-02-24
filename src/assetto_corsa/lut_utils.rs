use std::{fmt, io};
use std::fs::File;
use std::path::Path;
use std::str::FromStr;
use csv::Terminator;

pub fn load_lut_from_path<K, V>(lut_path: &Path) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr, <V as FromStr>::Err: fmt::Debug
{
    let file = match File::open(lut_path) {
        Ok(file) => { file }
        Err(e) => {
            return Err(format!("Failed to open {}: {}", lut_path.display(), e.to_string()));
        }
    };
    load_lut_from_reader(&file, b'|', Terminator::CRLF)
}

pub fn load_lut_from_reader<K, V, R>(lut_reader: R, delimiter: u8, terminator: Terminator) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr, <V as FromStr>::Err: fmt::Debug,
        R: io::Read
{
    let mut lut_data: Vec<(K, V)> = Vec::new();
    let mut rdr = csv::ReaderBuilder::new().has_headers(false).delimiter(delimiter).terminator(terminator).from_reader(lut_reader);
    for result in rdr.records() {
        match result {
            Ok(record) => {
                lut_data.push((parse_lut_element(&record, 0)?,
                               parse_lut_element(&record, 1)?));
            },
            _ => {}
        }
    }
    Ok(lut_data)
}

pub fn load_lut_from_property_value<K, V>(property_value: String, data_dir: &Path) -> Result<Vec<(K, V)>, String>
    where
        K: std::str::FromStr, <K as FromStr>::Err: fmt::Debug,
        V: std::str::FromStr, <V as FromStr>::Err: fmt::Debug
{
    return match property_value.starts_with("(") {
        true => {
            let data_slice = &property_value[1..(property_value.len() - 1)];
            load_lut_from_reader::<K, V, _>(data_slice.as_bytes(), b'=', Terminator::Any(b'|'))
        }
        false => {
            load_lut_from_path::<K, V>(data_dir.join(property_value.as_str()).as_path()) 
        }
    }
}

pub fn parse_lut_element<T>(record: &csv::StringRecord, index: usize) -> Result<T, String>
    where
        T: std::str::FromStr, <T as FromStr>::Err: fmt::Debug
{
    match record.get(index).unwrap().parse::<T>() {
        Ok(s) => { Ok(s) },
        Err(e) => {
            let mut err_str = String::from("Invalid lut types, Cannot convert first item");
            if let Some(pos) = record.position() {
                err_str.push_str(&format!(" at line {}", pos.line()));
            }
            return Err(format!("{} to {}",
                               err_str, std::any::type_name::<T>()))
        }
    }
}

pub fn write_lut_to_path<K, V>(data: &Vec<(K,V)>, path: &Path) -> Result<(), String>
    where
      K: std::fmt::Display,
      V: std::fmt::Display
{
    let mut writer = csv::WriterBuilder::new().has_headers(false).delimiter(b'|').from_path(path).map_err(
        |err| { format!("Couldn't write lut. {}", err.to_string()) }
    )?;
    for (key, val) in data {
        writer.write_record(&[key.to_string(), val.to_string()]).map_err(|err| {
            format!("Couldn't write lut. {}", err.to_string())
        })?;
    }
    writer.flush().map_err(
        |err| { format!("Couldn't write lut. {}", err.to_string()) }
    )?;
    Ok(())
}

pub fn write_lut_to_property_value<K, V>(data: &Vec<(K,V)>, delimiter: u8, terminator: Terminator) -> Result<String, String>
    where
      K: std::fmt::Display,
      V: std::fmt::Display
{
    let mut out : Vec<u8> = Vec::new();
    out.push(b'(');
    {
        let mut writer = csv::WriterBuilder::new().has_headers(false).delimiter(delimiter).terminator(terminator).flexible(true).from_writer(&mut out);
        for (key, val) in data {
            writer.write_record(&[key.to_string(), val.to_string()]).map_err(|err| {
                err.to_string()
            })?;
        }
    }
    out.pop();
    out.push(b')');
    return match std::str::from_utf8(&out) {
        Ok(v) => Ok(v.to_owned()),
        Err(e) => Err(e.to_string()),
    };
}

#[test]
fn load_lut_string() {
    let data = String::from("(100=12|200=13|300=40)");
    let mut vec: Vec<(i32, i32)> = load_lut_from_property_value(data, Path::new("")).unwrap();
    println!("{:?}", vec);
    let out = write_lut_to_property_value(&vec, b'=', Terminator::Any(b'|')).unwrap();
    println!("{}", out);
}
