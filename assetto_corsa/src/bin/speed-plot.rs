extern crate plotters;
extern crate ndarray;

use plotters::prelude::*;
use ndarray::prelude::*;

fn as_usize(x: &f64) -> String {
   format!("{}", *x as usize)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a chart context
    let root = BitMapBackend::new("chart.png", (800, 600)).into_drawing_area();
    root.fill(&BLACK)?;

    let car = setup_car1();
    let mut theoretical_speed: f64 = 0.0; // Initial speed (m/s)
    let mut current_speed: f64 = 0.0;
    let mut max_obtainable_speed: f64 = 0.0;
    let mut max_theoretical_speed: f64 = 0.0;

    let mut engine_rpm = car.min_rpm_needed_to_move_from_stationary() + 100.0; // Initial RPM
    let rolling_resistance_force = car.rolling_resistance_force();
    let rpm_limit = car.max_rpm();

    let mut real_gear_data_points: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut theoretical_gear_data_points: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current_real_gear_vec: Vec<(f64, f64)> = Vec::new();
    let mut current_theoretical_gear_vec: Vec<(f64, f64)> = Vec::new();
    let mut current_gear_idx = 0;
    let mut insufficient_forward_force = false;
    loop {
        if engine_rpm > rpm_limit {
            insufficient_forward_force = false;
            current_gear_idx += 1;
            engine_rpm = car.min_rpm_needed_to_move_from_stationary() + 100.0;
            real_gear_data_points.push(current_real_gear_vec);
            theoretical_gear_data_points.push(current_theoretical_gear_vec);
            current_real_gear_vec = Vec::new();
            current_theoretical_gear_vec = Vec::new();
            if current_gear_idx > car.max_gear_idx() {
                break;
            }
        }
        theoretical_speed = car.engine_rpm_to_wheel_speed(engine_rpm, current_gear_idx);

        let force_at_wheels = car.wheel_force_at(engine_rpm, current_gear_idx);
        let drag_force = car.drag_force_at(theoretical_speed);
        let net_force = force_at_wheels - drag_force - rolling_resistance_force;
        if !insufficient_forward_force && net_force >= 0.0 {
            current_speed = theoretical_speed;
            max_obtainable_speed = max_obtainable_speed.max(current_speed);
            current_real_gear_vec.push((current_speed * 3.6, engine_rpm));
        } else {
            if !insufficient_forward_force {
                insufficient_forward_force = true;
                if let Some(start_point) = current_real_gear_vec.last() {
                    current_theoretical_gear_vec.push(*start_point)
                }
            }
            max_theoretical_speed = max_theoretical_speed.max(theoretical_speed);
            current_theoretical_gear_vec.push((theoretical_speed * 3.6, engine_rpm));
        }
        engine_rpm += 50.0;
    }

    let x_axis_limit = (max_obtainable_speed.max(max_theoretical_speed) * 3.6) + 10f64;
    let y_axis_start = 0f64.max(car.min_rpm_needed_to_move_from_stationary()-500f64);
    let y_axis_limit = rpm_limit+1000f64;
    let x = FontDesc::new(FontFamily::Name("Arial"), 20.0, FontStyle::Normal);
    let mut context = ChartBuilder::on(&root)
        .margin(15)
        .caption("Ratios Graph", x.color(&WHITE))
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .build_cartesian_2d(0f64..x_axis_limit,
                            y_axis_start..y_axis_limit)?;

    // Create lines
    context
        .configure_mesh()
        .x_labels(10)
        .x_label_formatter(&as_usize)
        .x_desc("Speed (Km/H)")
        .y_labels(20)
        .y_desc("Engine RPM")
        .label_style(&WHITE)
        .bold_line_style(&WHITE.mix(0.2))
        .light_line_style(&WHITE.mix(0.1))
        .draw()?;

    for (idx, data) in real_gear_data_points.into_iter().enumerate() {
        context
            .draw_series(LineSeries::new(data, &YELLOW))?
            .label(format!("Gear {}", idx + 1))
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &YELLOW));
    }

    for (idx, data) in theoretical_gear_data_points.into_iter().enumerate() {
        if data.is_empty() {
            continue;
        }
        context
            .draw_series(LineSeries::new(data, &CYAN))?
            .label(format!("Gear {} Theoretical", idx + 1))
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &CYAN));
    }

    // Configure the legend
    context
        .configure_series_labels()
        .label_font(&WHITE)
        .background_style(&BLACK.mix(0.8))
        .border_style(&WHITE)
        .draw()?;

    Ok(())
}

const GRAVITY: f64 = 9.81; // m/s^2
const AIR_DENSITY: f64 = 1.225; // Air density in kg/m³ (at sea level)
const STATIC_FRICTION_COEFFICIENT: f64 = 0.7;

struct CarParams {
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

impl CarParams {
    fn new(rpm_values: Array1<f64>,
           torque_values: Array1<f64>,
           gear_ratios: Vec<f64>,
           final_drive: f64,
           mass: f64,
           wheel_radius: f64,
           coefficient_of_rolling_resistance: f64,
           frontal_area: f64,
           drag_coefficient: f64
    ) -> CarParams {
        CarParams { rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, coefficient_of_rolling_resistance, frontal_area, drag_coefficient}
    }

    fn min_rpm(&self) -> f64 {
        *self.rpm_values.first().unwrap()
    }

    fn max_rpm(&self) -> f64 {
        *self.rpm_values.last().unwrap()
    }

    fn calculate_acceleration_load_for_speed(
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

    fn torque_needed_to_move_from_stationary(&self) -> f64 {
        let normal_force = &self.mass * GRAVITY; // Calculate the normal force
        let static_friction_force = STATIC_FRICTION_COEFFICIENT * normal_force;

        // Calculate minimum torque required to overcome static friction
        let minimum_torque = static_friction_force * &self.wheel_radius;
        minimum_torque
    }

    fn min_rpm_needed_to_move_from_stationary(&self) -> f64 {
        let minimum_torque = self.torque_needed_to_move_from_stationary();
        min_rpm_needed_for_torque(minimum_torque,
                                  &self.rpm_values,
                                  &self.torque_values,
                                  *self.gear_ratios.first().unwrap(),
                                  self.final_drive,
                                  self.wheel_radius).expect("Not enough torque to move car")
    }

    fn interpolate_engine_torque(&self, rpm: f64) -> f64 {
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

    fn wheel_torque_at(&self, rpm: f64, gear_index: usize) -> f64 {
        let engine_torque_at_rpm = self.interpolate_engine_torque(rpm);
        let wheel_torque = (engine_torque_at_rpm * self.gear_ratios[gear_index] * self.final_drive) / self.wheel_radius;
        wheel_torque
    }

    fn wheel_force_at(&self, rpm: f64, gear_index: usize) -> f64 {
        self.wheel_torque_at(rpm, gear_index) / self.wheel_radius
    }

    fn engine_rpm_to_wheel_speed(&self, engine_rpm: f64, gear_ratio_idx: usize) -> f64 {
        // Calculate vehicle speed in m/s
        let vehicle_speed = (engine_rpm * 2.0 * std::f64::consts::PI * self.wheel_radius) /
            (60.0 * self.gear_ratios[gear_ratio_idx] * self.final_drive);
        vehicle_speed
    }

    fn gravitational_force(&self) -> f64 {
        // gravitational_force represents the force due to gravity and is calculated as the product of the car's mass (kg)
        // and the gravitational acceleration (m/s^2). Therefore, its units are also N (Newtons).
        self.mass * GRAVITY
    }

    fn rolling_resistance_force(&self) -> f64 {
        // rolling_resistance_force represents the force due to rolling resistance and is
        // calculated as a product of the
        // rolling resistance coefficient (dimensionless), car mass (kg), and gravitational acceleration (m/s^2).
        // Therefore, its units are N (Newtons).
        self.coefficient_of_rolling_resistance * self.gravitational_force()
    }

    fn drag_force_at(&self, speed: f64) -> f64 {
        // Calculate resistance forces
        // drag_force represents the aerodynamic drag force and is calculated using
        // the drag coefficient (dimensionless), frontal area (square meters), air density (kg/m^3), and speed (m/s).
        // Therefore, its units are N (Newtons), as the formula incorporates all the necessary conversions.
        0.5 * self.drag_coefficient * self.frontal_area * AIR_DENSITY * speed.powi(2)
    }

    fn max_gear_idx(&self) -> usize {
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

fn setup_car1() -> CarParams {
    let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
    let torque_values = Array::from_vec(vec![30.0, 100.0, 250.0, 200.0, 140.0]);

    let gear_ratios = vec![4.0, 3.0, 2.5, 2.0, 1.5];
    let final_drive = 3.5;

    let mass = 1500.0; // kg
    let wheel_radius = 0.3;

    let frontal_area = 2.0; // m^2
    let drag_coefficient = 0.3;

    let rolling_resistance_coefficient = 0.01;

    CarParams::new(rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, rolling_resistance_coefficient, frontal_area, drag_coefficient)
}

fn setup_car2() -> CarParams {
    let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0]);
    let torque_values = Array::from_vec(vec![30.0, 100.0, 250.0, 200.0, 140.0, 100.0]);

    let gear_ratios = vec![3.5, 2.5, 1.8, 1.2, 0.8];
    let final_drive = 3.0;

    let mass = 1500.0; // kg
    let wheel_radius = 0.3;

    let frontal_area = 2.0; // m^2
    let drag_coefficient = 0.5;

    let rolling_resistance_coefficient = 0.01;

    CarParams::new(rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, rolling_resistance_coefficient, frontal_area, drag_coefficient)
}