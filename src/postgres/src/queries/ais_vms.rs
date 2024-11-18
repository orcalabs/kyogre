use chrono::NaiveDate;
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use geozero::wkb;
use kyogre_core::{
    AisPermission, AisVmsAreaCount, AisVmsPosition, DateRange, Mmsi, NavigationStatus,
    PositionType, TripId, TripPositionLayerId, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY,
    LEISURE_VESSEL_SHIP_TYPES, PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};

use crate::{error::Result, models::AisVmsAreaPositionsReturning, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) fn ais_vms_area_positions_impl(
        &self,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        date_limit: NaiveDate,
    ) -> impl Stream<Item = Result<AisVmsAreaCount>> + '_ {
        let geom: geo_types::Geometry<f64> = geo_types::Rect::new((x1, y1), (x2, y2)).into();

        sqlx::query_as!(
            AisVmsAreaCount,
            r#"
SELECT
    latitude::DOUBLE PRECISION AS "lat!",
    longitude::DOUBLE PRECISION AS "lon!",
    SUM("count")::INT AS "count!",
    SUM(
        COALESCE(ARRAY_LENGTH(mmsis, 1), 0) + COALESCE(ARRAY_LENGTH(call_signs, 1), 0)
    )::INT AS "num_vessels!"
FROM
    ais_vms_area_aggregated
WHERE
    ST_CONTAINS ($1::geometry, ST_POINT (longitude, latitude))
    AND date >= $2::DATE
GROUP BY
    latitude,
    longitude
            "#,
            wkb::Encode(geom) as _,
            date_limit,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn prune_ais_vms_area_impl(&self, limit: NaiveDate) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
DELETE FROM ais_vms_area_aggregated
WHERE
    date < $1::DATE
            "#,
            limit,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM ais_vms_area_positions
WHERE
    "timestamp" < $1::DATE
            "#,
            limit,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn add_ais_vms_aggregated<'a>(
        &self,
        values: Vec<AisVmsAreaPositionsReturning>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let len = values.len();
        let mut lat = Vec::with_capacity(len);
        let mut lon = Vec::with_capacity(len);
        let mut date = Vec::with_capacity(len);
        let mut call_sign = Vec::with_capacity(len);
        let mut mmsi = Vec::with_capacity(len);

        for v in values {
            lat.push(v.latitude);
            lon.push(v.longitude);
            date.push(v.timestamp.date_naive());
            // We want our mmsi array to only contain mmsis
            // for vessels where we do not have a call sign
            if v.call_sign.is_none() {
                mmsi.push(v.mmsi);
            } else {
                mmsi.push(None);
            }
            call_sign.push(v.call_sign);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ais_vms_area_aggregated AS a (
        latitude,
        longitude,
        date,
        "count",
        mmsis,
        call_signs
    )
SELECT
    u.latitude::DECIMAL(10, 2),
    u.longitude::DECIMAL(10, 2),
    u.date,
    COUNT(*),
    COALESCE(
        ARRAY_AGG(DISTINCT u.mmsi) FILTER (
            WHERE
                u.mmsi IS NOT NULL
        ),
        '{}'
    ),
    COALESCE(
        ARRAY_AGG(DISTINCT u.call_sign) FILTER (
            WHERE
                u.call_sign IS NOT NULL
        ),
        '{}'
    )
FROM
    UNNEST(
        $1::DOUBLE PRECISION[],
        $2::DOUBLE PRECISION[],
        $3::DATE[],
        $4::INT[],
        $5::VARCHAR[]
    ) u (latitude, longitude, date, mmsi, call_sign)
GROUP BY
    u.latitude::DECIMAL(10, 2),
    u.longitude::DECIMAL(10, 2),
    u.date
ON CONFLICT (latitude, longitude, date) DO
UPDATE
SET
    "count" = a.count + EXCLUDED.count,
    mmsis = a.mmsis | EXCLUDED.mmsis,
    call_signs = ARRAY(
        SELECT
            UNNEST(a.call_signs)
        UNION
        SELECT
            UNNEST(EXCLUDED.call_signs)
    )
            "#,
            &lat,
            &lon,
            &date,
            mmsi as _,
            call_sign as _,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) fn ais_vms_positions_impl(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigational_status AS "navigational_status: NavigationStatus",
    rate_of_turn,
    true_heading,
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type!: PositionType",
    NULL AS "pruned_by: TripPositionLayerId",
    NULL AS "trip_cumulative_fuel_consumption!: Option<f64>",
    NULL AS "trip_cumulative_cargo_weight!: Option<f64>"
FROM
    (
        SELECT
            latitude,
            longitude,
            "timestamp",
            course_over_ground,
            speed_over_ground AS speed,
            navigation_status_id AS navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            $9::INT AS position_type_id
        FROM
            ais_positions a
        WHERE
            $1::INT IS NOT NULL
            AND mmsi = $1
            AND $1 IN (
                SELECT
                    a.mmsi
                FROM
                    ais_vessels a
                    LEFT JOIN fiskeridir_vessels f ON a.call_sign = f.call_sign
                WHERE
                    a.mmsi = $1
                    AND (
                        a.ship_type IS NOT NULL
                        AND NOT (a.ship_type = ANY ($5::INT[]))
                        OR COALESCE(f.length, a.ship_length) > $6
                    )
                    AND (
                        CASE
                            WHEN $7 = 0 THEN TRUE
                            WHEN $7 = 1 THEN COALESCE(f.length, a.ship_length) >= $8
                        END
                    )
            )
        UNION ALL
        SELECT
            latitude,
            longitude,
            "timestamp",
            course AS course_over_ground,
            speed,
            NULL AS navigational_status,
            NULL AS rate_of_turn,
            NULL AS true_heading,
            distance_to_shore,
            $10::INT AS position_type_id
        FROM
            vms_positions v
        WHERE
            $2::TEXT IS NOT NULL
            AND call_sign = $2
    ) q
WHERE
    "timestamp" BETWEEN $3 AND $4
ORDER BY
    "timestamp" ASC
            "#,
            mmsi as Option<Mmsi>,
            call_sign.map(|c| c.as_ref()),
            range.start(),
            range.end(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
        )
        .fetch(self.ais_pool())
        .map_err(|e| e.into())
    }

    pub(crate) fn trip_positions_impl(
        &self,
        trip_id: TripId,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    true_heading,
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type: PositionType",
    pruned_by AS "pruned_by: TripPositionLayerId",
    trip_cumulative_fuel_consumption,
    trip_cumulative_cargo_weight
FROM
    trip_positions
WHERE
    trip_id = $1
    AND (
        trip_id IN (
            SELECT
                t.trip_id
            FROM
                trips t
                INNER JOIN fiskeridir_ais_vessel_mapping_whitelist fw ON t.fiskeridir_vessel_id = fw.fiskeridir_vessel_id
                INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = fw.fiskeridir_vessel_id
                INNER JOIN ais_vessels a ON fw.mmsi = a.mmsi
            WHERE
                t.trip_id = $1
                AND (
                    a.ship_type IS NOT NULL
                    AND NOT (a.ship_type = ANY ($2::INT[]))
                    OR COALESCE(fv.length, a.ship_length) > $3
                )
                AND (
                    CASE
                        WHEN $4 = 0 THEN TRUE
                        WHEN $4 = 1 THEN COALESCE(fv.length, a.ship_length) >= $5
                    END
                )
        )
        OR position_type_id = $6
    )
ORDER BY
    "timestamp" ASC
            "#,
            trip_id.into_inner(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            PositionType::Vms as i32
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
}
