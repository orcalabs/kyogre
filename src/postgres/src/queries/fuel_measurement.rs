use crate::{PostgresAdapter, error::Result};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    BarentswatchUserId, DateRange, FiskeridirVesselId, FuelMeasurement, FuelMeasurementId,
    FuelMeasurementsQuery, ProcessingStatus, TripOverlappingFuelMeasurement,
};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn overlapping_measurment_fuel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> Result<TripOverlappingFuelMeasurement> {
        let pg_range: PgRange<DateTime<Utc>> = range.into();
        Ok(sqlx::query_as!(
            TripOverlappingFuelMeasurement,
            r#"
SELECT
    COALESCE(
        SUM(
            COMPUTE_TS_RANGE_PERCENT_OVERLAP (fuel_range, $1) * fuel_used_liter
        ),
        0.0
    ) AS "fuel_used_liter!",
    COALESCE(
        COMPUTE_TS_RANGE_MUTLIRANGE_PERCENT_OVERLAP ($1, RANGE_AGG(fuel_range)),
        0.0
    ) * 100 AS "percentage_of_trip_covered_by_measurements!"
FROM
    fuel_measurement_ranges
WHERE
    fuel_range && $1
    AND fiskeridir_vessel_id = $2
    AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (fuel_range, $1) >= 0.5
    AND NOT is_reset
            "#,
            pg_range,
            vessel_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) fn fuel_measurements_impl(
        &self,
        query: FuelMeasurementsQuery,
    ) -> impl Stream<Item = Result<FuelMeasurement>> + '_ {
        let FuelMeasurementsQuery {
            call_sign,
            range,
            limit,
            offset,
        } = query;

        sqlx::query_as!(
            FuelMeasurement,
            r#"
SELECT
    fuel_measurement_id AS "id: FuelMeasurementId ",
    timestamp,
    fuel_liter
FROM
    active_vessels w
    INNER JOIN fuel_measurements f ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    AND call_sign = $1
WHERE
    (
        $2::TIMESTAMPTZ IS NULL
        OR timestamp >= $2
    )
    AND (
        $3::TIMESTAMPTZ IS NULL
        OR timestamp <= $3
    )
ORDER BY
    timestamp DESC
LIMIT
    $4
OFFSET
    $5
            "#,
            call_sign.into_inner(),
            range.start(),
            range.end(),
            limit.map(|v| v as i64),
            offset.map(|v| v as i64),
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn update_fuel_measurement_impl(
        &self,
        measurement: &kyogre_core::FuelMeasurement,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let old_record = sqlx::query!(
            r#"
WITH
    to_delete AS (
        SELECT
            w.fiskeridir_vessel_id,
            $1 AS barentswatch_user_id,
            $2::TIMESTAMPTZ AS new_timestamp,
            f.timestamp AS old_timestamp,
            f.fuel_measurement_id
        FROM
            active_vessels w
            INNER JOIN fuel_measurements f ON f.fuel_measurement_id = $3
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
        WHERE
            w.call_sign = $4
    ),
    deleted_ranges AS (
        DELETE FROM fuel_measurement_ranges r USING to_delete t
        WHERE
            r.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND (
                r.fuel_range @> t.old_timestamp
                OR r.fuel_range @> t.new_timestamp
            )
        RETURNING
            r.fuel_range,
            r.fiskeridir_vessel_id,
            t.old_timestamp,
            r.fuel_range @> t.new_timestamp AS covered_delete
    ),
    updated_trips AS (
        UPDATE trips_detailed t
        SET
            benchmark_status = $5
        FROM
            deleted_ranges
        WHERE
            deleted_ranges.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND deleted_ranges.fuel_range && t.period
    )
SELECT
    d.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    d.old_timestamp AS "timestamp!"
FROM
    deleted_ranges d
WHERE
    NOT d.covered_delete
            "#,
            user_id as BarentswatchUserId,
            measurement.timestamp,
            measurement.id as FuelMeasurementId,
            call_sign.as_ref(),
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_optional(&mut *tx)
        .await?;

        let updated = sqlx::query!(
            r#"
UPDATE fuel_measurements f
SET
    fuel_liter = $1,
    barentswatch_user_id = $2,
    timestamp = $3
FROM
    active_vessels w
WHERE
    f.fuel_measurement_id = $4
    AND w.call_sign = $5
    AND w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
RETURNING
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    f.timestamp,
    f.fuel_liter
            "#,
            measurement.fuel_liter,
            user_id as BarentswatchUserId,
            measurement.timestamp,
            measurement.id as FuelMeasurementId,
            call_sign.as_ref(),
        )
        .fetch_one(&mut *tx)
        .await?;

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            updated.fiskeridir_vessel_id,
            updated.timestamp,
            updated.fuel_liter,
            &mut tx,
        )
        .await?;

        if let Some(old_record) = old_record {
            self.add_fuel_measurement_ranges_post_measurement_deletion(
                old_record.fiskeridir_vessel_id,
                old_record.timestamp,
                &mut tx,
            )
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_fuel_measurements_impl(
        &self,
        measurements: &kyogre_core::CreateFuelMeasurement,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<kyogre_core::FuelMeasurement> {
        let mut tx = self.pool.begin().await?;

        let out = self
            .add_fuel_measurements_tx(measurements, call_sign, user_id, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(out)
    }

    pub(crate) async fn add_fuel_measurements_tx(
        &self,
        measurement: &kyogre_core::CreateFuelMeasurement,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<kyogre_core::FuelMeasurement> {
        self.assert_call_sign_exists(call_sign, &mut **tx).await?;

        #[derive(Debug)]
        struct Intermediate {
            id: FuelMeasurementId,
            fiskeridir_vessel_id: FiskeridirVesselId,
            timestamp: DateTime<Utc>,
            fuel_liter: f64,
        }

        let measurement = sqlx::query_as!(
            Intermediate,
            r#"
WITH
    inserted AS (
        INSERT INTO
            fuel_measurements (
                fiskeridir_vessel_id,
                barentswatch_user_id,
                timestamp,
                fuel_liter
            )
        SELECT
            f.fiskeridir_vessel_id,
            $2,
            $3,
            $4
        FROM
            active_vessels f
        WHERE
            f.call_sign = $1
        ON CONFLICT (fiskeridir_vessel_id, timestamp) DO NOTHING
        RETURNING
            fuel_measurement_id,
            fiskeridir_vessel_id,
            timestamp,
            fuel_liter
    ),
    deleted AS (
        DELETE FROM fuel_measurement_ranges r USING inserted
        WHERE
            fuel_range @> inserted.timestamp
            AND r.fiskeridir_vessel_id = inserted.fiskeridir_vessel_id
        RETURNING
            r.fiskeridir_vessel_id,
            r.fuel_range
    ),
    invalidated_trips AS (
        UPDATE trips_detailed t
        SET
            benchmark_status = $5
        FROM
            deleted
        WHERE
            deleted.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND deleted.fuel_range && t.period
    )
SELECT
    fuel_measurement_id AS "id: FuelMeasurementId",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    timestamp,
    fuel_liter
FROM
    inserted
            "#,
            call_sign.as_ref(),
            user_id.as_ref(),
            measurement.timestamp,
            measurement.fuel_liter,
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_one(&mut **tx)
        .await?;

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            measurement.fiskeridir_vessel_id,
            measurement.timestamp,
            measurement.fuel_liter,
            &mut *tx,
        )
        .await?;

        Ok(kyogre_core::FuelMeasurement {
            id: measurement.id,
            timestamp: measurement.timestamp,
            fuel_liter: measurement.fuel_liter,
        })
    }

    pub(crate) async fn delete_fuel_measurement_impl(
        &self,
        measurement: &kyogre_core::DeleteFuelMeasurement,
        call_sign: &CallSign,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let record = sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            w.fiskeridir_vessel_id,
            f.timestamp
        FROM
            active_vessels w
            INNER JOIN fuel_measurements f ON f.fuel_measurement_id = $1
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
        WHERE
            w.call_sign = $2
    ),
    updated_trips AS (
        UPDATE trips_detailed t
        SET
            benchmark_status = $3
        FROM
            fuel_measurement_ranges r
            INNER JOIN input i ON r.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND (
                r.start_measurement_ts = i.timestamp
                OR r.end_measurement_ts = i.timestamp
            )
        WHERE
            r.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND r.fuel_range && t.period
    )
DELETE FROM fuel_measurements f USING input i
WHERE
    f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
    AND f.timestamp = i.timestamp
RETURNING
    f.timestamp,
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId"
            "#,
            measurement.id as FuelMeasurementId,
            call_sign.as_ref(),
            ProcessingStatus::Unprocessed as i32,
        )
        .fetch_one(&mut *tx)
        .await?;

        self.add_fuel_measurement_ranges_post_measurement_deletion(
            record.fiskeridir_vessel_id,
            record.timestamp,
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn add_fuel_measurement_ranges_post_measurement_deletion(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    top AS (
        SELECT
            timestamp,
            fuel_liter
        FROM
            fuel_measurements f
        WHERE
            fiskeridir_vessel_id = $1
            AND timestamp > $2
        ORDER BY
            timestamp
        LIMIT
            1
    ),
    bottom AS (
        SELECT
            timestamp,
            fuel_liter
        FROM
            fuel_measurements f
        WHERE
            fiskeridir_vessel_id = $1
            AND timestamp < $2
        ORDER BY
            timestamp DESC
        LIMIT
            1
    )
INSERT INTO
    fuel_measurement_ranges (
        fiskeridir_vessel_id,
        start_measurement_ts,
        start_measurement_fuel_liter,
        end_measurement_ts,
        end_measurement_fuel_liter
    )
SELECT
    $1,
    b.timestamp,
    b.fuel_liter,
    t.timestamp,
    t.fuel_liter
FROM
    top t
    INNER JOIN bottom b ON TRUE
    --! This only occurs if 'add_fuel_measurement_ranges_post_measurement_insertion' is called prior to this method
    --! then both will try to add the same fuel_measurement range
ON CONFLICT (fiskeridir_vessel_id, fuel_range) DO NOTHING
            "#,
            vessel_id as FiskeridirVesselId,
            timestamp
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn add_fuel_measurement_ranges_post_measurement_insertion(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        fuel_liter: f64,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    top AS (
        SELECT
            timestamp,
            fuel_liter
        FROM
            fuel_measurements f
        WHERE
            fiskeridir_vessel_id = $1
            AND timestamp > $2
        ORDER BY
            timestamp
        LIMIT
            1
    ),
    bottom AS (
        SELECT
            timestamp,
            fuel_liter
        FROM
            fuel_measurements f
        WHERE
            fiskeridir_vessel_id = $1
            AND timestamp < $2
        ORDER BY
            timestamp DESC
        LIMIT
            1
    ),
    inserted AS (
        INSERT INTO
            fuel_measurement_ranges (
                fiskeridir_vessel_id,
                start_measurement_ts,
                start_measurement_fuel_liter,
                end_measurement_ts,
                end_measurement_fuel_liter
            )
        SELECT
            *
        FROM
            (
                SELECT
                    $1,
                    b.timestamp,
                    b.fuel_liter,
                    $2,
                    $3
                FROM
                    bottom b
                UNION
                SELECT
                    $1,
                    $2,
                    $3,
                    t.timestamp,
                    t.fuel_liter
                FROM
                    top t
            )
        RETURNING
            fiskeridir_vessel_id,
            fuel_range
    )
UPDATE trips_detailed t
SET
    benchmark_status = $4
FROM
    inserted
WHERE
    inserted.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND inserted.fuel_range && t.period
            "#,
            vessel_id as FiskeridirVesselId,
            timestamp,
            fuel_liter,
            ProcessingStatus::Unprocessed as i32
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
