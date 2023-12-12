mod speed;
mod gearing;

const GRAVITY: f64 = 9.81; // m/s^2
const AIR_DENSITY: f64 = 1.225; // Air density in kg/m³ (at sea level)
const STATIC_FRICTION_COEFFICIENT: f64 = 0.7;

pub use speed::SpeedApproximator;
pub use gearing::GearingCalculator;