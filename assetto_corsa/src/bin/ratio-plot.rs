/*
 * Copyright (c):
 * 2025 zephyrj
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
use plotters::prelude::{BLACK, FontDesc, FontFamily, FontStyle, LineSeries, SVGBackend, WHITE};
use plotters::style::{Color, TextStyle, YELLOW};
use assetto_corsa::car::model::GearingCalculator;
use assetto_corsa::{Car, Installation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let car_folder_name = "ferrari_458";
    let ac_install = Installation::new();
    let car_folder_root = ac_install.get_installed_car_path();
    let car_folder_path = car_folder_root.join(car_folder_name);
    let mut car = Car::load_from_path(&car_folder_path).map_err(|err| err.to_string())?;
    let gear_calc = GearingCalculator::from_car(&mut car)?;

    let rpm_limit = gear_calc.max_rpm();
    let mut data_point_vec: Vec<Vec<(f64, f64)>> = gear_calc.calculate_speed_plot(Some(100));
    let mut max_speed: f64 = gear_calc.max_speed();

    let x_axis_limit = max_speed + 10f64;
    let y_axis_start = 0f64.max((gear_calc.min_rpm()-500) as f64);
    let y_axis_limit = (rpm_limit+1000) as f64;
    let font_desc = FontDesc::new(FontFamily::Name("sans-serif"), 20.0, FontStyle::Normal);

    let background_colour = BLACK.mix(0.9);
    let filename = format!("{}-ratios.svg", car_folder_name);
    let root =
        SVGBackend::new(&filename, (800, 600)).into_drawing_area();
    root.fill(&background_colour)?;
    let mut context = ChartBuilder::on(&root)
        .margin(15)
        .caption("Ratios Graph", font_desc.color(&WHITE))
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .set_label_area_size(LabelAreaPosition::Left, 50)
        .build_cartesian_2d(0f64..x_axis_limit,
                            y_axis_start..y_axis_limit)?;

    // Create lines
    context
        .configure_mesh()
        .x_label_formatter(&as_usize)
        .x_desc("Speed (KM/H)")
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
        let mut text_coord = context.backend_coord(&data.last().unwrap());
        text_coord.0 += 5;
        text_coord.1 -= 5;
        let mut series_anno = context.draw_series(LineSeries::new(data, &YELLOW))?;
        series_anno = series_anno.label(format!("Gear {}", idx + 1));
        series_anno = series_anno.legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &YELLOW));
        let mut text_style = TextStyle::from(("sans-serif", 12));
        text_style = text_style.color(&WHITE);
        let _ = root.draw_text(&*max_gear_speed.to_string(), &text_style, text_coord);
    }
    Ok(())
}

fn as_usize(x: &f64) -> String {
    format!("{}", *x as usize)
}
