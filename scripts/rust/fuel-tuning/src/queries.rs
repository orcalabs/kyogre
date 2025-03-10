#![allow(dead_code)]

use anyhow::{Context, Result};
use fiskeridir_rs::{CallSign, FiskeridirVesselId, Gear, GearGroup};
use kyogre_core::{DateRange, DepartureWeight, EngineType, HaulWeight, Mmsi, engines};
use sqlx::{PgPool, postgres::types::PgRange};
use tokio::join;

use crate::{Position, Vessel};

pub async fn get_vessel(pool: &PgPool, id: i64) -> Result<Vessel> {
    sqlx::query!(
        r#"
SELECT
    a.fiskeridir_vessel_id AS "id!: FiskeridirVesselId",
    a.mmsi AS "mmsi: Mmsi",
    a.call_sign AS "call_sign: CallSign",
    (
        SELECT
            MAX(landing_total_living_weight)
        FROM
            trips_detailed
        WHERE
            fiskeridir_vessel_id = $1
    ) AS max_cargo_weight,
    f.engine_building_year_final AS engine_building_year,
    f.engine_power_final AS engine_power,
    f.auxiliary_engine_power AS aux_engine_power,
    f.auxiliary_engine_building_year AS aux_engine_building_year,
    f.boiler_engine_power AS boil_engine_power,
    f.boiler_engine_building_year AS boil_engine_building_year,
    f.engine_type_manual AS "engine_type: EngineType",
    f.engine_rpm_manual AS "engine_rpm"
FROM
    all_vessels a
    INNER JOIN fiskeridir_vessels f ON a.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    a.fiskeridir_vessel_id = $1
        "#,
        id,
    )
    .fetch_one(pool)
    .await
    .map(|v| Vessel {
        id: v.id,
        mmsi: v.mmsi,
        call_sign: v.call_sign,
        max_cargo_weight: v.max_cargo_weight,
        engines: engines(
            v.engine_power.map(|v| v as _),
            v.engine_building_year.map(|v| v as _),
            v.aux_engine_power.map(|v| v as _),
            v.aux_engine_building_year.map(|v| v as _),
            v.boil_engine_power.map(|v| v as _),
            v.boil_engine_building_year.map(|v| v as _),
            v.engine_type,
            v.engine_rpm.map(|v| v as _),
        ),
    })
    .context("get_vessel")
}

pub async fn get_positions(
    pool: &PgPool,
    vessel: &Vessel,
    range: &DateRange,
) -> Result<Vec<Position>> {
    let (positions, departures, hauls) = join!(
        get_positions_impl(pool, vessel, range),
        departure_weights_from_range(pool, vessel, range),
        haul_weights_from_range(pool, vessel, range),
    );

    let mut positions = positions?;
    let departures = departures?;
    let hauls = hauls?;

    let positions_len = positions.len();

    let mut hauls_iter = hauls.into_iter();
    let mut current_haul = hauls_iter.next();
    let mut current_weight = 0.0;
    let mut i = 0;

    while i < positions_len {
        let current_position = &positions[i];

        if let Some(haul) = &current_haul {
            if haul.period.contains(current_position.timestamp) {
                let haul_start_idx = i;

                let haul_end_idx = positions
                    .iter()
                    .enumerate()
                    .skip(i + 1)
                    .skip_while(|(_, p)| haul.period.contains(p.timestamp))
                    .map(|(i, _)| i)
                    .next()
                    .unwrap_or(positions_len);

                let num_haul_positions = (haul_end_idx - haul_start_idx) as f64;
                // 'num_haul_positions' is ALWAYS 1 or greater
                let weight_per_position = haul.weight / num_haul_positions;

                (haul_start_idx..haul_end_idx).for_each(|idx| {
                    current_weight += weight_per_position;
                    positions[idx].cumulative_cargo_weight = current_weight;
                });

                current_haul = hauls_iter.next();
                i = haul_end_idx;
                continue;
            } else if haul.period.end() < current_position.timestamp {
                current_weight += haul.weight;
                current_haul = hauls_iter.next();
                continue;
            }
        }

        positions[i].cumulative_cargo_weight = current_weight;
        i += 1;
    }

    let mut deps_iter = departures.into_iter().peekable();
    let mut current_weight = 0.;

    for pos in positions.iter_mut() {
        if deps_iter
            .peek()
            .is_some_and(|v| v.departure_timestamp <= pos.timestamp)
        {
            // `unwrap` is safe due to `is_some_and` check
            current_weight = deps_iter.next().unwrap().weight;
        }

        pos.cumulative_cargo_weight += current_weight;
    }

    Ok(positions)
}

pub async fn get_positions_impl(
    pool: &PgPool,
    vessel: &Vessel,
    range: &DateRange,
) -> Result<Vec<Position>> {
    sqlx::query_as!(
        Position,
        r#"
WITH
    overlapping_haul_ranges AS (
        SELECT
            MIN(h.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
            UNNEST(RANGE_AGG(h.period)) AS range
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1::BIGINT
            AND h.period && TSTZRANGE ($2, $3, '[]')
            AND h.gear_group_id = ANY ($4::INT[])
    ),
    overlapping_hauls AS (
        SELECT DISTINCT
            ON (r.range) r.range,
            h.gear_id
        FROM
            hauls h
            INNER JOIN overlapping_haul_ranges r ON h.fiskeridir_vessel_id = r.fiskeridir_vessel_id
            AND h.period && r.range
            AND h.gear_group_id = ANY ($4)
        ORDER BY
            r.range,
            LEN_OF_RANGE (h.period) DESC
    )
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    timestamp AS "timestamp!",
    speed,
    h.gear_id AS "active_gear?: Gear",
    0 AS "cumulative_cargo_weight!"
FROM
    (
        SELECT
            latitude,
            longitude,
            timestamp,
            speed_over_ground AS speed
        FROM
            ais_positions
        WHERE
            mmsi = $5
            AND timestamp BETWEEN $2 AND $3
        UNION ALL
        SELECT
            latitude,
            longitude,
            timestamp,
            speed
        FROM
            vms_positions
        WHERE
            call_sign = $6
            AND timestamp BETWEEN $2 AND $3
    ) q
    LEFT JOIN overlapping_hauls h ON q.timestamp <@ h.range
ORDER BY
    q.timestamp
        "#,
        vessel.id.into_inner(),
        range.start(),
        range.end(),
        &GearGroup::active_int(),
        vessel.mmsi as Option<Mmsi>,
        vessel.call_sign.as_ref() as Option<&CallSign>,
    )
    .fetch_all(pool)
    .await
    .context("get_positions_impl")
}

pub async fn get_trip_positions(
    pool: &PgPool,
    vessel: &Vessel,
    range: DateRange,
) -> Result<Vec<Position>> {
    sqlx::query_as!(
        Position,
        r#"
WITH
    trip AS (
        SELECT
            trip_id
        FROM
            trips
        WHERE
            fiskeridir_vessel_id = $1
            AND period && $2
        ORDER BY
            COMPUTE_TS_RANGE_PERCENT_OVERLAP ($2, period) DESC
        LIMIT
            1
    ),
    overlapping_haul_ranges AS (
        SELECT
            MIN(h.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
            UNNEST(RANGE_AGG(h.period)) AS range
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1::BIGINT
            AND h.period && $2
            AND h.gear_group_id = ANY ($3::INT[])
    ),
    overlapping_hauls AS (
        SELECT DISTINCT
            ON (r.range) r.range,
            h.gear_id
        FROM
            hauls h
            INNER JOIN overlapping_haul_ranges r ON h.fiskeridir_vessel_id = r.fiskeridir_vessel_id
            AND h.period && r.range
            AND h.gear_group_id = ANY ($3)
        ORDER BY
            r.range,
            LEN_OF_RANGE (h.period) DESC
    )
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    timestamp AS "timestamp!",
    speed,
    h.gear_id AS "active_gear?: Gear",
    trip_cumulative_cargo_weight AS cumulative_cargo_weight
FROM
    trip_positions p
    INNER JOIN trip t ON p.trip_id = t.trip_id
    LEFT JOIN overlapping_hauls h ON p.timestamp <@ h.range
ORDER BY
    p.timestamp
        "#,
        vessel.id.into_inner(),
        PgRange::from(&range),
        &GearGroup::active_int(),
    )
    .fetch_all(pool)
    .await
    .context("get_trip_positions")
}

async fn departure_weights_from_range(
    pool: &PgPool,
    vessel: &Vessel,
    range: &DateRange,
) -> Result<Vec<DepartureWeight>> {
    sqlx::query_as!(
        DepartureWeight,
        r#"
WITH
    deps AS (
        SELECT
            MAX(message_id) AS message_id,
            departure_timestamp
        FROM
            ers_departures
        WHERE
            fiskeridir_vessel_id = $1::BIGINT
            AND departure_timestamp >= $2::TIMESTAMPTZ
            AND departure_timestamp < $3::TIMESTAMPTZ
        GROUP BY
            departure_timestamp
    )
SELECT
    e.departure_timestamp,
    COALESCE(SUM(c.living_weight), 0)::DOUBLE PRECISION AS "weight!"
FROM
    deps e
    LEFT JOIN ers_departure_catches c ON e.message_id = c.message_id
GROUP BY
    e.message_id,
    e.departure_timestamp
ORDER BY
    e.departure_timestamp ASC
        "#,
        vessel.id.into_inner(),
        range.start(),
        range.end(),
    )
    .fetch_all(pool)
    .await
    .context("departure_weights_from_range")
}

async fn haul_weights_from_range(
    pool: &PgPool,
    vessel: &Vessel,
    range: &DateRange,
) -> Result<Vec<HaulWeight>> {
    sqlx::query_as!(
        HaulWeight,
        r#"
SELECT
    q.ranges AS "period!: DateRange",
    COALESCE(SUM(h.total_living_weight), 0.0)::DOUBLE PRECISION AS "weight!"
FROM
    hauls h
    INNER JOIN (
        SELECT
            UNNEST(RANGE_AGG(period)) AS ranges
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1
            AND h.period <@ $2
    ) q ON q.ranges <@ h.period
WHERE
    h.fiskeridir_vessel_id = $1
    AND h.period <@ $2
GROUP BY
    q.ranges
        "#,
        vessel.id.into_inner(),
        PgRange::from(range),
    )
    .fetch_all(pool)
    .await
    .context("haul_weights_from_range")
}
