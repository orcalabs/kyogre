use std::ops::{Add, AddAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

mod fishing_facility;
mod fuel;
mod haul;
mod landing;
mod ocean_climate;
mod pagination;
mod trip;
mod trip_benchmark;
mod vessel_benchmark;
mod weather;

pub use fishing_facility::*;
pub use fuel::*;
pub use haul::*;
pub use landing::*;
pub use ocean_climate::*;
pub use pagination::*;
use strum::{AsRefStr, EnumString};
pub use trip::*;
pub use trip_benchmark::*;
pub use vessel_benchmark::*;
pub use weather::*;

#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(
    Deserialize, Serialize, Debug, Default, Clone, Copy, strum::Display, AsRefStr, EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum Ordering {
    #[serde(alias = "asc", alias = "Asc", alias = "ascending", alias = "Ascending")]
    Asc = 1,
    #[serde(
        alias = "desc",
        alias = "Desc",
        alias = "Descending",
        alias = "descending"
    )]
    #[default]
    Desc = 2,
}

pub fn compute_sum_area_table<T: Add + Sub + AddAssign + SubAssign + Copy>(
    input: &mut [T],
    width: usize,
) {
    let mut i = 0;

    while i < input.len() {
        let mut sum = input[i];

        let y = i / width;
        let x = i % width;

        if y > 0 {
            let idx = (width * (y - 1)) + x;
            sum += input[idx];
        }
        if x > 0 {
            let idx = (width * y) + (x - 1);
            sum += input[idx];
        }
        if x > 0 && y > 0 {
            let idx = (width * (y - 1)) + (x - 1);
            sum -= input[idx];
        }
        input[i] = sum;

        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_area_table() {
        let mut input = vec![1, 2, 3, 4, 6, 5, 3, 8, 1, 2, 4, 6, 7, 5, 5, 2, 4, 8, 9, 4];
        compute_sum_area_table(&mut input, 5);
        assert_eq!(
            vec![
                1, 3, 6, 10, 16, 6, 11, 22, 27, 35, 10, 21, 39, 49, 62, 12, 27, 53, 72, 89
            ],
            input
        );
    }
}
