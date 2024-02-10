extern crate ndarray;

use ndarray::Array1;
use crate::car::model::{AIR_DENSITY, GRAVITY, STATIC_FRICTION_COEFFICIENT};


pub struct SpeedApproximator {
    rpm_values: Array1<f64>,
    torque_values: Array1<f64>,
    gear_ratios: Vec<f64>,
    final_drive: f64,
    mass: f64,
    wheel_radius: f64,
    coefficient_of_rolling_resistance: f64,
    frontal_area: f64,
    drag_coefficient: f64,
}

impl SpeedApproximator {
    pub fn new(rpm_values: Array1<f64>,
           torque_values: Array1<f64>,
           gear_ratios: Vec<f64>,
           final_drive: f64,
           mass: f64,
           wheel_radius: f64,
           coefficient_of_rolling_resistance: f64,
           frontal_area: f64,
           drag_coefficient: f64
    ) -> SpeedApproximator {
        SpeedApproximator { rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, coefficient_of_rolling_resistance, frontal_area, drag_coefficient}
    }

    pub fn min_rpm(&self) -> f64 {
        *self.rpm_values.first().unwrap()
    }

    pub fn max_rpm(&self) -> f64 {
        *self.rpm_values.last().unwrap()
    }

    pub fn calculate_acceleration_load_for_speed(
        &self,
        speed: f64,
        desired_acceleration: f64, // Desired acceleration in meters per second squared (m/s²)
    ) -> f64
    {
        // Calculate the load required for acceleration
        let load_due_to_acceleration = self.mass * desired_acceleration;
        let load_due_to_rolling_resistance = self.mass * GRAVITY * self.coefficient_of_rolling_resistance;
        let load_due_to_drag = 0.5 * AIR_DENSITY * speed.powi(2) * self.drag_coefficient * self.frontal_area;

        // Calculate the total load required for acceleration (sum of all loads)
        let total_load = load_due_to_acceleration + load_due_to_rolling_resistance + load_due_to_drag;
        total_load
    }

    pub fn torque_needed_to_move_from_stationary(&self) -> f64 {
        let normal_force = &self.mass * GRAVITY; // Calculate the normal force
        let static_friction_force = STATIC_FRICTION_COEFFICIENT * normal_force;

        // Calculate minimum torque required to overcome static friction
        let minimum_torque = static_friction_force * &self.wheel_radius;
        minimum_torque
    }

    pub fn min_rpm_needed_to_move_from_stationary(&self) -> f64 {
        let minimum_torque = self.torque_needed_to_move_from_stationary();
        min_rpm_needed_for_torque(minimum_torque,
                                  &self.rpm_values,
                                  &self.torque_values,
                                  *self.gear_ratios.first().unwrap(),
                                  self.final_drive,
                                  self.wheel_radius).expect("Not enough torque to move car")
    }

    pub fn interpolate_engine_torque(&self, rpm: f64) -> f64 {
        // Linear interpolation to find torque at a given RPM
        let mut prev_rpm = 0.0;
        let mut prev_torque = 0.0;
        let mut found = false;

        for (i, &rpm_value) in self.rpm_values.iter().enumerate() {
            if rpm_value >= rpm {
                found = true;
                let next_torque = self.torque_values[i];
                let next_rpm = rpm_value;

                if i == 0 {
                    return next_torque;
                }

                // Linear interpolation
                let slope = (next_torque - prev_torque) / (next_rpm - prev_rpm);
                return prev_torque + slope * (rpm - prev_rpm);
            }

            prev_rpm = rpm_value;
            prev_torque = self.torque_values[i];
        }

        // If RPM is higher than the provided values, return the torque at the highest RPM
        if !found {
            return prev_torque;
        }

        0.0
    }

    pub fn wheel_torque_at(&self, rpm: f64, gear_index: usize) -> f64 {
        let engine_torque_at_rpm = self.interpolate_engine_torque(rpm);
        let wheel_torque = (engine_torque_at_rpm * self.gear_ratios[gear_index] * self.final_drive) / self.wheel_radius;
        wheel_torque
    }

    pub fn wheel_force_at(&self, rpm: f64, gear_index: usize) -> f64 {
        self.wheel_torque_at(rpm, gear_index) / self.wheel_radius
    }

    pub fn engine_rpm_to_wheel_speed(&self, engine_rpm: f64, gear_ratio_idx: usize) -> f64 {
        // Calculate vehicle speed in m/s
        let vehicle_speed = (engine_rpm * 2.0 * std::f64::consts::PI * self.wheel_radius) /
            (60.0 * self.gear_ratios[gear_ratio_idx] * self.final_drive);
        vehicle_speed
    }

    pub fn gravitational_force(&self) -> f64 {
        // gravitational_force represents the force due to gravity and is calculated as the product of the car's mass (kg)
        // and the gravitational acceleration (m/s^2). Therefore, its units are also N (Newtons).
        self.mass * GRAVITY
    }

    pub fn rolling_resistance_force(&self) -> f64 {
        // rolling_resistance_force represents the force due to rolling resistance and is
        // calculated as a product of the
        // rolling resistance coefficient (dimensionless), car mass (kg), and gravitational acceleration (m/s^2).
        // Therefore, its units are N (Newtons).
        self.coefficient_of_rolling_resistance * self.gravitational_force()
    }

    pub fn drag_force_at(&self, speed: f64) -> f64 {
        // Calculate resistance forces
        // drag_force represents the aerodynamic drag force and is calculated using
        // the drag coefficient (dimensionless), frontal area (square meters), air density (kg/m^3), and speed (m/s).
        // Therefore, its units are N (Newtons), as the formula incorporates all the necessary conversions.
        0.5 * self.drag_coefficient * self.frontal_area * AIR_DENSITY * speed.powi(2)
    }

    pub fn max_gear_idx(&self) -> usize {
        self.gear_ratios.len() - 1
    }
}

fn min_rpm_needed_for_torque(minimum_torque: f64,
                             rpm_values: &Array1<f64>,
                             torque_values: &Array1<f64>,
                             gear_ratio: f64,
                             final_drive_ratio: f64,
                             wheel_radius: f64) -> Option<f64>
{
    // Find the minimum RPM at which the engine produces the required minimum torque
    let mut minimum_rpm_to_move = 0.0; // Initialize to zero RPM
    let rpm_limit = 5000.0;
    let mut rpm = *rpm_values.first().unwrap();
    let rpm_step = 100;
    loop {
        let engine_torque_at_rpm = interpolate_torque(&rpm_values, &torque_values, rpm);
        let wheel_torque = (engine_torque_at_rpm * gear_ratio * final_drive_ratio) / wheel_radius;
        if wheel_torque >= minimum_torque {
            minimum_rpm_to_move = rpm;
            break;
        }
        rpm += rpm_step as f64;
        if rpm >= rpm_limit {
            break;
        }
    }
    if minimum_rpm_to_move == 0.0 {
        None
    } else {
        Some(minimum_rpm_to_move)
    }
}

fn interpolate_torque(rpm_values: &Array1<f64>, torque_values: &Array1<f64>, rpm: f64) -> f64 {
    // Linear interpolation to find torque at a given RPM
    let mut prev_rpm = 0.0;
    let mut prev_torque = 0.0;
    let mut found = false;

    for (i, &rpm_value) in rpm_values.iter().enumerate() {
        if rpm_value >= rpm {
            found = true;
            let next_torque = torque_values[i];
            let next_rpm = rpm_value;

            if i == 0 {
                return next_torque;
            }

            // Linear interpolation
            let slope = (next_torque - prev_torque) / (next_rpm - prev_rpm);
            return prev_torque + slope * (rpm - prev_rpm);
        }

        prev_rpm = rpm_value;
        prev_torque = torque_values[i];
    }

    // If RPM is higher than the provided values, return the torque at the highest RPM
    if !found {
        return prev_torque;
    }

    0.0
}