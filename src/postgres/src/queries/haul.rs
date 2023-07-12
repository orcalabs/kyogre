use super::{bound_float_to_decimal, float_to_decimal};
use crate::{error::PostgresError, models::Haul, models::HaulMessage, PostgresAdapter};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use fiskeridir_rs::{Gear, GearGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::*;
use sqlx::{postgres::types::PgRange, Pool, Postgres};

impl PostgresAdapter {
    pub(crate) async fn hauls_matrix_impl(
        pool: Pool<Postgres>,
        args: HaulsMatrixArgs,
        active_filter: ActiveHaulsFilter,
        x_feature: HaulMatrixXFeature,
    ) -> Result<Vec<u64>, PostgresError> {
        let y_feature = if x_feature == active_filter {
            HaulMatrixYFeature::CatchLocation
        } else {
            HaulMatrixYFeature::from(active_filter)
        };

        let data: Vec<HaulMatrixQueryOutput> = sqlx::query_as!(
            HaulMatrixQueryOutput,
            r#"
SELECT
    CASE
        WHEN $1 = 0 THEN h.matrix_month_bucket
        WHEN $1 = 1 THEN h.gear_group_id
        WHEN $1 = 2 THEN h.species_group_id
        WHEN $1 = 3 THEN h.vessel_length_group
    END AS "x_index!",
    CASE
        WHEN $2 = 0 THEN h.matrix_month_bucket
        WHEN $2 = 1 THEN h.gear_group_id
        WHEN $2 = 2 THEN h.species_group_id
        WHEN $2 = 3 THEN h.vessel_length_group
        WHEN $2 = 4 THEN h.catch_location_matrix_index
    END AS "y_index!",
    COALESCE(SUM(living_weight::BIGINT), 0)::BIGINT AS "sum_living!"
FROM
    hauls_matrix h
WHERE
    (
        $1 = 0
        OR $2 = 0
        OR $3::INT[] IS NULL
        OR h.matrix_month_bucket = ANY ($3)
    )
    AND (
        $2 = 4
        OR $4::VARCHAR[] IS NULL
        OR h.catch_location = ANY ($4)
    )
    AND (
        $1 = 1
        OR $2 = 1
        OR $5::INT[] IS NULL
        OR h.gear_group_id = ANY ($5)
    )
    AND (
        $1 = 2
        OR $2 = 2
        OR $6::INT[] IS NULL
        OR h.species_group_id = ANY ($6)
    )
    AND (
        $1 = 3
        OR $2 = 3
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
            y_feature as i32,
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

        calculate_haul_sum_area_table(x_feature, y_feature, data)
            .change_context(PostgresError::DataConversion)
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
    h.haul_id,
    h.ers_activity_id,
    h.duration,
    h.haul_distance,
    h.catch_location_start,
    h.catch_locations,
    h.ocean_depth_end,
    h.ocean_depth_start,
    h.quota_type_id,
    h.start_latitude,
    h.start_longitude,
    h.start_timestamp,
    h.stop_timestamp,
    h.stop_latitude,
    h.stop_longitude,
    h.total_living_weight,
    h.gear_id AS "gear_id!: Gear",
    h.gear_group_id AS "gear_group_id!: GearGroup",
    h.fiskeridir_vessel_id,
    h.vessel_call_sign,
    h.vessel_call_sign_ers,
    h.vessel_length,
    h.vessel_length_group AS "vessel_length_group!: VesselLengthGroup",
    h.vessel_name,
    h.vessel_name_ers,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!"
FROM
    hauls h
WHERE
    (
        $1::tstzrange[] IS NULL
        OR h.period && ANY ($1)
    )
    AND (
        $2::TEXT[] IS NULL
        OR CASE
            WHEN catch_locations IS NULL THEN h.catch_location_start = ANY ($2)
            ELSE h.catch_locations && $2
        END
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
ORDER BY
    CASE
        WHEN $7 = 1
        AND $8 = 1 THEN start_timestamp
    END ASC,
    CASE
        WHEN $7 = 1
        AND $8 = 2 THEN stop_timestamp
    END ASC,
    CASE
        WHEN $7 = 1
        AND $8 = 3 THEN total_living_weight
    END ASC,
    CASE
        WHEN $7 = 2
        AND $8 = 1 THEN start_timestamp
    END DESC,
    CASE
        WHEN $7 = 2
        AND $8 = 2 THEN stop_timestamp
    END DESC,
    CASE
        WHEN $7 = 2
        AND $8 = 3 THEN total_living_weight
    END DESC
            "#,
            args.ranges,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.species_group_ids as _,
            args.vessel_length_ranges as _,
            args.fiskeridir_vessel_ids as _,
            args.ordering,
            args.sorting,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn haul_messages_of_vessel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>, PostgresError> {
        sqlx::query_as!(
            HaulMessage,
            r#"
SELECT DISTINCT
    h.message_id,
    h.start_timestamp,
    h.stop_timestamp
FROM
    hauls h
    LEFT JOIN hauls_matrix m ON h.message_id = m.message_id
    AND h.start_timestamp = m.start_timestamp
    AND h.stop_timestamp = m.stop_timestamp
WHERE
    m.haul_distributor_id IS NULL
    AND h.total_living_weight > 0
    AND h.fiskeridir_vessel_id = $1
            "#,
            vessel_id.0,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_haul_distribution_output(
        &self,
        values: Vec<HaulDistributionOutput>,
    ) -> Result<(), PostgresError> {
        let len = values.len();

        let mut message_id = Vec::with_capacity(len);
        let mut start_timestamp = Vec::with_capacity(len);
        let mut stop_timestamp = Vec::with_capacity(len);
        let mut catch_location = Vec::with_capacity(len);
        let mut factor = Vec::with_capacity(len);
        let mut distributor_id = Vec::with_capacity(len);

        for v in values {
            message_id.push(v.message_id);
            start_timestamp.push(v.start_timestamp);
            stop_timestamp.push(v.stop_timestamp);
            catch_location.push(v.catch_location.into_inner());
            factor.push(float_to_decimal(v.factor).change_context(PostgresError::DataConversion)?);
            distributor_id.push(v.distributor_id as i32);
        }

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
UPDATE hauls h
SET
    catch_locations = q.catch_locations
FROM
    (
        SELECT
            u.message_id,
            u.start_timestamp,
            u.stop_timestamp,
            ARRAY_AGG(DISTINCT u.catch_location) AS catch_locations
        FROM
            UNNEST(
                $1::BIGINT[],
                $2::TIMESTAMPTZ[],
                $3::TIMESTAMPTZ[],
                $4::TEXT[]
            ) u (
                message_id,
                start_timestamp,
                stop_timestamp,
                catch_location
            )
        GROUP BY
            u.message_id,
            u.start_timestamp,
            u.stop_timestamp
    ) q
WHERE
    h.message_id = q.message_id
    AND h.start_timestamp = q.start_timestamp
    AND h.stop_timestamp = q.stop_timestamp
            "#,
            message_id.as_slice(),
            start_timestamp.as_slice(),
            stop_timestamp.as_slice(),
            catch_location.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
DELETE FROM hauls_matrix h USING UNNEST(
    $1::BIGINT[],
    $2::TIMESTAMPTZ[],
    $3::TIMESTAMPTZ[]
) u (message_id, start_timestamp, stop_timestamp)
WHERE
    h.message_id = u.message_id
    AND h.start_timestamp = u.start_timestamp
    AND h.stop_timestamp = u.stop_timestamp
            "#,
            message_id.as_slice(),
            start_timestamp.as_slice(),
            stop_timestamp.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query!(
            r#"
INSERT INTO
    hauls_matrix (
        message_id,
        start_timestamp,
        stop_timestamp,
        catch_location_matrix_index,
        catch_location,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight,
        haul_distributor_id
    )
SELECT
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp,
    l.matrix_index,
    l.catch_location_id,
    HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp),
    TO_VESSEL_LENGTH_GROUP (e.vessel_length),
    e.fiskeridir_vessel_id,
    e.gear_group_id,
    c.species_group_id,
    SUM(c.living_weight) * MIN(u.factor),
    MIN(u.haul_distributor_id)
FROM
    UNNEST(
        $1::BIGINT[],
        $2::TIMESTAMPTZ[],
        $3::TIMESTAMPTZ[],
        $4::TEXT[],
        $5::DECIMAL[],
        $6::INT[]
    ) u (
        message_id,
        start_timestamp,
        stop_timestamp,
        catch_location,
        factor,
        haul_distributor_id
    )
    INNER JOIN ers_dca e ON e.message_id = u.message_id
    AND e.start_timestamp = u.start_timestamp
    AND e.stop_timestamp = u.stop_timestamp
    INNER JOIN ers_dca_catches c ON e.message_id = c.message_id
    AND e.start_timestamp = c.start_timestamp
    AND e.stop_timestamp = c.stop_timestamp
    INNER JOIN catch_locations l ON u.catch_location = l.catch_location_id
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp,
    c.species_group_id,
    l.catch_location_id
            "#,
            message_id.as_slice(),
            start_timestamp.as_slice(),
            stop_timestamp.as_slice(),
            catch_location.as_slice(),
            factor.as_slice(),
            distributor_id.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}

pub struct HaulsArgs {
    pub ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub catch_locations: Option<Vec<String>>,
    pub gear_group_ids: Option<Vec<i32>>,
    pub species_group_ids: Option<Vec<i32>>,
    pub vessel_length_ranges: Option<Vec<PgRange<BigDecimal>>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
    pub sorting: Option<i32>,
    pub ordering: Option<i32>,
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
            sorting: v.sorting.map(|s| s as i32),
            ordering: v.ordering.map(|o| o as i32),
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
