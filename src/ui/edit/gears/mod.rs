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

mod gear_config;
mod fixed;
mod final_drive;
mod gear_sets;
mod customizable;
mod ratio_set;

use iced::{Alignment, Length};
use iced::alignment::Vertical;
use iced::widget::{Row, Text};
use zephyrj_ac_tools as assetto_corsa;
use assetto_corsa::car::model::GearingCalculator;
pub use gear_config::{gear_configuration_builder,
                      convert_gear_configuration,
                      GearConfig,
                      GearConfiguration,
                      GearUpdateType,
                      GearConfigType};
pub use final_drive::FinalDriveUpdate;
pub use fixed::FixedGearUpdate;
pub use gear_sets::GearsetUpdate;
pub use customizable::CustomizedGearUpdate;
use crate::ui::edit::EditMessage;

fn create_max_ratio_speed_element(ratio: &str, gear_calc: &GearingCalculator) -> Row<'static, EditMessage> {
    let mut speed_row = Row::new()
        .width(Length::Units(50))
        .align_items(Alignment::End);
    match ratio.parse::<f64>() {
        Ok(ratio) => {
            speed_row = speed_row.push(
                Text::new(format!("{}", gear_calc.max_speed_for_ratio(ratio).round()))
                    .vertical_alignment(Vertical::Bottom)
                    .size(12)
            );
            speed_row = speed_row.push(
                Text::new("km/h")
                    .vertical_alignment(Vertical::Bottom)
                    .size(10)
            );
        }
        Err(_) => {
            speed_row = speed_row.push(
                Text::new("n/a")
                    .vertical_alignment(Vertical::Bottom)
                    .size(12)
            );
        }
    };
    speed_row
}
// #[derive(Clone, Copy, Debug)]
// pub struct DeleteButtonStyle;
//
// impl button::StyleSheet for DeleteButtonStyle {
//     type Style = Theme;
//
//     fn active(&self, _style: &Self::Style) -> button::Appearance {
//         button::Appearance {
//             background: Some(iced::Background::Color(iced::Color::from_rgb(0.89,0.15,0.21))),
//             text_color: iced::Color::BLACK,
//             ..Default::default()
//         }
//     }
//     // other methods in Stylesheet have a default impl
// }

// #[derive(Clone, Copy, Debug)]
// pub struct GearStyle;
//
// impl scrollable::StyleSheet for GearStyle {
//     type Style = Theme;
//
//     fn active(&self, _style: &Self::Style) -> scrollable::Appearance {
//         button::Appearance {
//             background: Some(iced::Background::Color(iced::Color::from_rgb(0.89,0.15,0.21))),
//             text_color: iced::Color::BLACK,
//             ..Default::default()
//         }
//     }
//     // other methods in Stylesheet have a default impl
// }