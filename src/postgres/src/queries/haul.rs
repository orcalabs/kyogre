use super::bound_float_to_decimal;
use crate::{
    error::{HaulMatrixIndexError, PostgresError},
    models::Haul,
    PostgresAdapter,
};
use bigdecimal::BigDecimal;
use bigdecimal::FromPrimitive;
use chrono::{DateTime, Utc};
use enum_index::EnumIndex;
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use fiskeridir_rs::{Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    date_feature_matrix_size, ActiveHaulsFilter, HaulsMatrixQuery, HaulsQuery,
    ERS_OLDEST_DATA_MONTHS, NUM_CATCH_LOCATIONS,
};
use sqlx::{postgres::types::PgRange, Pool, Postgres};
use strum::EnumCount;

struct MatrixQueryOutput {
    sum_living: i64,
    x_index: i32,
    y_index: i32,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum HaulMatrixFeatures {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
    CatchLocation = 4,
}

impl From<ActiveHaulsFilter> for HaulMatrixFeatures {
    fn from(value: ActiveHaulsFilter) -> Self {
        match value {
            ActiveHaulsFilter::Date => HaulMatrixFeatures::Date,
            ActiveHaulsFilter::GearGroup => HaulMatrixFeatures::GearGroup,
            ActiveHaulsFilter::SpeciesGroup => HaulMatrixFeatures::SpeciesGroup,
            ActiveHaulsFilter::VesselLength => HaulMatrixFeatures::VesselLength,
            ActiveHaulsFilter::CatchLocation => HaulMatrixFeatures::CatchLocation,
        }
    }
}

impl HaulMatrixFeatures {
    fn convert_from_row(&self, val: i32) -> Result<usize, HaulMatrixIndexError> {
        match self {
            HaulMatrixFeatures::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    Err(HaulMatrixIndexError::Date(val))
                }
            }
            HaulMatrixFeatures::GearGroup => GearGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::GearGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::SpeciesGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::VesselLength(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::CatchLocation => Ok(val as usize),
        }
        .into_report()
    }
    fn size(&self) -> usize {
        match self {
            HaulMatrixFeatures::Date => date_feature_matrix_size(),
            HaulMatrixFeatures::GearGroup => GearGroup::COUNT,
            HaulMatrixFeatures::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixFeatures::VesselLength => VesselLengthGroup::COUNT,
            HaulMatrixFeatures::CatchLocation => NUM_CATCH_LOCATIONS,
        }
    }
}

impl PostgresAdapter {
    pub(crate) async fn hauls_matrix_impl(
        pool: Pool<Postgres>,
        args: HaulsMatrixArgs,
        active_filter: ActiveHaulsFilter,
        x_feature: HaulMatrixFeatures,
    ) -> Result<Vec<u64>, PostgresError> {
        let data: Vec<MatrixQueryOutput> = sqlx::query_as!(
            MatrixQueryOutput,
            r#"
SELECT
    CASE
        WHEN $1 = 0 THEN h.matrix_month_bucket
        WHEN $1 = 1 THEN h.gear_group_id
        WHEN $1 = 2 THEN h.species_group_id
        WHEN $1 = 3 THEN h.vessel_length_group
        WHEN $1 = 4 THEN h.catch_location_start_matrix_index
    END AS "x_index!",
    CASE
        WHEN $2 = 0 THEN h.matrix_month_bucket
        WHEN $2 = 1 THEN h.gear_group_id
        WHEN $2 = 2 THEN h.species_group_id
        WHEN $2 = 3 THEN h.vessel_length_group
        WHEN $2 = 4 THEN h.catch_location_start_matrix_index
    END AS "y_index!",
    COALESCE(SUM(living_weight::BIGINT), 0)::BIGINT AS "sum_living!"
FROM
    hauls_matrix_view h
WHERE
    (
        $1 = 0
        OR (
            $1 = 4
            AND $2 = 0
        )
        OR $3::INT[] IS NULL
        OR h.matrix_month_bucket = ANY ($3)
    )
    AND (
        $1 = 4
        OR $4::VARCHAR[] IS NULL
        OR h.catch_location_start = ANY ($4)
    )
    AND (
        $1 = 1
        OR (
            $1 = 4
            AND $2 = 1
        )
        OR $5::INT[] IS NULL
        OR h.gear_group_id = ANY ($5)
    )
    AND (
        $1 = 2
        OR (
            $1 = 4
            AND $2 = 2
        )
        OR $6::INT[] IS NULL
        OR h.species_group_id = ANY ($6)
    )
    AND (
        $1 = 3
        OR (
            $1 = 4
            AND $2 = 3
        )
        OR $7::INT[] IS NULL
        OR h.vessel_length_group = ANY ($7)
    )
    AND (
        $8::BIGINT[] IS NULL
        OR fiskeridir_vessel_id = ANY ($8)
    )
GROUP BY
    1,
    2
            "#,
            x_feature as i32,
            active_filter as i32,
            args.months as _,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.species_group_ids as _,
            args.vessel_length_groups as _,
            args.fiskeridir_vessel_ids as _,
        )
        .fetch_all(&pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        let y_feature = HaulMatrixFeatures::from(active_filter);

        calculate_sum_area_table(x_feature, y_feature, data)
    }
    pub(crate) fn hauls_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<impl Stream<Item = Result<Haul, PostgresError>> + '_, PostgresError> {
        let args = HaulsArgs::try_from(query)?;

        let stream = sqlx::query_as!(
            Haul,
            r#"
SELECT
    MD5(
        e.message_id::TEXT || e.start_timestamp::TEXT || e.stop_timestamp::TEXT
    ) AS "haul_id!",
    e.start_timestamp,
    e.stop_timestamp,
    e.ers_activity_id,
    e.duration AS "duration!",
    e.haul_distance,
    MIN(l.catch_location_id) AS catch_location_start,
    e.ocean_depth_end AS "ocean_depth_end!",
    e.ocean_depth_start AS "ocean_depth_start!",
    e.quota_type_id,
    e.start_latitude AS "start_latitude!",
    e.start_longitude AS "start_longitude!",
    e.stop_latitude AS "stop_latitude!",
    e.stop_longitude AS "stop_longitude!",
    e.gear_id AS "gear_id!: Gear",
    e.gear_group_id AS "gear_group_id!: GearGroup",
    e.fiskeridir_vessel_id,
    e.vessel_call_sign,
    e.vessel_call_sign_ers,
    e.vessel_length,
    TO_VESSEL_LENGTH_GROUP (e.vessel_length) AS "vessel_length_group!: VesselLengthGroup",
    e.vessel_name,
    e.vessel_name_ers,
    JSONB_AGG(
        JSON_BUILD_OBJECT(
            'living_weight',
            c.living_weight,
            'species_fao_id',
            c.species_fao_id,
            'species_fiskeridir_id',
            COALESCE(c.species_fiskeridir_id, 0),
            'species_group_id',
            c.species_group_id,
            'species_main_group_id',
            c.species_main_group_id
        )
    )::TEXT AS "catches!",
    '[]' AS "whale_catches!"
FROM
    ers_dca e
    INNER JOIN ers_dca_catches c ON e.message_id = c.message_id
    AND e.start_timestamp = c.start_timestamp
    AND e.stop_timestamp = c.stop_timestamp
    LEFT JOIN catch_locations l ON ST_CONTAINS (
        l.polygon,
        ST_POINT (e.start_longitude, e.start_latitude)
    )
WHERE
    (
        $1::tstzrange[] IS NULL
        OR TSTZRANGE (e.start_timestamp, e.stop_timestamp, '[]') && ANY ($1)
    )
    AND (
        $2::TEXT[] IS NULL
        OR l.catch_location_id = ANY ($2)
    )
    AND (
        $3::INT[] IS NULL
        OR e.gear_group_id = ANY ($3)
    )
    AND (
        $4::numrange[] IS NULL
        OR e.vessel_length <@ ANY ($4::numrange[])
    )
    AND (
        $5::BIGINT[] IS NULL
        OR e.fiskeridir_vessel_id = ANY ($5)
    )
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp
HAVING
    (
        $6::INT[] IS NULL
        OR ARRAY_AGG(c.species_group_id) && $6
    )
            "#,
            args.ranges,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.vessel_length_ranges as _,
            args.fiskeridir_vessel_ids as _,
            args.species_group_ids as _,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }
}

pub struct HaulsArgs {
    pub ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub catch_locations: Option<Vec<String>>,
    pub gear_group_ids: Option<Vec<i32>>,
    pub species_group_ids: Option<Vec<i32>>,
    pub vessel_length_ranges: Option<Vec<PgRange<BigDecimal>>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
}

impl TryFrom<HaulsQuery> for HaulsArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulsQuery) -> std::result::Result<Self, Self::Error> {
        Ok(HaulsArgs {
            ranges: v.ranges.map(|ranges| {
                ranges
                    .into_iter()
                    .map(|m| PgRange {
                        start: m.start,
                        end: m.end,
                    })
                    .collect()
            }),
            catch_locations: v
                .catch_locations
                .map(|cls| cls.into_iter().map(|c| c.into_inner()).collect()),
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            vessel_length_ranges: v
                .vessel_length_ranges
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|r| {
                            Ok(PgRange {
                                start: bound_float_to_decimal(r.start)?,
                                end: bound_float_to_decimal(r.end)?,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .change_context(PostgresError::DataConversion)?,
            fiskeridir_vessel_ids: v
                .vessel_ids
                .map(|ids| ids.into_iter().map(|i| i.0).collect()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HaulsMatrixArgs {
    pub months: Option<Vec<i32>>,
    pub catch_locations: Option<Vec<String>>,
    pub gear_group_ids: Option<Vec<i32>>,
    pub species_group_ids: Option<Vec<i32>>,
    pub vessel_length_groups: Option<Vec<i32>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
}

impl TryFrom<HaulsMatrixQuery> for HaulsMatrixArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulsMatrixQuery) -> std::result::Result<Self, Self::Error> {
        Ok(HaulsMatrixArgs {
            months: v
                .months
                .map(|months| months.into_iter().map(|m| m as i32).collect()),
            catch_locations: v
                .catch_locations
                .map(|cls| cls.into_iter().map(|c| c.into_inner()).collect()),
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g as i32).collect()),
            vessel_length_groups: v
                .vessel_length_groups
                .map(|groups| groups.into_iter().map(|g| g as i32).collect()),
            fiskeridir_vessel_ids: v
                .vessel_ids
                .map(|ids| ids.into_iter().map(|i| i.0).collect()),
        })
    }
}

fn calculate_sum_area_table(
    x_feature: HaulMatrixFeatures,
    y_feature: HaulMatrixFeatures,
    data: Vec<MatrixQueryOutput>,
) -> Result<Vec<u64>, PostgresError> {
    let height = y_feature.size();
    let width = x_feature.size();

    let mut matrix: Vec<u64> = vec![0; width * height];

    for d in data {
        let x = x_feature
            .convert_from_row(d.x_index)
            .change_context_lazy(|| PostgresError::DataConversion)?;
        let y = y_feature
            .convert_from_row(d.y_index)
            .change_context_lazy(|| PostgresError::DataConversion)?;

        matrix[(y * width) + x] = d.sum_living as u64;
    }

    compute_sum_area_table(&mut matrix, width);

    Ok(matrix)
}

fn compute_sum_area_table(input: &mut [u64], width: usize) {
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
            vec![1, 3, 6, 10, 16, 6, 11, 22, 27, 35, 10, 21, 39, 49, 62, 12, 27, 53, 72, 89],
            input
        );
    }
}
