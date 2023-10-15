
#[cfg(test)]
mod tests {
    extern crate ndarray;
    use ndarray::prelude::*;

    const GRAVITY: f64 = 9.81; // m/s^2
    const AIR_DENSITY: f64 = 1.225; // Air density in kg/m³ (at sea level)
    const STATIC_FRICTION_COEFFICIENT: f64 = 0.7;

    // let engine_power = (2.0 * std::f64::consts::PI * engine_rpm * engine_torque) / 60_000.0; // kW

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

    fn speed_to_rpm(speed: f64, gear_ratio: f64, final_drive_ratio: f64, wheel_radius: f64) -> f64 {
        // Calculate engine RPM based on car speed, gear ratio, final drive ratio, and wheel radius
        let wheel_circumference = 2.0 * std::f64::consts::PI * wheel_radius;
        let wheel_angular_velocity = speed / wheel_circumference;
        let engine_angular_velocity = wheel_angular_velocity * gear_ratio * final_drive_ratio;

        // Convert angular velocity to RPM
        engine_angular_velocity * 60.0 / (2.0 * std::f64::consts::PI)
    }

    fn rpm_to_speed(engine_rpm: f64, wheel_radius: f64, gear_ratio: f64, final_drive_ratio: f64) -> f64 {
        // Calculate vehicle speed in m/s
        let vehicle_speed = (engine_rpm * 2.0 * std::f64::consts::PI * wheel_radius) /
            (60.0 * gear_ratio * final_drive_ratio);

        vehicle_speed
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

            let minimum_rpm_to_move =
                min_rpm_needed_for_torque(minimum_torque,
                                          &self.rpm_values,
                                          &self.torque_values,
                                          *self.gear_ratios.first().unwrap(),
                                          self.final_drive,
                                          self.wheel_radius).expect("Not enough torque to move car"); // Initialize to zero RPM
            minimum_torque
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

    #[test]
    fn calc_min_rpm_to_move() {
        let car = setup_car1();

        // Calculate minimum torque required to overcome static friction
        let minimum_torque = car.torque_needed_to_move_from_stationary();
        let rpm_limit = car.max_rpm();
        let mut rpm = car.min_rpm();

        // Find the minimum RPM at which the engine produces the required minimum torque
        let mut minimum_rpm_to_move = 0.0; // Initialize to zero RPM
        let rpm_step = 100;

        loop {
            let force_at_wheel = car.wheel_force_at(rpm, 0);
            if force_at_wheel >= minimum_torque {
                minimum_rpm_to_move = rpm;
                break;
            }
            rpm += rpm_step as f64;
            if rpm >= rpm_limit {
                break;
            }
        }
        if minimum_rpm_to_move == 0.0 {
            println!("Not enough engine torque to move car");
        } else {
            println!("Minimum RPM required = {} rpm", minimum_rpm_to_move);
        }
    }

    #[test]
    fn calc_max_speed() {
        let car = setup_car1();

        let mut theoretical_speed: f64 = 0.0; // Initial speed (m/s)
        let mut max_speed: f64 = 0.0;

        let mut engine_rpm = car.torque_needed_to_move_from_stationary() + 100.0; // Initial RPM
        let rolling_resistance_force = car.rolling_resistance_force();
        let rpm_limit = car.max_rpm();
        loop {
            if engine_rpm > rpm_limit {
                // TODO plot this on a graph
                println!("Maximum Speed: {:.2} m/s", max_speed);
                println!("Maximum Speed: {:.2} km/h", max_speed * 3.6);
                break;
            }
            theoretical_speed = car.engine_rpm_to_wheel_speed(engine_rpm, 0);
            let force_at_wheels = car.wheel_force_at(engine_rpm, 0);
            let drag_force = car.drag_force_at(theoretical_speed);
            let net_force = force_at_wheels - drag_force - rolling_resistance_force;
            if net_force >= 0.0 {
                max_speed = theoretical_speed;
            }
            engine_rpm += 100.0;
        }
    }

//     fn clutch_example() {
//         let frontal_area = 2.0; // m^2
//         let air_density = 1.225; // kg/m^3
//         let rolling_resistance_coefficient = 0.01;
//
//         let car_mass = 1500.0; // kg
//         let drag_coefficient = 0.3;
//         let car =
//             CarParams::new(
//                 car_mass,
//                 rolling_resistance_coefficient,
//                 frontal_area,
//                 drag_coefficient
//             );
//
//         // Define a variable to represent the clutch state (engaged or disengaged)
//         let mut clutch_engaged = true;
//
//         // Calculate the total load required for acceleration (adjust as needed)
//         let acceleration_load_threshold = car.calculate_acceleration_load_for_speed(0.0, 0.1);
//
// // Main simulation loop
//         loop {
//             // Calculate total load on the engine
//             let total_load = calculate_load(vehicle_speed, road_gradient, vehicle_weight);
//
//             // Update the engine load threshold based on the load required for acceleration
//             if !clutch_engaged {
//                 engine_load_threshold = acceleration_load_threshold;
//             } else {
//                 // You can add hysteresis or other logic here if needed
//                 engine_load_threshold = 0.8 * acceleration_load_threshold; // Adjust as needed
//             }
//
//             // Check if the engine can provide enough torque to overcome the load
//             if total_load > engine_load_threshold {
//                 // If the engine load exceeds the threshold, engage the clutch to prevent stalling
//                 clutch_engaged = true;
//             } else {
//                 // If the engine load is below the threshold, disengage the clutch to allow slip
//                 clutch_engaged = false;
//             }
//
//             // Apply clutch slip effect if the clutch is disengaged
//             if !clutch_engaged {
//                 let clutch_slip = calculate_clutch_slip(total_load);
//                 engine_rpm += (torque / engine_inertia) * time_step * clutch_slip;
//             } else {
//                 // Normal operation with the clutch engaged
//                 engine_rpm += (torque / engine_inertia) * time_step;
//             }
//
//             // Apply other simulation updates (forces, speed, etc.)
//             // ...
//
//             // Check for user input, update time step, and exit loop as needed
//             // ...
//         }
//
//     }

    #[test]
    fn single_gear() {
        let frontal_area = 2.0; // m^2
        let air_density = 1.225; // kg/m^3
        let rolling_resistance_coefficient = 0.01;

        let car_mass = 1500.0; // kg
        let drag_coefficient = 0.3;
        let final_drive_ratio = 3.5; // Adjust to your car's final drive ratio
        let wheel_radius = 0.3; // meters (adjust to your car's wheel radius)
        let static_friction_coefficient = 0.7;

        // Define torque map (RPM vs. Torque)
        let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
        let torque_values = Array::from_vec(vec![100.0, 250.0, 250.0, 200.0, 140.0]);

        let gear_ratio = 3.5;

        let normal_force = car_mass * GRAVITY; // Calculate the normal force
        let static_friction_force = static_friction_coefficient * normal_force;

        // Calculate minimum torque required to overcome static friction
        let minimum_torque = static_friction_force * wheel_radius;

        // Find the minimum RPM at which the engine produces the required minimum torque
        let mut minimum_rpm_to_move =
            min_rpm_needed_for_torque(minimum_torque,
                                      &rpm_values,
                                      &torque_values,
                                      gear_ratio,
                                      final_drive_ratio,
                                      wheel_radius).expect("Not enough torque to move car"); // Initialize to zero RPM

        let rpm_limit = 5000.0;

        // Integration parameters
        let dt = 0.1; // Time step (seconds)
        let mut speed: f64 = 0.2777778; // Initial speed (m/s)

        let rpm_limit = 5000.0;

        // Start simulation
        let mut time = 0.0;
        let mut engine_rpm = minimum_rpm_to_move + 100.0; // Initial RPM

        let min_rpm = 0.0;
        let found_min_rpm = false;

        loop {
            // Calculate engine torque and power based on current RPM and gear
            let engine_torque = interpolate_torque(&rpm_values, &torque_values, engine_rpm);
            let engine_power = (2.0 * std::f64::consts::PI * engine_rpm * engine_torque) / 60_000.0; // kW

            // Calculate resistance forces
            // drag_force represents the aerodynamic drag force and is calculated using
            // the drag coefficient (dimensionless), frontal area (square meters), air density (kg/m^3), and speed (m/s).
            // Therefore, its units are N (Newtons), as the formula incorporates all the necessary conversions.
            let drag_force = 0.5 * drag_coefficient * frontal_area * air_density * speed.powi(2);

            // rolling_resistance_force represents the force due to rolling resistance and is
            // calculated as a product of the
            // rolling resistance coefficient (dimensionless), car mass (kg), and gravitational acceleration (m/s^2).
            // Therefore, its units are N (Newtons).
            let rolling_resistance_force = rolling_resistance_coefficient * car_mass * GRAVITY;

            // gravitational_force represents the force due to gravity and is calculated as the product of the car's mass (kg)
            // and the gravitational acceleration (m/s^2). Therefore, its units are also N (Newtons).
            let gravitational_force = car_mass * GRAVITY;

            // Calculate net force in N
            //   engine_power * 1000.0: This calculates the engine power in watts (W) by multiplying engine_power (kW) by 1000.0 to convert it to watts.
            //   / speed: This division calculates the term (engine_power * 1000.0) / speed, which represents the power-to-speed ratio in watts per meter per second (W/m/s).
            //   - drag_force: This subtracts the aerodynamic drag force (N) from the result obtained in step 2.
            //   - rolling_resistance_force: This subtracts the rolling resistance force (N) from the result obtained in step 3.
            //   - gravitational_force: This subtracts the gravitational force (N) from the result obtained in step 4.

            // Calculate net force
            let net_force = (engine_power * 1000.0 / speed) - drag_force - rolling_resistance_force - gravitational_force;

            // Calculate acceleration
            let acceleration = net_force / car_mass;

            // Update speed using numerical integration (Euler's method)
            speed += acceleration * dt;

            // Check if engine RPM exceeds redline; if so, we're done
            if engine_rpm >= rpm_limit || net_force <= 0.0 {
                println!("Maximum Speed: {:.2} m/s", speed);
                println!("Time to Reach Maximum Speed: {:.2} seconds", time);
                break;
            }

            // Update time and RPM
            time += dt;
            let expected_rpm = speed_to_rpm(speed, gear_ratio, final_drive_ratio, wheel_radius);
            engine_rpm = expected_rpm.max(minimum_rpm_to_move + 100.0);
        }
    }

    #[test]
    fn main() {
        // Define constants and parameters
        let frontal_area = 2.0; // m^2
        let air_density = 1.225; // kg/m^3
        let rolling_resistance_coefficient = 0.01;
        let car_mass = 1500.0; // kg
        let drag_coefficient = 0.3;
        let final_drive_ratio = 3.5; // Adjust to your car's final drive ratio
        let wheel_radius = 0.3; // meters (adjust to your car's wheel radius)

        // Define torque map (RPM vs. Torque)
        let rpm_values = Array::from_vec(vec![1000.0, 2000.0, 3000.0, 4000.0, 5000.0]);
        let torque_values = Array::from_vec(vec![300.0, 250.0, 200.0, 150.0, 100.0]);

        // Define gear ratios
        let gear_ratios = vec![4.0, 3.0, 2.5, 2.0, 1.5];

        // Integration parameters
        let dt = 0.1; // Time step (seconds)
        let mut speed: f64 = 0.2777778; // Initial speed (m/s)

        // Define maximum speed limit
        let max_speed = 200.0; // Adjust to your car's maximum speed
        let rpm_limit = 6000.0;

        // Start simulation
        let mut time = 0.0;
        let mut engine_rpm = 1000.0; // Initial RPM
        let mut gear_index = 0;

        loop {
            // Calculate engine torque and power based on current RPM and gear
            let engine_torque = interpolate_torque(&rpm_values, &torque_values, engine_rpm);
            let engine_power = (2.0 * std::f64::consts::PI * engine_rpm * engine_torque) / 60_000.0; // kW

            // Calculate resistance forces
            let drag_force = 0.5 * drag_coefficient * frontal_area * air_density * speed.powi(2);
            let rolling_resistance_force = rolling_resistance_coefficient * car_mass * GRAVITY;
            let gravitational_force = car_mass * GRAVITY;

            // Calculate net force
            let net_force = engine_power * 1000.0 / speed - drag_force - rolling_resistance_force - gravitational_force;

            // Calculate acceleration
            let acceleration = net_force / car_mass;

            // Update speed using numerical integration (Euler's method)
            speed += acceleration * dt;

            // Check if engine RPM exceeds redline; if so, shift to the next gear
            if engine_rpm >= rpm_limit {
                gear_index += 1;
            }

            // Check if the net force is negative or if the speed exceeds the maximum limit
            if net_force <= 0.0 || speed >= max_speed || gear_index >= gear_ratios.len() {
                println!("Maximum Speed: {:.2} m/s", speed);
                println!("Time to Reach Maximum Speed: {:.2} seconds", time);
                break;
            }

            // Update time and RPM
            time += dt;
            engine_rpm = speed_to_rpm(speed, gear_ratios[gear_index], final_drive_ratio, wheel_radius);
        }
    }



}

