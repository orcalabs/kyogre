use futures::StreamExt;
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use fiskeridir_rs::{
    CallSign, DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup,
};
use futures::{Stream, TryStreamExt, future::ready};
use kyogre_core::{
    BoxIterator, DeletedLandingType, EmptyVecToNone, FiskeridirVesselId, LandingsQuery, Range,
    TripAssemblerId, TripId, VesselEventType,
};
use tracing::error;

use crate::{
    PostgresAdapter,
    chunk::Chunks,
    error::Result,
    landing_set::LandingSet,
    models::{Landing, NewLanding, NewTripAssemblerConflict},
};

static CHUNK_SIZE: usize = 100_000;

pub static LANDINGS_VERIFY_CHUNK_SIZE: OnceLock<usize> = OnceLock::new();

impl PostgresAdapter {
    pub(crate) fn landings_impl(
        &self,
        query: LandingsQuery,
    ) -> impl Stream<Item = Result<Landing>> + '_ {
        let (catch_area_ids, catch_main_area_ids) = if !query.catch_locations.is_empty() {
            let (catch_areas, main_areas): (Vec<_>, Vec<_>) = query
                .catch_locations
                .into_iter()
                .map(|v| (v.catch_area(), v.main_area()))
                .unzip();
            (Some(catch_areas), Some(main_areas))
        } else {
            (None, None)
        };

        sqlx::query_as!(
            Landing,
            r#"
SELECT
    l.landing_id AS "landing_id!: LandingId",
    MAX(e.trip_id) AS "trip_id: TripId",
    l.landing_timestamp,
    l.catch_area_id,
    l.catch_main_area_id,
    l.gear_id AS "gear_id!: Gear",
    l.gear_group_id AS "gear_group_id!: GearGroup",
    COALESCE(MIN(d.new_delivery_point_id), l.delivery_point_id) AS "delivery_point_id: DeliveryPointId",
    l.fiskeridir_vessel_id AS "fiskeridir_vessel_id?: FiskeridirVesselId",
    l.vessel_call_sign AS "vessel_call_sign: CallSign",
    l.vessel_name,
    l.vessel_length,
    l.vessel_length_group_id AS "vessel_length_group!: VesselLengthGroup",
    COALESCE(SUM(le.gross_weight), 0) AS "total_gross_weight!",
    COALESCE(SUM(le.living_weight), 0) AS "total_living_weight!",
    COALESCE(SUM(le.product_weight), 0) AS "total_product_weight!",
    JSONB_AGG(
        JSONB_BUILD_OBJECT(
            'living_weight',
            COALESCE(le.living_weight, 0),
            'gross_weight',
            COALESCE(le.gross_weight, 0),
            'product_weight',
            le.product_weight,
            'species_fiskeridir_id',
            le.species_fiskeridir_id,
            'species_group_id',
            le.species_group_id
        )
    )::TEXT AS "catches!",
    "version"
FROM
    landings l
    INNER JOIN landing_entries le ON l.landing_id = le.landing_id
    LEFT JOIN deprecated_delivery_points d ON l.delivery_point_id = d.old_delivery_point_id
    LEFT JOIN vessel_events e ON l.vessel_event_id = e.vessel_event_id
WHERE
    (
        $1::tstzrange[] IS NULL
        OR l.landing_timestamp <@ ANY ($1::tstzrange[])
    )
    AND (
        $2::INT[] IS NULL
        OR l.catch_area_id = ANY ($2::INT[])
    )
    AND (
        $3::INT[] IS NULL
        OR l.catch_main_area_id = ANY ($3::INT[])
    )
    AND (
        $4::INT[] IS NULL
        OR l.gear_group_id = ANY ($4)
    )
    AND (
        $5::INT[] IS NULL
        OR l.vessel_length_group_id = ANY ($5)
    )
    AND (
        $6::BIGINT[] IS NULL
        OR l.fiskeridir_vessel_id = ANY ($6)
    )
    AND (
        $7::TIMESTAMPTZ IS NULL
        OR l.landing_timestamp >= $7
    )
    AND (
        $8::TIMESTAMPTZ IS NULL
        OR l.landing_timestamp <= $8
    )
GROUP BY
    l.landing_id
HAVING
    (
        $9::INT[] IS NULL
        OR ARRAY_AGG(le.species_group_id) && $9
    )
ORDER BY
    CASE
        WHEN $10 = 1
        AND $11 = 1 THEN l.landing_timestamp
    END ASC,
    CASE
        WHEN $10 = 1
        AND $11 = 2 THEN SUM(le.living_weight)
    END ASC,
    CASE
        WHEN $10 = 2
        AND $11 = 1 THEN l.landing_timestamp
    END DESC,
    CASE
        WHEN $10 = 2
        AND $11 = 2 THEN SUM(le.living_weight)
    END DESC
OFFSET
    $12
LIMIT
    $13
            "#,
            query.ranges.empty_to_none() as Option<Vec<Range<DateTime<Utc>>>>,
            catch_area_ids.as_deref(),
            catch_main_area_ids.as_deref(),
            query.gear_group_ids.empty_to_none() as Option<Vec<GearGroup>>,
            query.vessel_length_groups.empty_to_none() as Option<Vec<VesselLengthGroup>>,
            query.vessel_ids.empty_to_none() as Option<Vec<FiskeridirVesselId>>,
            query.range.start(),
            query.range.end(),
            query.species_group_ids.empty_to_none() as Option<Vec<SpeciesGroup>>,
            query.ordering.map(|o| o as i32),
            query.sorting.map(|s| s as i32),
            query.pagination.offset() as i64,
            query.pagination.limit() as i64,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_landings_impl(
        &self,
        landings: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::Landing>>,
        data_year: u32,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let existing_landings = self.existing_landings(data_year, &mut tx).await?;

        let mut all_vessel_ids = HashSet::new();
        let mut existing_landing_ids = HashSet::new();
        let mut inserted_landing_ids = HashSet::new();
        let mut vessel_event_ids = Vec::new();
        let mut trip_assembler_conflicts =
            HashMap::<FiskeridirVesselId, NewTripAssemblerConflict>::new();

        let mut num_landings = 0;

        let mut set = LandingSet::with_capacity(CHUNK_SIZE, data_year);

        let mut chunks = landings.chunks(CHUNK_SIZE);
        while let Some(chunk) = chunks.next() {
            // SAFETY: This `transmute` is necessary to "reset" the lifetime of the set so
            // that it no longer borrows from `chunk` at the end of the scope.
            // This is safe as long as the set is completely cleared before `chunk` is
            // dropped.
            let temp_set: &mut LandingSet<'_> = unsafe { std::mem::transmute(&mut set) };

            temp_set.add_all(chunk.iter().filter_map(|v| match v {
                Ok(v) => {
                    num_landings += 1;
                    existing_landing_ids.insert(v.id.clone());
                    if let Some(vessel_id) = v.vessel.id {
                        all_vessel_ids.insert(vessel_id);
                    }

                    if existing_landings
                        .get(&v.id)
                        .is_some_and(|version| version >= &v.document_info.version_number)
                    {
                        None
                    } else {
                        Some(v)
                    }
                }
                Err(e) => {
                    error!("failed to read data: {e:?}");
                    None
                }
            }));

            self.add_landing_set(
                temp_set,
                &mut inserted_landing_ids,
                &mut vessel_event_ids,
                &mut trip_assembler_conflicts,
                &mut tx,
            )
            .await?;

            temp_set.assert_is_empty();
        }

        let existing_landing_ids = existing_landing_ids.into_iter().collect::<Vec<_>>();
        let inserted_landing_ids = inserted_landing_ids.into_iter().collect::<Vec<_>>();

        self.delete_removed_landings(
            &existing_landing_ids,
            &mut trip_assembler_conflicts,
            data_year,
            &mut tx,
        )
        .await?;

        self.add_landing_matrix(&inserted_landing_ids, &mut tx)
            .await?;

        self.add_trip_assembler_conflicts(
            trip_assembler_conflicts.into_values().collect(),
            TripAssemblerId::Landings,
            &mut tx,
        )
        .await?;
        self.connect_trip_to_events(&vessel_event_ids, VesselEventType::Landing, &mut tx)
            .await?;
        self.add_vessel_gear_and_species_groups(all_vessel_ids, &mut tx)
            .await?;

        self.set_landing_vessels_call_signs(&mut tx).await?;
        self.refresh_vessel_mappings(&mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_landing_set<'a>(
        &'a self,
        set: &mut LandingSet<'_>,
        inserted_landing_ids: &mut HashSet<String>,
        vessel_event_ids: &mut Vec<i64>,
        trip_assembler_conflicts: &mut HashMap<FiskeridirVesselId, NewTripAssemblerConflict>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        self.unnest_insert(set.delivery_points(), &mut **tx).await?;
        self.unnest_insert(set.municipalities(), &mut **tx).await?;
        self.unnest_insert(set.counties(), &mut **tx).await?;
        self.unnest_insert(set.vessels(), &mut **tx).await?;
        self.unnest_insert(set.species_fiskeridir(), &mut **tx)
            .await?;
        self.unnest_insert(set.species(), &mut **tx).await?;
        self.unnest_insert(set.species_fao(), &mut **tx).await?;
        self.unnest_insert(set.catch_areas(), &mut **tx).await?;
        self.unnest_insert(set.catch_main_areas(), &mut **tx)
            .await?;
        self.unnest_insert(set.catch_main_area_fao(), &mut **tx)
            .await?;
        self.unnest_insert(set.area_groupings(), &mut **tx).await?;

        self.add_landings(
            set.landings(),
            inserted_landing_ids,
            vessel_event_ids,
            trip_assembler_conflicts,
            tx,
        )
        .await?;

        self.unnest_insert(set.landing_entries(), &mut **tx).await?;

        Ok(())
    }

    async fn add_landings<'a>(
        &'a self,
        mut landings: Vec<NewLanding<'_>>,
        inserted_landing_ids: &mut HashSet<String>,
        vessel_event_ids: &mut Vec<i64>,
        trip_assembler_conflicts: &mut HashMap<FiskeridirVesselId, NewTripAssemblerConflict>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        landings.retain(|l| !inserted_landing_ids.contains(l.landing_id));

        let len = landings.len();
        let mut landing_id = Vec::with_capacity(len);
        let mut version = Vec::with_capacity(len);

        for l in landings.iter() {
            landing_id.push(l.landing_id);
            version.push(l.version);
        }

        let deleted = sqlx::query!(
            r#"
WITH
    deleted AS (
        DELETE FROM landings l USING UNNEST($1::TEXT[], $2::INT[]) u (landing_id, "version")
        WHERE
            l.landing_id = u.landing_id
            AND l.version < u.version
        RETURNING
            l.landing_id,
            l.fiskeridir_vessel_id,
            l.landing_timestamp
    ),
    _ AS (
        INSERT INTO
            deleted_landings (
                landing_id,
                fiskeridir_vessel_id,
                landing_timestamp,
                deleted_landing_type_id
            )
        SELECT
            landing_id,
            fiskeridir_vessel_id,
            landing_timestamp,
            $3::INT
        FROM
            deleted
    )
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id?: FiskeridirVesselId",
    landing_timestamp AS "landing_timestamp!"
FROM
    deleted
            "#,
            &landing_id as _,
            &version,
            DeletedLandingType::NewVersion as i32,
        )
        .fetch_all(&mut **tx)
        .await?;

        self.unnest_insert_returning(landings, &mut **tx)
            .try_for_each(|i| {
                if let (Some(id), Some(event_id)) = (i.fiskeridir_vessel_id, i.vessel_event_id) {
                    trip_assembler_conflicts
                        .entry(id)
                        .and_modify(|v| v.timestamp = min(v.timestamp, i.landing_timestamp))
                        .or_insert_with(|| NewTripAssemblerConflict {
                            fiskeridir_vessel_id: id,
                            timestamp: Utc.from_utc_datetime(&NaiveDateTime::new(
                                i.landing_timestamp.date_naive(),
                                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                            )),
                            vessel_event_id: Some(event_id),
                            event_type: VesselEventType::Landing,
                            vessel_event_timestamp: i.landing_timestamp,
                        });
                    vessel_event_ids.push(event_id);
                }
                inserted_landing_ids.insert(i.landing_id);
                ready(Ok(()))
            })
            .await?;

        for d in deleted {
            if let Some(id) = d.fiskeridir_vessel_id {
                trip_assembler_conflicts
                    .entry(id)
                    .and_modify(|v| v.timestamp = min(v.timestamp, d.landing_timestamp))
                    .or_insert_with(|| NewTripAssemblerConflict {
                        fiskeridir_vessel_id: id,
                        timestamp: Utc.from_utc_datetime(&NaiveDateTime::new(
                            d.landing_timestamp.date_naive(),
                            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        )),
                        vessel_event_id: None,
                        event_type: VesselEventType::Landing,
                        vessel_event_timestamp: d.landing_timestamp,
                    });
            }
        }

        Ok(())
    }

    async fn existing_landings<'a>(
        &'a self,
        data_year: u32,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashMap<LandingId, i32>> {
        let landings = sqlx::query!(
            r#"
SELECT
    landing_id AS "id!: LandingId",
    "version"
FROM
    landings
WHERE
    data_year = $1
            "#,
            data_year as i32,
        )
        .fetch(&mut **tx)
        .map_ok(|r| (r.id, r.version))
        .try_collect::<HashMap<_, _>>()
        .await?;

        Ok(landings)
    }

    pub(crate) async fn delete_removed_landings<'a>(
        &'a self,
        existing_landing_ids: &[LandingId],
        trip_assembler_conflicts: &mut HashMap<FiskeridirVesselId, NewTripAssemblerConflict>,
        data_year: u32,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let (vessel_ids, landing_timestamps): (Vec<_>, Vec<_>) = sqlx::query!(
            r#"
WITH
    deleted AS (
        DELETE FROM landings
        WHERE
            (NOT landing_id = ANY ($1::TEXT[]))
            AND data_year = $2::INT
        RETURNING
            landing_id,
            fiskeridir_vessel_id,
            landing_timestamp
    ),
    _ AS (
        INSERT INTO
            deleted_landings (
                landing_id,
                fiskeridir_vessel_id,
                landing_timestamp,
                deleted_landing_type_id
            )
        SELECT
            landing_id,
            fiskeridir_vessel_id,
            landing_timestamp,
            $3::INT
        FROM
            deleted
    )
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    landing_timestamp
FROM
    deleted
WHERE
    fiskeridir_vessel_id IS NOT NULL
            "#,
            existing_landing_ids as &[LandingId],
            data_year as i32,
            DeletedLandingType::RemovedFromDataset as i32,
        )
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|v| (v.fiskeridir_vessel_id, v.landing_timestamp))
        .unzip();

        if vessel_ids.is_empty() {
            return Ok(());
        }

        let conflicts = sqlx::query!(
            r#"
WITH
    deleted AS (
        SELECT
            UNNEST($1::BIGINT[]) AS fiskeridir_vessel_id,
            UNNEST($2::TIMESTAMPTZ[]) AS landing_timestamp
    ),
    min_landings AS (
        SELECT
            fiskeridir_vessel_id,
            MIN(landing_timestamp) AS landing_timestamp
        FROM
            deleted d
        GROUP BY
            fiskeridir_vessel_id
    ),
    first_landings AS (
        SELECT
            l.fiskeridir_vessel_id,
            MIN(l.landing_timestamp) AS landing_timestamp
        FROM
            landings l
        WHERE
            l.fiskeridir_vessel_id = ANY ($1::BIGINT[])
        GROUP BY
            l.fiskeridir_vessel_id
    )
SELECT
    m.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    (
        SELECT
            CASE
                WHEN l.landing_timestamp = f.landing_timestamp THEN NULL
                ELSE l.landing_timestamp
            END
        FROM
            landings l
            INNER JOIN first_landings f ON l.fiskeridir_vessel_id = f.fiskeridir_vessel_id
        WHERE
            l.fiskeridir_vessel_id = m.fiskeridir_vessel_id
            AND l.landing_timestamp < m.landing_timestamp
        ORDER BY
            l.landing_timestamp DESC
        OFFSET
            1
        LIMIT
            1
    ) AS landing_timestamp
FROM
    min_landings m
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &landing_timestamps,
        )
        .fetch_all(&mut **tx)
        .await?;

        let mut queued_resets = Vec::new();

        for c in conflicts {
            if let Some(landing_timestamp) = c.landing_timestamp {
                trip_assembler_conflicts
                    .entry(c.fiskeridir_vessel_id)
                    .and_modify(|v| v.timestamp = min(v.timestamp, landing_timestamp))
                    .or_insert_with(|| NewTripAssemblerConflict {
                        fiskeridir_vessel_id: c.fiskeridir_vessel_id,
                        timestamp: Utc.from_utc_datetime(&NaiveDateTime::new(
                            landing_timestamp.date_naive(),
                            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                        )),
                        vessel_event_id: None,
                        event_type: VesselEventType::Landing,
                        vessel_event_timestamp: landing_timestamp,
                    });
            } else {
                queued_resets.push(c.fiskeridir_vessel_id);
            }
        }

        if !queued_resets.is_empty() {
            sqlx::query!(
                r#"
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
WHERE
    fiskeridir_vessel_id = ANY ($1)
                "#,
                &queued_resets as &[FiskeridirVesselId],
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn landing_matrix_vs_landings_living_weight(&self) -> Result<i64> {
        let mut stream = sqlx::query!(
            r#"
SELECT
    l.landing_id
FROM
    landings l
    INNER JOIN catch_locations c ON l.catch_main_area_id = c.catch_main_area_id
    AND l.catch_area_id = c.catch_area_id
            "#
        )
        .fetch(&self.pool)
        .map_ok(|r| r.landing_id)
        .chunks(*LANDINGS_VERIFY_CHUNK_SIZE.get_or_init(|| 100_000));

        let mut sum = 0;
        while let Some(landing_id_chunk) = stream.next().await {
            let ids: Vec<String> = landing_id_chunk.into_iter().flatten().collect();
            sum += sqlx::query!(
                r#"
SELECT
    (
        (
            SELECT
                COALESCE(SUM(living_weight), 0)
            FROM
                landing_entries
            WHERE
                landing_id = ANY ($1)
        ) - (
            SELECT
                COALESCE(SUM(living_weight), 0)
            FROM
                landing_matrix
            WHERE
                landing_id = ANY ($1)
        )
    )::BIGINT AS "sum!"
                "#,
                &ids,
            )
            .fetch_one(&self.pool)
            .await?
            .sum;
        }

        Ok(sum)
    }

    async fn add_landing_matrix<'a>(
        &'a self,
        landing_ids: &[String],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    landing_matrix (
        landing_id,
        catch_location_id,
        catch_location_matrix_index,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight
    )
SELECT
    l.landing_id,
    MIN(c.catch_location_id),
    MIN(c.matrix_index),
    l.landing_matrix_month_bucket,
    l.vessel_length_group_id,
    l.fiskeridir_vessel_id,
    l.gear_group_id,
    e.species_group_id,
    COALESCE(SUM(e.living_weight), 0)
FROM
    UNNEST($1::TEXT[]) u (landing_id)
    INNER JOIN landings l ON l.landing_id = u.landing_id
    INNER JOIN landing_entries e ON l.landing_id = e.landing_id
    INNER JOIN catch_locations c ON l.catch_main_area_id = c.catch_main_area_id
    AND l.catch_area_id = c.catch_area_id
GROUP BY
    l.landing_id,
    e.species_group_id
ON CONFLICT (landing_id, species_group_id) DO UPDATE
SET
    living_weight = EXCLUDED.living_weight
            "#,
            landing_ids,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn landings_without_trip(&self) -> Result<i64> {
        let c = sqlx::query!(
            r#"
SELECT
    COUNT(*) AS "c!"
FROM
    vessel_events v
    INNER JOIN fiskeridir_vessels f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    vessel_event_type_id = 1
    AND f.preferred_trip_assembler = 1
    AND trip_id IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?
        .c;

        let c2 = sqlx::query!(
            r#"
SELECT
    COUNT(*) AS "c!"
FROM
    vessel_events v
    INNER JOIN fiskeridir_vessels f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN trips t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    AND v.report_timestamp <@ t.landing_coverage
WHERE
    v.vessel_event_type_id = 1
    AND v.trip_id IS NULL
    AND f.preferred_trip_assembler = 2
    -- Data before 2012 is highly unreliable, so don't include it
    AND v.report_timestamp > '2012-01-01T00:00:00Z'
    AND v.report_timestamp > (
        SELECT
            HSTORE (
                ARRAY_AGG(fiskeridir_vessel_id::TEXT),
                ARRAY_AGG(arrival_timestamp::TEXT)
            )
        FROM
            (
                SELECT
                    fiskeridir_vessel_id,
                    MIN(arrival_timestamp) AS arrival_timestamp
                FROM
                    ers_arrivals a
                WHERE
                    fiskeridir_vessel_id IS NOT NULL
                    AND arrival_timestamp > (
                        SELECT
                            departure_timestamp
                        FROM
                            ers_departures d
                        WHERE
                            d.fiskeridir_vessel_id = a.fiskeridir_vessel_id
                        ORDER BY
                            departure_timestamp
                        LIMIT
                            1
                        OFFSET
                            1
                    )
                GROUP BY
                    fiskeridir_vessel_id
            ) q
    ) [v.fiskeridir_vessel_id::TEXT]::TIMESTAMPTZ
    AND v.report_timestamp < (
        SELECT
            HSTORE (
                ARRAY_AGG(fiskeridir_vessel_id::TEXT),
                ARRAY_AGG(arrival_timestamp::TEXT)
            )
        FROM
            (
                SELECT
                    fiskeridir_vessel_id,
                    MAX(arrival_timestamp) AS arrival_timestamp
                FROM
                    ers_arrivals a
                WHERE
                    fiskeridir_vessel_id IS NOT NULL
                GROUP BY
                    fiskeridir_vessel_id
            ) q
    ) [v.fiskeridir_vessel_id::TEXT]::TIMESTAMPTZ
            "#,
        )
        .fetch_one(&self.pool)
        .await?
        .c;

        Ok(c + c2)
    }
}
