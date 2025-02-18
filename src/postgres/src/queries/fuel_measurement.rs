use crate::{error::Result, PostgresAdapter};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use itertools::MultiUnzip;
use kyogre_core::{
    BarentswatchUserId, DateRange, FiskeridirVesselId, FuelMeasurement, FuelMeasurementId,
    FuelMeasurementsQuery, ProcessingStatus,
};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn overlapping_measurment_fuel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> Result<f64> {
        let pg_range: PgRange<DateTime<Utc>> = range.into();
        Ok(sqlx::query!(
            r#"
SELECT
    COALESCE(
        SUM(
            COMPUTE_TS_RANGE_PERCENT_OVERLAP (fuel_range, $1) * fuel_used_liter
        ),
        0.0
    ) AS "estimate!"
FROM
    fuel_measurement_ranges
WHERE
    fuel_range && $1
    AND fiskeridir_vessel_id = $2
    AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (fuel_range, $1) >= 0.5
            "#,
            pg_range,
            vessel_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?
        .estimate)
    }

    pub(crate) fn fuel_measurements_impl(
        &self,
        query: FuelMeasurementsQuery,
    ) -> impl Stream<Item = Result<FuelMeasurement>> + '_ {
        sqlx::query_as!(
            FuelMeasurement,
            r#"
SELECT
    fuel_measurement_id AS "id: FuelMeasurementId ",
    timestamp,
    fuel_liter,
    fuel_after_liter
FROM
    fiskeridir_ais_vessel_mapping_whitelist w
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
            "#,
            query.call_sign.into_inner(),
            query.start_date,
            query.end_date,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn update_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::FuelMeasurement],
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<()> {
        let mut fuel = Vec::with_capacity(measurements.len());
        let mut call_signs = Vec::with_capacity(measurements.len());
        let mut user_ids = Vec::with_capacity(measurements.len());
        let mut timestamp = Vec::with_capacity(measurements.len());
        let mut id = Vec::with_capacity(measurements.len());
        let mut fuel_after = Vec::with_capacity(measurements.len());
        for m in measurements {
            fuel.push(m.fuel_liter);
            call_signs.push(call_sign.as_ref());
            user_ids.push(user_id);
            timestamp.push(m.timestamp);
            id.push(m.id);
            fuel_after.push(m.fuel_after_liter);
        }

        let mut tx = self.pool.begin().await?;
        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let (old_vessel_ids, old_timestamps): (Vec<_>, Vec<_>) = sqlx::query!(
            r#"
WITH
    to_delete AS (
        SELECT
            w.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp AS new_timestamp,
            f.timestamp AS old_timestamp,
            f.fuel_measurement_id
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID[],
                $3::TIMESTAMPTZ[],
                $4::BIGINT[]
            ) u (call_sign, barentswatch_user_id, timestamp, id)
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist w ON w.call_sign = u.call_sign
            INNER JOIN fuel_measurements f ON u.id = f.fuel_measurement_id
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
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
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &id as &[FuelMeasurementId],
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| (r.fiskeridir_vessel_id, r.timestamp))
        .unzip();

        let (updated_vessel_ids, updated_timestamps, updated_fuel, updated_fuel_after): (
            Vec<_>,
            Vec<_>,
            Vec<_>,
            Vec<_>,
        ) = sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            w.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp,
            u.fuel_liter,
            f.fuel_measurement_id,
            u.fuel_after_liter
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID[],
                $3::TIMESTAMPTZ[],
                $4::DOUBLE PRECISION[],
                $5::BIGINT[],
                $6::DOUBLE PRECISION[]
            ) u (
                call_sign,
                barentswatch_user_id,
                timestamp,
                fuel_liter,
                id,
                fuel_after_liter
            )
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist w ON w.call_sign = u.call_sign
            INNER JOIN fuel_measurements f ON u.id = f.fuel_measurement_id
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    )
UPDATE fuel_measurements f
SET
    fuel_liter = input.fuel_liter,
    barentswatch_user_id = input.barentswatch_user_id,
    timestamp = input.timestamp,
    fuel_after_liter = input.fuel_after_liter
FROM
    input
WHERE
    f.fuel_measurement_id = input.fuel_measurement_id
RETURNING
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    f.timestamp,
    f.fuel_liter,
    f.fuel_after_liter
            "#,
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &fuel,
            &id as &[FuelMeasurementId],
            &fuel_after as &[Option<f64>],
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| {
            (
                r.fiskeridir_vessel_id,
                r.timestamp,
                r.fuel_liter,
                r.fuel_after_liter,
            )
        })
        .multiunzip();

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            &updated_vessel_ids,
            &updated_timestamps,
            &updated_fuel,
            &updated_fuel_after,
            &mut tx,
        )
        .await?;

        self.add_fuel_measurement_ranges_post_measurement_deletion(
            &old_vessel_ids,
            &old_timestamps,
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::CreateFuelMeasurement],
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<Vec<kyogre_core::FuelMeasurement>> {
        let mut fuel = Vec::with_capacity(measurements.len());
        let mut call_signs = Vec::with_capacity(measurements.len());
        let mut user_ids = Vec::with_capacity(measurements.len());
        let mut timestamp = Vec::with_capacity(measurements.len());
        let mut fuel_after = Vec::with_capacity(measurements.len());
        for m in measurements {
            fuel.push(m.fuel_liter);
            call_signs.push(call_sign.as_ref());
            user_ids.push(user_id);
            timestamp.push(m.timestamp);
            fuel_after.push(m.fuel_after_liter);
        }

        let mut tx = self.pool.begin().await?;

        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        #[derive(Debug)]
        struct Intermediate {
            id: FuelMeasurementId,
            fiskeridir_vessel_id: FiskeridirVesselId,
            timestamp: DateTime<Utc>,
            fuel_liter: f64,
            fuel_after_liter: Option<f64>,
        }

        let measurements = sqlx::query_as!(
            Intermediate,
            r#"
WITH
    inserted AS (
        INSERT INTO
            fuel_measurements (
                fiskeridir_vessel_id,
                barentswatch_user_id,
                timestamp,
                fuel_liter,
                fuel_after_liter
            )
        SELECT
            f.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp,
            u.fuel_liter,
            u.fuel_after_liter
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID[],
                $3::TIMESTAMPTZ[],
                $4::DOUBLE PRECISION[],
                $5::DOUBLE PRECISION[]
            ) u (
                call_sign,
                barentswatch_user_id,
                timestamp,
                fuel_liter,
                fuel_after_liter
            )
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist f ON f.call_sign = u.call_sign
        ON CONFLICT (fiskeridir_vessel_id, timestamp) DO NOTHING
        RETURNING
            fuel_measurement_id,
            fiskeridir_vessel_id,
            timestamp,
            fuel_liter,
            fuel_after_liter
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
            benchmark_status = $6
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
    fuel_liter,
    fuel_after_liter
FROM
    inserted
            "#,
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &fuel,
            &fuel_after as &[Option<f64>],
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut vessel_ids = Vec::with_capacity(measurements.len());
        let mut ts = Vec::with_capacity(measurements.len());
        let mut fuel = Vec::with_capacity(measurements.len());
        let mut fuel_after = Vec::with_capacity(measurements.len());
        for m in &measurements {
            vessel_ids.push(m.fiskeridir_vessel_id);
            ts.push(m.timestamp);
            fuel.push(m.fuel_liter);
            fuel_after.push(m.fuel_after_liter);
        }

        let out = measurements
            .into_iter()
            .map(|m| kyogre_core::FuelMeasurement {
                id: m.id,
                timestamp: m.timestamp,
                fuel_liter: m.fuel_liter,
                fuel_after_liter: m.fuel_after_liter,
            })
            .collect();

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            &vessel_ids,
            &ts,
            &fuel,
            &fuel_after,
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(out)
    }

    pub(crate) async fn delete_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::DeleteFuelMeasurement],
        call_sign: &CallSign,
    ) -> Result<()> {
        let mut call_signs = Vec::with_capacity(measurements.len());
        let mut id = Vec::with_capacity(measurements.len());
        for m in measurements {
            call_signs.push(call_sign.as_ref());
            id.push(m.id);
        }
        let mut tx = self.pool.begin().await?;
        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let (ts, vessel_ids): (Vec<_>, Vec<_>) = sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            w.fiskeridir_vessel_id,
            f.timestamp
        FROM
            UNNEST($1::TEXT[], $2::BIGINT[]) u (call_sign, id)
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist w ON w.call_sign = u.call_sign
            INNER JOIN fuel_measurements f ON u.id = f.fuel_measurement_id
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    ),
    ranges AS (
        SELECT
            r.fiskeridir_vessel_id,
            r.fuel_range
        FROM
            fuel_measurement_ranges r
            INNER JOIN input i ON r.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND (
                r.start_measurement_ts = i.timestamp
                OR r.end_measurement_ts = i.timestamp
            )
    ),
    updated_trips AS (
        UPDATE trips_detailed t
        SET
            benchmark_status = $3
        FROM
            ranges
        WHERE
            ranges.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND ranges.fuel_range && t.period
    )
DELETE FROM fuel_measurements f USING input i
WHERE
    f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
    AND f.timestamp = i.timestamp
RETURNING
    f.timestamp,
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId"
            "#,
            &call_signs as &[&str],
            &id as &[FuelMeasurementId],
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|v| (v.timestamp, v.fiskeridir_vessel_id))
        .unzip();

        self.add_fuel_measurement_ranges_post_measurement_deletion(&vessel_ids, &ts, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn add_fuel_measurement_ranges_post_measurement_deletion(
        &self,
        vessel_ids: &[FiskeridirVesselId],
        timestamps: &[DateTime<Utc>],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            UNNEST($1::BIGINT[]) fiskeridir_vessel_id,
            UNNEST($2::TIMESTAMPTZ[]) timestamp
    ),
    top AS (
        SELECT DISTINCT
            ON (i.fiskeridir_vessel_id, i.timestamp) i.fiskeridir_vessel_id AS fiskeridir_vessel_id,
            i.timestamp AS deleted_timestamp,
            f.timestamp AS end_ts,
            f.fuel_liter AS end_fuel_liter
        FROM
            fuel_measurements f
            INNER JOIN input i ON f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND f.timestamp > i.timestamp
        ORDER BY
            i.fiskeridir_vessel_id,
            i.timestamp,
            f.timestamp ASC
    ),
    bottom AS (
        SELECT DISTINCT
            ON (i.fiskeridir_vessel_id, i.timestamp) i.fiskeridir_vessel_id AS fiskeridir_vessel_id,
            i.timestamp AS deleted_timestamp,
            f.timestamp AS start_ts,
            f.fuel_liter AS start_fuel_liter,
            f.fuel_after_liter AS start_fuel_after_liter
        FROM
            fuel_measurements f
            INNER JOIN input i ON f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND f.timestamp < i.timestamp
        ORDER BY
            i.fiskeridir_vessel_id,
            i.timestamp,
            f.timestamp DESC
    )
INSERT INTO
    fuel_measurement_ranges (
        fiskeridir_vessel_id,
        start_measurement_ts,
        start_measurement_fuel_liter,
        start_measurement_fuel_after_liter,
        end_measurement_ts,
        end_measurement_fuel_liter
    )
SELECT
    t.fiskeridir_vessel_id,
    b.start_ts,
    b.start_fuel_liter,
    b.start_fuel_after_liter,
    t.end_ts,
    t.end_fuel_liter
FROM
    top t
    INNER JOIN bottom b ON t.fiskeridir_vessel_id = b.fiskeridir_vessel_id
    AND t.deleted_timestamp = b.deleted_timestamp
WHERE
    COMPUTE_FUEL_USED (
        b.start_fuel_liter,
        b.start_fuel_after_liter,
        t.end_fuel_liter
    ) > 0.0
    --! This only occurs if 'add_fuel_measurement_ranges_post_measurement_insertion' is called prior to this method
    --! then both will try to add the same fuel_measurement range
ON CONFLICT (fiskeridir_vessel_id, fuel_range) DO NOTHING
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &timestamps
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn add_fuel_measurement_ranges_post_measurement_insertion(
        &self,
        vessel_ids: &[FiskeridirVesselId],
        timestamps: &[DateTime<Utc>],
        fuel_liter: &[f64],
        fuel_after_liter: &[Option<f64>],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            UNNEST($1::BIGINT[]) fiskeridir_vessel_id,
            UNNEST($2::DOUBLE PRECISION[]) fuel_liter,
            UNNEST($3::TIMESTAMPTZ[]) timestamp,
            UNNEST($4::DOUBLE PRECISION[]) fuel_after_liter
    ),
    top AS (
        SELECT DISTINCT
            ON (i.fiskeridir_vessel_id, i.timestamp) i.fiskeridir_vessel_id AS fiskeridir_vessel_id,
            i.timestamp AS start_ts,
            i.fuel_liter AS start_fuel_liter,
            i.fuel_after_liter AS start_fuel_after_liter,
            f.timestamp AS end_ts,
            f.fuel_liter AS end_fuel_liter
        FROM
            fuel_measurements f
            INNER JOIN input i ON f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND f.timestamp > i.timestamp
        ORDER BY
            i.fiskeridir_vessel_id,
            i.timestamp,
            f.timestamp ASC
    ),
    bottom AS (
        SELECT DISTINCT
            ON (i.fiskeridir_vessel_id, i.timestamp) i.fiskeridir_vessel_id AS fiskeridir_vessel_id,
            f.timestamp AS start_ts,
            f.fuel_liter AS start_fuel_liter,
            f.fuel_after_liter AS start_fuel_after_liter,
            i.timestamp AS end_ts,
            i.fuel_liter AS end_fuel_liter
        FROM
            fuel_measurements f
            INNER JOIN input i ON f.fiskeridir_vessel_id = i.fiskeridir_vessel_id
            AND f.timestamp < i.timestamp
        ORDER BY
            i.fiskeridir_vessel_id,
            i.timestamp,
            f.timestamp DESC
    ),
    inserted AS (
        INSERT INTO
            fuel_measurement_ranges (
                fiskeridir_vessel_id,
                start_measurement_ts,
                start_measurement_fuel_liter,
                start_measurement_fuel_after_liter,
                end_measurement_ts,
                end_measurement_fuel_liter
            )
        SELECT
            *
        FROM
            (
                SELECT
                    b.fiskeridir_vessel_id,
                    b.start_ts,
                    b.start_fuel_liter,
                    b.start_fuel_after_liter,
                    b.end_ts,
                    b.end_fuel_liter
                FROM
                    bottom b
                UNION
                SELECT
                    t.fiskeridir_vessel_id,
                    t.start_ts,
                    t.start_fuel_liter,
                    t.start_fuel_after_liter,
                    t.end_ts,
                    t.end_fuel_liter
                FROM
                    top t
            ) q
        WHERE
            COMPUTE_FUEL_USED (
                q.start_fuel_liter,
                q.start_fuel_after_liter,
                q.end_fuel_liter
            ) > 0.0
        RETURNING
            fiskeridir_vessel_id,
            fuel_range
    )
UPDATE trips_detailed t
SET
    benchmark_status = $5
FROM
    inserted
WHERE
    inserted.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND inserted.fuel_range && t.period
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &fuel_liter,
            &timestamps,
            &fuel_after_liter as &[Option<f64>],
            ProcessingStatus::Unprocessed as i32
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
