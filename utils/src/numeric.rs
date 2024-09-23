/*
 * Copyright (c):
 * 2024 zephyrj
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

pub fn round_float_to(float: f64, decimal_places: u32) -> f64 {
    let precision_base: u64 = 10;
    let precision_factor = precision_base.pow(decimal_places) as f64;
    (float * precision_factor).round() / precision_factor
}

pub fn round_up_to_nearest_multiple(val: i32, multiple: i32) -> i32 {
    if val < multiple {
        return multiple;
    }
    ((val + (multiple-1)) / multiple) * multiple
}

pub fn is_valid_percentage_str(val: &str) -> bool {
    if val.is_empty() {
        return true;
    }
    match val.parse::<i32>() {
        Ok(v) => is_valid_percentage(v),
        Err(_) => false
    }
}

pub fn is_valid_percentage(val: i32) -> bool {
    if val >= 0 && val <= 100 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::numeric::{is_valid_percentage, round_up_to_nearest_multiple};

    #[test]
    fn round_multiple_tests()  {
        assert_eq!(round_up_to_nearest_multiple(0, 1), 1);
        assert_eq!(round_up_to_nearest_multiple(1, 1), 1);
        assert_eq!(round_up_to_nearest_multiple(2, 1), 2);
        assert_eq!(round_up_to_nearest_multiple(0, 50), 50);
        assert_eq!(round_up_to_nearest_multiple(1, 50), 50);
        assert_eq!(round_up_to_nearest_multiple(10, 50), 50);
        assert_eq!(round_up_to_nearest_multiple(49, 50), 50);
        assert_eq!(round_up_to_nearest_multiple(50, 50), 50);
        assert_eq!(round_up_to_nearest_multiple(51, 50), 100);
        assert_eq!(round_up_to_nearest_multiple(99, 50), 100);
        assert_eq!(round_up_to_nearest_multiple(100, 50), 100);
        assert_eq!(round_up_to_nearest_multiple(101, 50), 150);
    }

    #[test]
    fn valid_percentage_tests()  {
        assert_eq!(is_valid_percentage(-1), false);
        assert_eq!(is_valid_percentage(0), true);
        assert_eq!(is_valid_percentage(1), true);
        assert_eq!(is_valid_percentage(50), true);
        assert_eq!(is_valid_percentage(99), true);
        assert_eq!(is_valid_percentage(100), true);
        assert_eq!(is_valid_percentage(101), false);
        assert_eq!(is_valid_percentage(i32::MAX), false);
        assert_eq!(is_valid_percentage(i32::MIN), false);
    }
}