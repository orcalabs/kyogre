use std::ops::Bound;

use crate::{
    error::PostgresError,
    models::{Haul, HaulsGrid},
    PostgresAdapter,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::HaulsQuery;
use sqlx::postgres::types::PgRange;

use super::bound_float_to_decimal;

impl PostgresAdapter {
    pub(crate) fn hauls_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<impl Stream<Item = Result<Haul, PostgresError>> + '_, PostgresError> {
        let args = HaulsArgs::try_from(query)?;

        let stream = sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.haul_id AS "haul_id!",
    h.ers_activity_id AS "ers_activity_id!",
    h.duration AS "duration!",
    h.haul_distance AS haul_distance,
    h.catch_location_start AS catch_location_start,
    h.ocean_depth_end AS "ocean_depth_end!",
    h.ocean_depth_start AS "ocean_depth_start!",
    h.quota_type_id AS "quota_type_id!",
    h.start_latitude AS "start_latitude!",
    h.start_longitude AS "start_longitude!",
    LOWER(h.period) AS "start_timestamp!",
    UPPER(h.period) AS "stop_timestamp!",
    h.stop_latitude AS "stop_latitude!",
    h.stop_longitude AS "stop_longitude!",
    h.gear_fiskeridir_id AS gear_fiskeridir_id,
    h.gear_group_id AS "gear_group_id: GearGroup",
    h.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    h.vessel_call_sign AS vessel_call_sign,
    h.vessel_call_sign_ers AS "vessel_call_sign_ers!",
    h.vessel_length AS "vessel_length!",
    h.vessel_length_group AS "vessel_length_group!: VesselLengthGroup",
    h.vessel_name AS vessel_name,
    h.vessel_name_ers AS vessel_name_ers,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!"
FROM
    hauls_view h
WHERE
    (
        $1::tstzrange[] IS NULL
        OR h.period && ANY ($1)
    )
    AND (
        $2::VARCHAR[] IS NULL
        OR h.catch_location_start = ANY ($2)
    )
    AND (
        $3::INT[] IS NULL
        OR h.gear_group_id = ANY ($3)
    )
    AND (
        $4::INT[] IS NULL
        OR h.species_group_ids && $4
    )
    AND (
        $5::numrange[] IS NULL
        OR h.vessel_length <@ ANY ($5)
    )
    AND (
        $6::BIGINT[] IS NULL
        OR fiskeridir_vessel_id = ANY ($6)
    )
            "#,
            args.ranges,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.species_group_ids as _,
            args.vessel_length_ranges as _,
            args.fiskeridir_vessel_ids as _,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn hauls_grid_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<HaulsGrid, PostgresError> {
        let args = HaulsArgs::try_from(query)?;

        sqlx::query_as!(
            HaulsGrid,
            r#"
WITH
    hauls AS (
        SELECT
            catch_location_start,
            total_living_weight,
            species_group_ids,
            gear_group_id,
            vessel_length,
            vessel_length_group,
            fiskeridir_vessel_id,
            catches
        FROM
            hauls_view
        WHERE
            catch_location_start IS NOT NULL
            AND (
                $1::tstzrange[] IS NULL
                OR period && ANY ($1)
            )
    )
SELECT
    COALESCE(q1.grid::TEXT, '{}') AS "grid!",
    COALESCE(q1.max_weight, 0)::BIGINT AS "max_weight!",
    COALESCE(q1.min_weight, 0)::BIGINT AS "min_weight!",
    COALESCE(q2.weight_by_gear_group::TEXT, '{}') AS "weight_by_gear_group!",
    COALESCE(q3.weight_by_species_group::TEXT, '{}') AS "weight_by_species_group!",
    COALESCE(q4.weight_by_vessel_length_group::TEXT, '{}') AS "weight_by_vessel_length_group!"
FROM
    (
        SELECT
            JSONB_OBJECT_AGG(h.catch_location_start, h.total_living_weight) AS grid,
            MIN(h.total_living_weight) AS min_weight,
            MAX(h.total_living_weight) AS max_weight
        FROM
            (
                SELECT
                    catch_location_start,
                    COALESCE(SUM(total_living_weight), 0) AS total_living_weight
                FROM
                    hauls
                WHERE
                    (
                        $2::VARCHAR[] IS NULL
                        OR catch_location_start = ANY ($2)
                    )
                    AND (
                        $3::INT[] IS NULL
                        OR gear_group_id = ANY ($3)
                    )
                    AND (
                        $4::INT[] IS NULL
                        OR species_group_ids && $4
                    )
                    AND (
                        $5::numrange[] IS NULL
                        OR vessel_length <@ ANY ($5::numrange[])
                    )
                    AND (
                        $6::BIGINT[] IS NULL
                        OR fiskeridir_vessel_id = ANY ($6)
                    )
                GROUP BY
                    catch_location_start
            ) h
    ) q1,
    (
        SELECT
            JSONB_OBJECT_AGG(h.gear_group_id, h.total_living_weight) AS weight_by_gear_group
        FROM
            (
                SELECT
                    gear_group_id,
                    COALESCE(SUM(total_living_weight), 0) AS total_living_weight
                FROM
                    hauls
                GROUP BY
                    gear_group_id
            ) h
    ) q2,
    (
        SELECT
            JSONB_OBJECT_AGG(h2.species_group_id, h2.total_living_weight) AS weight_by_species_group
        FROM
            (
                SELECT
                    h1.catch['species_group_id']::INT AS species_group_id,
                    COALESCE(SUM(h1.catch['living_weight']::INT), 0) AS total_living_weight
                FROM
                    (
                        SELECT
                            JSONB_ARRAY_ELEMENTS(catches) catch
                        FROM
                            hauls
                    ) h1
                GROUP BY
                    h1.catch['species_group_id']
            ) h2
    ) q3,
    (
        SELECT
            JSONB_OBJECT_AGG(h.vessel_length_group, h.total_living_weight) AS weight_by_vessel_length_group
        FROM
            (
                SELECT
                    vessel_length_group,
                    COALESCE(SUM(total_living_weight), 0) AS total_living_weight
                FROM
                    hauls
                GROUP BY
                    vessel_length_group
            ) h
    ) q4
            "#,
            args.ranges,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.species_group_ids as _,
            args.vessel_length_ranges as _,
            args.fiskeridir_vessel_ids as _,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}

struct HaulsArgs {
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
                        start: Bound::Included(m.start()),
                        end: Bound::Included(m.end()),
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
