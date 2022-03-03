use std::path::Path;
use crate::{automation, beam_ng};
use crate::automation::sandbox::{EngineV1, load_engine_by_uuid};

pub fn build_ac_engine_from_beam_ng_mod(beam_ng_mod_path: &Path) -> Result<(), String>{
    let mod_data = beam_ng::extract_mod_data(beam_ng_mod_path).unwrap();
    let car_file = automation::car::CarFile::from_bytes(mod_data.car_file_data).unwrap();
    let uid = car_file.get_section("Car").unwrap().get_section("Variant").unwrap().get_attribute("UID").unwrap().value.as_str().unwrap();
    println!("Car UID = {}", uid);
    let eng = match load_engine_by_uuid(uid)? {
        None => { return Err(String::from("No engine found")); }
        Some(eng) => { eng }
    };
    println!("{:?}", eng);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::beam_ng::get_mod_list;
    use crate::fabricator::build_ac_engine_from_beam_ng_mod;

    #[test]
    fn load_mods() -> Result<(), String> {
        let mods = get_mod_list().unwrap();
        build_ac_engine_from_beam_ng_mod(PathBuf::from(&mods[0]).as_path());
        Ok(())
    }
}
