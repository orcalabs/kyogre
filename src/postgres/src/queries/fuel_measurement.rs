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
    fuel
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
        for m in measurements {
            fuel.push(m.fuel);
            call_signs.push(call_sign.as_ref());
            user_ids.push(user_id);
            timestamp.push(m.timestamp);
            id.push(m.id);
        }

        let mut tx = self.pool.begin().await?;

        let (old_vessel_ids, old_timestamps): (Vec<_>, Vec<_>) = sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            w.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp,
            u.fuel,
            f.timestamp as old_timestamp
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID [],
                $3::TIMESTAMPTZ[],
                $4::DOUBLE PRECISION[],
                $5::BIGINT[]
            ) u (
                call_sign,
                barentswatch_user_id,
                timestamp,
                fuel,
                id
            )
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist w ON w.call_sign = u.call_sign
            INNER JOIN fuel_measurements f ON u.id = f.fuel_measurement_id
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    ),
    deleted_ranges AS (
        DELETE FROM fuel_measurement_ranges r USING input
        WHERE
            r.fiskeridir_vessel_id = input.fiskeridir_vessel_id
            AND (
                r.start_measurement_ts = input.timestamp
                OR r.end_measurement_ts = input.timestamp
            )
        RETURNING
            r.fiskeridir_vessel_id,
            r.fuel_range
    ),
    updated_trips AS (
        UPDATE trips_detailed t
        SET
            benchmark_status = $6
        FROM
            deleted_ranges
        WHERE
            deleted_ranges.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND deleted_ranges.fuel_range && t.period
    )
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    old_timestamp
FROM
    input
            "#,
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &fuel,
            &id as &[FuelMeasurementId],
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| (r.fiskeridir_vessel_id, r.old_timestamp))
        .unzip();

        let (vessel_ids, timestamps, fuel): (Vec<_>, Vec<_>, Vec<_>) = sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            w.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp,
            u.fuel
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID [],
                $3::TIMESTAMPTZ[],
                $4::DOUBLE PRECISION[],
                $5::BIGINT[]
            ) u (
                call_sign,
                barentswatch_user_id,
                timestamp,
                fuel,
                id
            )
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist w ON w.call_sign = u.call_sign
            INNER JOIN fuel_measurements f ON u.id = f.fuel_measurement_id
            AND f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    )
UPDATE fuel_measurements f
SET
    fuel = input.fuel,
    barentswatch_user_id = input.barentswatch_user_id,
    timestamp = input.timestamp
FROM
    input
WHERE
    f.fiskeridir_vessel_id = input.fiskeridir_vessel_id
    AND f.timestamp = input.timestamp
RETURNING
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    f.fuel,
    f.timestamp
            "#,
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &fuel,
            &id as &[FuelMeasurementId],
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| (r.fiskeridir_vessel_id, r.timestamp, r.fuel))
        .multiunzip();

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            &vessel_ids,
            &timestamps,
            &fuel,
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

    //    start_updates AS (
    //    SELECT
    //        f.fiskeridir_vessel_id,
    //        f.fuel_range,
    //        updates.fuel
    //    FROM
    //        fuel_measurement_ranges f
    //        INNER JOIN updates ON updates.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    //        AND updates.timestamp = f.start_measurement_ts
    //),
    //    end_updates AS (
    //        SELECT
    //            f.fiskeridir_vessel_id,
    //            f.fuel_range,
    //            updates.fuel
    //        FROM
    //            fuel_measurement_ranges f
    //            INNER JOIN updates ON updates.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    //            AND updates.timestamp = f.end_measurement_ts
    //    ),
    //    updated_ranges AS (
    //        UPDATE fuel_measurement_ranges f
    //        SET
    //            start_measurement_fuel = q.start_fuel,
    //            end_measurement_fuel = q.end_fuel
    //        FROM
    //            (
    //                SELECT
    //                    s.fiskeridir_vessel_id,
    //                    s.fuel_range,
    //                    s.fuel AS start_fuel,
    //                    e.fuel AS end_fuel
    //                FROM
    //                    start_updates s
    //                    INNER JOIN end_updates e ON s.fiskeridir_vessel_id = e.fiskeridir_vessel_id
    //                    AND s.fuel_range = e.fuel_range
    //            ) q
    //        WHERE
    //            q.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    //            AND q.fuel_range = f.fuel_range
    //        RETURNING
    //            f.fiskeridir_vessel_id,
    //            f.fuel_range
    //    )
    //UPDATE trips_detailed t
    //SET
    //    benchmark_status = $6
    //FROM
    //    updated_ranges
    //WHERE
    //    updated_ranges.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    //    AND updated_ranges.fuel_range && t.period
    //
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
        for m in measurements {
            fuel.push(m.fuel);
            call_signs.push(call_sign.as_ref());
            user_ids.push(user_id);
            timestamp.push(m.timestamp);
        }

        let mut tx = self.pool.begin().await?;

        #[derive(Debug)]
        struct Intermediate {
            id: FuelMeasurementId,
            fiskeridir_vessel_id: FiskeridirVesselId,
            timestamp: DateTime<Utc>,
            fuel: f64,
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
                fuel
            )
        SELECT
            f.fiskeridir_vessel_id,
            u.barentswatch_user_id,
            u.timestamp,
            u.fuel
        FROM
            UNNEST(
                $1::TEXT[],
                $2::UUID [],
                $3::TIMESTAMPTZ[],
                $4::DOUBLE PRECISION[]
            ) u (call_sign, barentswatch_user_id, timestamp, fuel)
            INNER JOIN fiskeridir_ais_vessel_mapping_whitelist f ON f.call_sign = u.call_sign
        ON CONFLICT (fiskeridir_vessel_id, timestamp) DO NOTHING
        RETURNING
            fuel_measurement_id,
            fiskeridir_vessel_id,
            timestamp,
            fuel
    ),
    deleted AS (
        DELETE FROM fuel_measurement_ranges r USING inserted
        WHERE
            fuel_range @> inserted.timestamp
            AND r.fiskeridir_vessel_id = inserted.fiskeridir_vessel_id
    )
SELECT
    fuel_measurement_id AS "id: FuelMeasurementId",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId",
    timestamp,
    fuel
FROM
    inserted
            "#,
            &call_signs as &[&str],
            &user_ids as &[BarentswatchUserId],
            &timestamp,
            &fuel
        )
        .fetch_all(&mut *tx)
        .await?;

        dbg!(&measurements);

        let mut vessel_ids = Vec::with_capacity(measurements.len());
        let mut ts = Vec::with_capacity(measurements.len());
        let mut fuel = Vec::with_capacity(measurements.len());
        for m in &measurements {
            vessel_ids.push(m.fiskeridir_vessel_id);
            ts.push(m.timestamp);
            fuel.push(m.fuel);
        }

        let out = measurements
            .into_iter()
            .map(|m| kyogre_core::FuelMeasurement {
                id: m.id,
                timestamp: m.timestamp,
                fuel: m.fuel,
            })
            .collect();

        self.add_fuel_measurement_ranges_post_measurement_insertion(
            &vessel_ids,
            &ts,
            &fuel,
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

    pub(crate) async fn add_fuel_measurement_ranges_updates_impl(
        &self,
        updates: &[kyogre_core::UpdateFuelMeasurementRange],
    ) -> Result<()> {
        // TODO: add support for 'PgRange' in unnest_insert
        let mut vessel_ids = Vec::with_capacity(updates.len());
        let mut fuel_range: Vec<PgRange<DateTime<Utc>>> = Vec::with_capacity(updates.len());
        let mut pre_fuel = Vec::with_capacity(updates.len());
        let mut post_fuel = Vec::with_capacity(updates.len());

        for u in updates {
            vessel_ids.push(u.fiskeridir_vessel_id.into_inner());
            fuel_range.push((&u.fuel_range).into());
            pre_fuel.push(u.pre_fuel);
            post_fuel.push(u.post_fuel);
        }

        sqlx::query!(
            r#"
UPDATE fuel_measurement_ranges f
SET
    pre_estimate_value = q.pre_estimate_value,
    post_estimate_value = q.post_estimate_value,
    pre_post_estimate_status = $5
FROM
    (
        SELECT
            u.fiskeridir_vessel_id,
            u.fuel_range,
            u.pre_estimate_value,
            u.post_estimate_value
        FROM
            unnest(
                $1::BIGINT[],
                $2::TSTZRANGE[],
                $3::DOUBLE PRECISION[],
                $4::DOUBLE PRECISION[]
            ) u (
                fiskeridir_vessel_id,
                fuel_range,
                pre_estimate_value,
                post_estimate_value
            )
    ) q
WHERE
    f.fiskeridir_vessel_id = q.fiskeridir_vessel_id
    AND f.fuel_range = q.fuel_range
            "#,
            &vessel_ids,
            &fuel_range,
            &pre_fuel,
            &post_fuel,
            ProcessingStatus::Successful as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn unprocessed_fuel_measurement_ranges_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<kyogre_core::FuelMeasurementRange>> {
        let ranges = sqlx::query_as!(
            kyogre_core::FuelMeasurementRange,
            r#"
SELECT
    fuel_used,
    fuel_range AS "fuel_range: DateRange",
    pre_estimate_ts,
    pre_estimate_value,
    post_estimate_ts,
    post_estimate_value,
    fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId"
FROM
    fuel_measurement_ranges
WHERE
    fiskeridir_vessel_id = $1
    AND pre_post_estimate_status = $2
            "#,
            vessel_id.into_inner(),
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(ranges)
    }
    pub(crate) async fn trip_fuel_measurements_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> Result<kyogre_core::TripFuelMeasurement> {
        let range: PgRange<DateTime<Utc>> = range.into();
        let out = sqlx::query_as!(
            kyogre_core::TripFuelMeasurement,
            r#"
WITH
    ranges AS (
        SELECT
            f.fiskeridir_vessel_id,
            f.fuel_range,
            f.fuel_used
        FROM
            fuel_measurement_ranges f
        WHERE
            f.fiskeridir_vessel_id = $1
            AND f.fuel_range && $2
            AND UPPER(f.fuel_range * $2) - LOWER(f.fuel_range * $2) >= (UPPER(f.fuel_range) - LOWER(f.fuel_range)) / 2
    ),
    start AS (
        --! fuel_measurement_ranges are non-overlapping so this can only return a single row
        SELECT
            fiskeridir_vessel_id,
            LOWER(fuel_range) AS start_measurement_ts
        FROM
            ranges
        WHERE
            LOWER(fuel_range) < LOWER($2)
    ),
    ending AS (
        --! fuel_measurement_ranges are non-overlapping so this can only return a single row
        SELECT
            fiskeridir_vessel_id,
            UPPER(fuel_range) AS end_measurement_ts
        FROM
            ranges
        WHERE
            UPPER(fuel_range) > UPPER($2)
    )
SELECT
    COALESCE(SUM(r.fuel_used), 0.0) AS "total_overlapping_fuel!",
    MAX(s.start_measurement_ts) AS start_measurement_ts,
    MAX(e.end_measurement_ts) AS end_measurement_ts
FROM
    ranges r
    LEFT JOIN start s ON r.fiskeridir_vessel_id = s.fiskeridir_vessel_id
    LEFT JOIN ending e ON r.fiskeridir_vessel_id = e.fiskeridir_vessel_id
                "#,
                vessel_id.into_inner(),
                range)
                    .fetch_one(&self.pool)
                    .await?;
        Ok(out)
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
            f.fuel AS end_fuel
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
            f.fuel AS start_fuel
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
        start_measurement_fuel,
        end_measurement_ts,
        end_measurement_fuel,
        pre_post_estimate_status
    )
SELECT
    t.fiskeridir_vessel_id,
    b.start_ts,
    b.start_fuel,
    t.end_ts,
    t.end_fuel,
    $3
FROM
    top t
    INNER JOIN bottom b ON t.fiskeridir_vessel_id = b.fiskeridir_vessel_id
    AND t.deleted_timestamp = b.deleted_timestamp
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &timestamps,
            ProcessingStatus::Unprocessed as i32
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn add_fuel_measurement_ranges_post_measurement_insertion(
        &self,
        vessel_ids: &[FiskeridirVesselId],
        timestamps: &[DateTime<Utc>],
        fuel: &[f64],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        dbg!(vessel_ids);
        dbg!(timestamps);
        dbg!(fuel);
        sqlx::query!(
            r#"
WITH
    input AS (
        SELECT
            UNNEST($1::BIGINT[]) fiskeridir_vessel_id,
            UNNEST($2::DOUBLE PRECISION[]) fuel,
            UNNEST($3::TIMESTAMPTZ[]) timestamp
    ),
    top AS (
        SELECT DISTINCT
            ON (i.fiskeridir_vessel_id, i.timestamp) i.fiskeridir_vessel_id AS fiskeridir_vessel_id,
            i.timestamp AS start_ts,
            i.fuel AS start_fuel,
            f.timestamp AS end_ts,
            f.fuel AS end_fuel
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
            f.fuel AS start_fuel,
            i.timestamp AS end_ts,
            i.fuel AS end_fuel
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
                start_measurement_fuel,
                end_measurement_ts,
                end_measurement_fuel,
                pre_post_estimate_status
            )
        SELECT
            b.fiskeridir_vessel_id,
            b.start_ts,
            b.start_fuel,
            b.end_ts,
            b.end_fuel,
            $4::INT
        FROM
            bottom b
        UNION
        SELECT
            t.fiskeridir_vessel_id,
            t.start_ts,
            t.start_fuel,
            t.end_ts,
            t.end_fuel,
            $4::INT
        FROM
            top t
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
            &vessel_ids as &[FiskeridirVesselId],
            &fuel,
            &timestamps,
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&mut **tx)
        .await?;

        Ok(())
    }
}
