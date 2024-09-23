use assetto_corsa::Car;
use assetto_corsa::car::data;
use assetto_corsa::car::data::drivetrain::traction::DriveType;
use assetto_corsa::car::data::{Drivetrain, Engine};
use assetto_corsa::car::data::engine::{EngineData, PowerCurve};
use assetto_corsa::car::lut_utils::LutInterpolator;
use assetto_corsa::traits::{extract_mandatory_section, MandatoryDataSection};

pub(crate) fn get_fuel_use_per_sec_at_rpm(eff_percentage: i32, fuel_lhv: f64, power_kw: f64) -> f64 {
    // fuel lhv in kWh/g
    // BSFC [g/(kW⋅h)] = 1 / (eff * fuel_lhv)
    // https://en.wikipedia.org/wiki/Brake-specific_fuel_consumption
    // BSFC (g/J) = fuel_consumption (g/s) / power (watts)
    // fuel_consumption (g/s) = BSFC * power (watts)
    // BSFC value stored in econ curve as g/kWh
    // BSFC [g/(kW⋅h)] = BSFC [g/J] × (3.6 × 10^6)
    let power_watts = power_kw * 1000.0;
    let eff: f64 = eff_percentage as f64 / 100.0;
    let bsfc = 1.0 / (eff * fuel_lhv);
    (bsfc / 3600000_f64) * power_watts
}

pub(crate) fn get_fuel_use_kg_per_hour(eff_percentage: i32, fuel_lhv: f64, power_kw: f64) -> i32{
    (get_fuel_use_per_sec_at_rpm(eff_percentage, fuel_lhv, power_kw) * 3.6).round() as i32
}

// TODO this would be a useful func on one of the engine structs; come back and refactor
pub(crate) fn get_min_max_rpms(engine_ini: &Engine) -> Result<(i32, i32), String> {
    match EngineData::load_from_parent(engine_ini) {
        Ok(ed) => {
            Ok((ed.minimum, ed.limiter))
        }
        Err(e) => {
            return Err(format!("Failed to load engine data. {}", e.to_string()));
        }
    }
}

pub(crate) fn load_drive_type(car: &mut Car) -> Result<DriveType, String> {
    let drivetrain = Drivetrain::from_car(car).map_err(|e|{
        format!("Failed to load {}. {}", Drivetrain::INI_FILENAME.to_string(), e.to_string())
    })?;
    Ok(extract_mandatory_section::<data::drivetrain::Traction>(&drivetrain).map_err(|_|{
        format!("{} is missing data section 'Traction'", Drivetrain::INI_FILENAME.to_string())
    })?.drive_type)
}

pub(crate) fn create_engine_power_interpolator(engine: &Engine,
                                               mechanical_efficiency: f64,
                                               boost_interpolator_opt: Option<LutInterpolator<f64, f64>>)
                                               -> Result<LutInterpolator<i32, f64>, String>
{
    match PowerCurve::load_from_parent(engine) {
        Ok(curve) => {
            let power_curve_vec: Vec<(i32, f64)> = curve.get_lut().to_vec().into_iter().map(
                |(rpm, torque)|{
                    let mut scaled_torque = (torque / (mechanical_efficiency * 100.0)) * 100.0;
                    if let Some(boost_interpolator) = &boost_interpolator_opt {
                        scaled_torque = match boost_interpolator.get_value(rpm as f64) {
                            None => scaled_torque,
                            Some(boost) => scaled_torque * (1.0 + boost)
                        }
                    }
                    let power = (scaled_torque * (rpm as f64) * 2.0 * std::f64::consts::PI) / (60.0 * 1000.0);
                    (rpm, power)
                }
            ).collect();
            Ok(LutInterpolator::from_vec(power_curve_vec))
        },
        Err(e) => {
            Err(format!("Failed to load engine curve data. {}", e.to_string()))
        }
    }
}

// pub fn fuel_flow_consumption(mechanical_efficiency: f64) -> data::engine::FuelConsumptionFlowRate {
//     // The lut values should be: rpm, kg/hr
//     // The max-flow should be weighted to the upper end of the rev-range as racing is usually done in that range.
//     // This is probably enough of a fallback as this will only be used if a lut isn't found and that will be
//     // calculated below
//     let max_flow_entry_index = (self.engine_sqlite_data.rpm_curve.len() as f64 * 0.70).round() as usize;
//     let max_fuel_flow = (self.get_fuel_use_per_sec_at_rpm(max_flow_entry_index) * 3.6).round() as i32;
//
//     let mut max_flow_lut: Vec<(i32, i32)> = Vec::new();
//     for (rpm_idx, rpm) in self.engine_sqlite_data.rpm_curve.iter().enumerate() {
//         max_flow_lut.push((*rpm as i32, (self.get_fuel_use_per_sec_at_rpm(rpm_idx) * 3.6).round() as i32))
//     }
//     data::engine::FuelConsumptionFlowRate::new(
//         0.03,
//         (self.idle_speed().unwrap() + 100_f64).round() as i32,
//         mechanical_efficiency,
//         Some(max_flow_lut),
//         max_fuel_flow
//     )
// }
