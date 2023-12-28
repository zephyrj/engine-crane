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

#[cfg(test)]
mod tests {
    use crate::numeric::round_up_to_nearest_multiple;

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
}