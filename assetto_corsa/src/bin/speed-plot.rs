extern crate plotters;
extern crate ndarray;

use plotters::prelude::*;
use ndarray::prelude::*;
use assetto_corsa::{Car, Installation};

use assetto_corsa::car::model::SpeedApproximator;

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

fn setup_car1() -> SpeedApproximator {
    let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
    let torque_values = Array::from_vec(vec![30.0, 100.0, 250.0, 200.0, 140.0]);

    let gear_ratios = vec![4.0, 3.0, 2.5, 2.0, 1.5];
    let final_drive = 3.5;

    let mass = 1500.0; // kg
    let wheel_radius = 0.3;

    let frontal_area = 2.0; // m^2
    let drag_coefficient = 0.3;

    let rolling_resistance_coefficient = 0.01;

    SpeedApproximator::new(rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, rolling_resistance_coefficient, frontal_area, drag_coefficient)
}

fn setup_car2() -> SpeedApproximator {
    let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0]);
    let torque_values = Array::from_vec(vec![30.0, 100.0, 250.0, 200.0, 140.0, 100.0]);

    let gear_ratios = vec![3.5, 2.5, 1.8, 1.2, 0.8];
    let final_drive = 3.0;

    let mass = 1500.0; // kg
    let wheel_radius = 0.3;

    let frontal_area = 2.0; // m^2
    let drag_coefficient = 0.5;

    let rolling_resistance_coefficient = 0.01;

    SpeedApproximator::new(rpm_values, torque_values, gear_ratios, final_drive, mass, wheel_radius, rolling_resistance_coefficient, frontal_area, drag_coefficient)
}

fn load_car(folder_name: &str) -> Result<SpeedApproximator, String> {
    let ac_install = Installation::new();
    let car_folder_root = ac_install.get_installed_car_path();
    let car_folder_path = car_folder_root.join(folder_name);
    let car = Car::load_from_path(&car_folder_path).map_err(|err| err.to_string())?;


    Err("Not Implemented".to_string())
}