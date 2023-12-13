/*
 * Copyright (c):
 * 2023 zephyrj
 * zephyrj@protonmail.com
 *
 * This file is part of engine-crane.
 *
 * engine-crane is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * engine-crane is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with engine-crane. If not, see <https://www.gnu.org/licenses/>.
 */

use plotters::chart::{ChartBuilder, LabelAreaPosition};
use plotters::drawing::IntoDrawingArea;
use plotters::element::PathElement;
use plotters::prelude::{BitMapBackend, BLACK, CYAN, FontDesc, FontFamily, FontStyle, LineSeries, WHITE};
use plotters::style::{Color, TextStyle, YELLOW};
use assetto_corsa::car::model::GearingCalculator;
use assetto_corsa::{Car, Installation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let car_folder_name = "tatuusfa1";
    let ac_install = Installation::new();
    let car_folder_root = ac_install.get_installed_car_path();
    let car_folder_path = car_folder_root.join(car_folder_name);
    let mut car = Car::load_from_path(&car_folder_path).map_err(|err| err.to_string())?;
    let gear_calc = GearingCalculator::from_car(&mut car)?;

    let mut engine_rpm = gear_calc.min_rpm();
    let mut current_gear_idx = 0;
    let rpm_limit = gear_calc.max_rpm();
    let mut data_point_vec: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current_gear_vec: Vec<(f64, f64)> = Vec::new();
    let mut max_speed: f64 = 0.0;

    loop {
        if engine_rpm > rpm_limit {
            current_gear_idx += 1;
            engine_rpm = gear_calc.min_rpm();
            max_speed = max_speed.max(current_gear_vec.last().unwrap().0);
            data_point_vec.push(current_gear_vec);
            current_gear_vec = Vec::new();
            if current_gear_idx > gear_calc.max_gear_idx() {
                break;
            }
        }

        let speed = gear_calc.engine_rpm_to_wheel_speed(engine_rpm, current_gear_idx);
        current_gear_vec.push((speed * 3.6, engine_rpm as f64));
        engine_rpm += 100;
    }

    let x_axis_limit = max_speed + 10f64;
    let y_axis_start = 0f64.max((gear_calc.min_rpm()-500) as f64);
    let y_axis_limit = (rpm_limit+1000) as f64;
    let x = FontDesc::new(FontFamily::Name("Arial"), 20.0, FontStyle::Normal);

    let background_colour = BLACK.mix(0.9);
    let filename = format!("{}-ratios.png", car_folder_name);
    let root =
        BitMapBackend::new(&filename, (800, 600)).into_drawing_area();
    root.fill(&background_colour)?;
    let mut context = ChartBuilder::on(&root)
        .margin(15)
        .caption("Ratios Graph", x.color(&WHITE))
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .set_label_area_size(LabelAreaPosition::Left, 50)
        .build_cartesian_2d(0f64..x_axis_limit,
                            y_axis_start..y_axis_limit)?;

    // Create lines
    context
        .configure_mesh()
        .x_label_formatter(&as_usize)
        .x_desc("Speed (Km/H)")
        .y_label_formatter(&as_usize)
        .y_desc("Engine RPM")
        .label_style(&WHITE)
        .bold_line_style(&WHITE.mix(0.2))
        .light_line_style(&WHITE.mix(0.1))
        .draw()?;

    for (idx, data) in data_point_vec.into_iter().enumerate() {
        if data.is_empty() {
            continue;
        }
        let max_gear_speed = data.last().unwrap().0 as usize;
        let mut last_point = context.backend_coord(&data.last().unwrap());
        last_point.0 += 5;
        last_point.1 -= 5;
        let mut series_anno = context.draw_series(LineSeries::new(data, &YELLOW))?;
        series_anno = series_anno.label(format!("Gear {}", idx + 1));
        series_anno = series_anno.legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &YELLOW));
        let mut text_style = TextStyle::from(("sans-serif", 12));
        text_style = text_style.color(&WHITE);
        root.draw_text(&*max_gear_speed.to_string(),
                       &text_style,
                       last_point);
    }

    // Configure the legend
    // context
    //     //.configure_series_labels()
    //     //.label_font(&WHITE)
    //     //.background_style(&BLACK.mix(0.8))
    //     //.border_style(&WHITE)
    //     .draw()?;

    Ok(())
}

fn as_usize(x: &f64) -> String {
    format!("{}", *x as usize)
}
