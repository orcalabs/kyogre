use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear, GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, StreamExt, TryStreamExt, future::ready};
use kyogre_core::*;
use sqlx::{Pool, Postgres, postgres::types::PgRange};
use tracing::instrument;

use crate::{PostgresAdapter, error::Result, models::Haul};

static ADD_HAUL_MATRIX_CHUNK_SIZE: usize = 1000;

pub static HAULS_VERIFY_CHUNK_SIZE: OnceLock<usize> = OnceLock::new();

impl PostgresAdapter {
    pub(crate) async fn haul_weights_from_range_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> Result<Vec<HaulWeight>> {
        let pg_range: PgRange<DateTime<Utc>> = range.into();
        Ok(sqlx::query_as!(
            HaulWeight,
            r#"
SELECT
    q.ranges AS "period!: DateRange",
    COALESCE(SUM(h.total_living_weight), 0.0)::DOUBLE PRECISION AS "weight!"
FROM
    hauls h
    INNER JOIN (
        SELECT
            unnest(range_agg(period)) AS ranges
        FROM
            hauls h
        WHERE
            h.fiskeridir_vessel_id = $1
            AND h.period <@ $2::tstzrange
    ) q ON q.ranges <@ h.period
WHERE
    h.fiskeridir_vessel_id = $1
    AND h.period <@ $2::tstzrange
GROUP BY
    q.ranges
            "#,
            vessel_id.into_inner(),
            pg_range
        )
        .fetch_all(&self.pool)
        .await?)
    }
    pub(crate) async fn hauls_matrix_impl(&self, query: &HaulsMatrixQuery) -> Result<HaulsMatrix> {
        let active_filter = query.active_filter;
        let args = HaulsMatrixArgs::from(query.clone());

        let j1 = tokio::spawn(PostgresAdapter::single_haul_matrix(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::Date,
        ));
        let j2 = tokio::spawn(PostgresAdapter::single_haul_matrix(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::VesselLength,
        ));
        let j3 = tokio::spawn(PostgresAdapter::single_haul_matrix(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::GearGroup,
        ));
        let j4 = tokio::spawn(PostgresAdapter::single_haul_matrix(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::SpeciesGroup,
        ));

        let (dates, length_group, gear_group, species_group) = tokio::join!(j1, j2, j3, j4);

        Ok(HaulsMatrix {
            dates: dates??,
            length_group: length_group??,
            gear_group: gear_group??,
            species_group: species_group??,
        })
    }

    pub(crate) async fn single_haul_matrix(
        pool: Pool<Postgres>,
        args: HaulsMatrixArgs,
        active_filter: ActiveHaulsFilter,
        x_feature: HaulMatrixXFeature,
    ) -> Result<Vec<u64>> {
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
    COALESCE(SUM(living_weight), 0)::BIGINT AS "sum_living!"
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
    AND (
        $9::DOUBLE PRECISION IS NULL
        OR species_group_weight_percentage_of_haul >= $9
    )
    AND (
        $10 IS FALSE
        OR is_majority_species_group_of_haul = $10
    )
GROUP BY
    1,
    2
            "#,
            x_feature as i32,
            y_feature as i32,
            args.months.empty_to_none() as _,
            args.catch_locations.empty_to_none() as Option<Vec<CatchLocationId>>,
            args.gear_group_ids.empty_to_none() as Option<Vec<GearGroup>>,
            args.species_group_ids.empty_to_none() as Option<Vec<SpeciesGroup>>,
            args.vessel_length_groups.empty_to_none() as Option<Vec<VesselLengthGroup>>,
            args.fiskeridir_vessel_ids.empty_to_none() as Option<Vec<FiskeridirVesselId>>,
            args.bycatch_percentage,
            args.majority_species_group,
        )
        .fetch_all(&pool)
        .await?;

        let table = calculate_haul_sum_area_table(x_feature, y_feature, data)?;

        Ok(table)
    }

    pub(crate) fn hauls_impl(&self, query: HaulsQuery) -> impl Stream<Item = Result<Haul>> + '_ {
        sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.haul_id AS "haul_id!: HaulId",
    e.trip_id AS "trip_id: TripId",
    h.haul_distance,
    h.catch_locations AS "catch_locations?: Vec<CatchLocationId>",
    h.species_group_ids AS "species_group_ids!: Vec<SpeciesGroup>",
    h.start_latitude AS "start_latitude!",
    h.start_longitude AS "start_longitude!",
    h.start_timestamp AS "start_timestamp!",
    h.stop_latitude AS "stop_latitude!",
    h.stop_longitude AS "stop_longitude!",
    h.stop_timestamp AS "stop_timestamp!",
    h.gear_group_id AS "gear_group_id!: GearGroup",
    h.gear_id AS "gear_id!: Gear",
    h.fiskeridir_vessel_id AS "fiskeridir_vessel_id?: FiskeridirVesselId",
    h.vessel_length_group AS "vessel_length_group!: VesselLengthGroup",
    COALESCE(h.vessel_name, h.vessel_name_ers) AS vessel_name,
    COALESCE(h.vessel_call_sign, h.vessel_call_sign_ers) AS "call_sign!: CallSign",
    h.catches::TEXT AS "catches!"
FROM
    hauls h
    LEFT JOIN vessel_events e ON h.vessel_event_id = e.vessel_event_id
WHERE
    (
        $1::tstzrange[] IS NULL
        OR h.period && ANY ($1)
    )
    AND (
        $2::TEXT[] IS NULL
        OR h.catch_locations && $2
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
        $5::INT[] IS NULL
        OR h.vessel_length_group = ANY ($5)
    )
    AND (
        $6::BIGINT[] IS NULL
        OR h.fiskeridir_vessel_id = ANY ($6)
    )
    AND (
        $7::TIMESTAMPTZ IS NULL
        OR h.start_timestamp >= $7
    )
    AND (
        $8::TIMESTAMPTZ IS NULL
        OR h.start_timestamp <= $8
    )
ORDER BY
    CASE
        WHEN $9 = 1
        AND $10 = 1 THEN h.start_timestamp
    END ASC,
    CASE
        WHEN $9 = 1
        AND $10 = 2 THEN h.stop_timestamp
    END ASC,
    CASE
        WHEN $9 = 1
        AND $10 = 3 THEN h.total_living_weight
    END ASC,
    CASE
        WHEN $9 = 2
        AND $10 = 1 THEN h.start_timestamp
    END DESC,
    CASE
        WHEN $9 = 2
        AND $10 = 2 THEN h.stop_timestamp
    END DESC,
    CASE
        WHEN $9 = 2
        AND $10 = 3 THEN h.total_living_weight
    END DESC
            "#,
            query.ranges.empty_to_none() as Option<Vec<Range<DateTime<Utc>>>>,
            query.catch_locations.empty_to_none() as Option<Vec<CatchLocationId>>,
            query.gear_group_ids.empty_to_none() as Option<Vec<GearGroup>>,
            query.species_group_ids.empty_to_none() as Option<Vec<SpeciesGroup>>,
            query.vessel_length_groups.empty_to_none() as Option<Vec<VesselLengthGroup>>,
            query.vessel_ids.empty_to_none() as Option<Vec<FiskeridirVesselId>>,
            query.start_timestamp,
            query.end_timestamp,
            query.ordering.map(|o| o as i32),
            query.sorting.map(|s| s as i32),
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn haul_messages_of_vessel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>> {
        let messages = sqlx::query_as!(
            HaulMessage,
            r#"
SELECT DISTINCT
    h.haul_id AS "haul_id!: HaulId",
    h.start_timestamp,
    h.stop_timestamp
FROM
    hauls h
    LEFT JOIN hauls_matrix m ON h.haul_id = m.haul_id
WHERE
    (
        m.haul_distribution_status IS NULL
        OR m.haul_distribution_status = $1
    )
    AND h.total_living_weight > 0
    AND h.fiskeridir_vessel_id = $2
            "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    pub(crate) async fn update_bycatch_status_impl(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
UPDATE hauls_matrix
SET
    species_group_weight_percentage_of_haul = 0.0
WHERE
    living_weight = 0
           "#
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
UPDATE hauls_matrix
SET
    species_group_weight_percentage_of_haul = q.percentage
FROM
    (
        SELECT DISTINCT
            ON (haul_id, species_group_id) haul_id,
            species_group_id,
            100 * SUM(living_weight) OVER (
                PARTITION BY
                    haul_id,
                    species_group_id
            ) / SUM(living_weight) OVER (
                PARTITION BY
                    haul_id
            ) AS percentage
        FROM
            hauls_matrix hm
        WHERE
            species_group_weight_percentage_of_haul IS NULL
    ) q
WHERE
    q.haul_id = hauls_matrix.haul_id
    AND q.species_group_id = hauls_matrix.species_group_id
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
UPDATE hauls_matrix
SET
    is_majority_species_group_of_haul = (
        q.species_group_id = hauls_matrix.species_group_id
    )
FROM
    (
        SELECT DISTINCT
            ON (haul_id) haul_id,
            species_group_id,
            SUM(living_weight) OVER (
                PARTITION BY
                    haul_id,
                    species_group_id
            ) AS weight
        FROM
            hauls_matrix hm
        WHERE
            is_majority_species_group_of_haul IS NULL
        ORDER BY
            haul_id,
            weight DESC
    ) q
WHERE
    q.haul_id = hauls_matrix.haul_id
            "#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_haul_distribution_output(
        &self,
        values: Vec<HaulDistributionOutput>,
    ) -> Result<()> {
        let len = values.len();

        let mut haul_id = Vec::with_capacity(len);
        let mut catch_location = Vec::with_capacity(len);
        let mut factor = Vec::with_capacity(len);
        let mut status = Vec::with_capacity(len);

        for v in values {
            haul_id.push(v.haul_id);
            catch_location.push(v.catch_location.into_inner());
            factor.push(v.factor);
            status.push(v.status as i32);
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
UPDATE hauls h
SET
    catch_locations = (
        SELECT
            ARRAY_AGG(DISTINCT e) FILTER (
                WHERE
                    e IS NOT NULL
            )
        FROM
            UNNEST(q.catch_locations || h.catch_location_start) e
    )
FROM
    (
        SELECT
            u.haul_id,
            ARRAY_AGG(DISTINCT u.catch_location) AS catch_locations
        FROM
            UNNEST($1::BIGINT[], $2::TEXT[]) u (haul_id, catch_location)
        GROUP BY
            u.haul_id
    ) q
WHERE
    h.haul_id = q.haul_id
            "#,
            &haul_id as &[HaulId],
            catch_location.as_slice(),
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM hauls_matrix h USING UNNEST($1::BIGINT[]) u (haul_id)
WHERE
    h.haul_id = u.haul_id
            "#,
            &haul_id as &[HaulId],
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    hauls_matrix (
        haul_id,
        catch_location_matrix_index,
        catch_location,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight,
        haul_distribution_status
    )
SELECT
    h.haul_id,
    l.matrix_index,
    l.catch_location_id,
    HAULS_MATRIX_MONTH_BUCKET (h.start_timestamp),
    TO_VESSEL_LENGTH_GROUP (h.vessel_length) AS vessel_length_group,
    h.fiskeridir_vessel_id,
    MIN(b.gear_group_id),
    b.species_group_id,
    COALESCE(SUM(b.living_weight) * MIN(u.factor), 0),
    MIN(u.haul_distribution_status)
FROM
    UNNEST(
        $1::BIGINT[],
        $2::TEXT[],
        $3::DOUBLE PRECISION[],
        $4::INT[]
    ) u (
        haul_id,
        catch_location,
        factor,
        haul_distribution_status
    )
    INNER JOIN hauls h ON h.haul_id = u.haul_id
    INNER JOIN ers_dca_bodies b ON h.message_id = b.message_id
    AND h.start_timestamp = b.start_timestamp
    AND h.stop_timestamp = b.stop_timestamp
    AND h.start_latitude = b.start_latitude
    AND h.start_longitude = b.start_longitude
    AND h.stop_latitude = b.stop_latitude
    AND h.stop_longitude = b.stop_longitude
    AND h.duration = b.duration
    AND h.haul_distance IS NOT DISTINCT FROM b.haul_distance
    AND h.gear_id = b.gear_id
    INNER JOIN catch_locations l ON u.catch_location = l.catch_location_id
GROUP BY
    h.haul_id,
    b.species_group_id,
    l.catch_location_id;
            "#,
            &haul_id as &[HaulId],
            catch_location.as_slice(),
            factor.as_slice(),
            status.as_slice(),
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn hauls_with_incorrect_catches(&self) -> Result<Vec<i64>> {
        let mut stream = sqlx::query!(
            r#"
SELECT
    haul_id,
    message_id
FROM
    hauls
            "#
        )
        .fetch(&self.pool)
        .map_ok(|r| (r.haul_id, r.message_id))
        .chunks(*HAULS_VERIFY_CHUNK_SIZE.get_or_init(|| 50_000));

        let mut incorrect_message_ids = Vec::new();

        while let Some(haul_id_chunk) = stream.next().await {
            let (haul_ids, message_ids): (Vec<i64>, Vec<i64>) =
                haul_id_chunk.into_iter().flatten().unzip();
            sqlx::query!(
                r#"
SELECT
    h.haul_id,
    COALESCE(h.message_id, c.message_id) AS "message_id!"
FROM
    (
        SELECT
            *,
            JSONB_ARRAY_ELEMENTS(catches) AS catch
        FROM
            hauls
        WHERE
            haul_id = ANY ($1)
    ) h
    LEFT JOIN (
        SELECT
            message_id,
            start_timestamp,
            stop_timestamp,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            duration,
            haul_distance,
            gear_id,
            species_fao_id,
            SUM(living_weight) AS living_weight
        FROM
            ers_dca_bodies
        WHERE
            message_id = ANY ($2)
            AND species_fao_id IS NOT NULL
        GROUP BY
            message_id,
            start_timestamp,
            stop_timestamp,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            duration,
            haul_distance,
            gear_id,
            species_fao_id
    ) c ON h.message_id = c.message_id
    AND h.start_timestamp = c.start_timestamp
    AND h.stop_timestamp = c.stop_timestamp
    AND h.start_latitude = c.start_latitude
    AND h.start_longitude = c.start_longitude
    AND h.stop_latitude = c.stop_latitude
    AND h.stop_longitude = c.stop_longitude
    AND h.duration = c.duration
    AND h.haul_distance IS NOT DISTINCT FROM c.haul_distance
    AND h.gear_id = c.gear_id
    AND h.catch ->> 'species_fao_id' = c.species_fao_id
WHERE
    h.message_id IS NULL
    OR c.message_id IS NULL
    OR (h.catch ->> 'living_weight')::INT != c.living_weight
                "#,
                &haul_ids,
                &message_ids,
            )
            .fetch(&self.pool)
            .try_for_each(|v| {
                incorrect_message_ids.push(v.message_id);
                ready(Ok(()))
            })
            .await?;
        }

        Ok(incorrect_message_ids)
    }

    pub(crate) async fn ers_dca_catches_without_haul(&self) -> Result<Vec<i64>> {
        sqlx::query!(
            r#"
SELECT
    e.message_id
FROM
    ers_dca_bodies e
    LEFT JOIN hauls h ON h.message_id = e.message_id
WHERE
    species_fao_id IS NOT NULL
    AND h.message_id IS NULL
                "#,
        )
        .fetch(&self.pool)
        .map_ok(|v| v.message_id)
        .try_collect()
        .await
        .map_err(|e| e.into())
    }

    pub(crate) async fn hauls_matrix_vs_ers_dca_living_weight(&self) -> Result<i64> {
        let row = sqlx::query!(
            r#"
SELECT
    COALESCE(
        (
            SELECT
                SUM(living_weight)
            FROM
                ers_dca_bodies
        ) - (
            SELECT
                SUM(b.living_weight)
            FROM
                ers_dca_bodies b
                LEFT JOIN hauls h ON h.message_id = b.message_id
                AND h.start_timestamp = b.start_timestamp
                AND h.stop_timestamp = b.stop_timestamp
                AND h.start_latitude = b.start_latitude
                AND h.start_longitude = b.start_longitude
                AND h.stop_latitude = b.stop_latitude
                AND h.stop_longitude = b.stop_longitude
                AND h.duration = b.duration
                AND h.haul_distance IS NOT DISTINCT FROM b.haul_distance
                AND h.gear_id = b.gear_id
                LEFT JOIN hauls_matrix m ON h.haul_id = m.haul_id
            WHERE
                m.haul_id IS NULL
        ) - (
            SELECT
                SUM(living_weight)
            FROM
                hauls_matrix
        ),
        0
    )::BIGINT AS "sum!"
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.sum)
    }

    pub(crate) async fn add_hauls<'a>(
        &'a self,
        message_ids: &[i64],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<Vec<i64>> {
        let event_ids: Vec<i64> = sqlx::query!(
            r#"
INSERT INTO
    hauls (
        message_id,
        message_timestamp,
        start_timestamp,
        stop_timestamp,
        ers_activity_id,
        duration,
        haul_distance,
        ocean_depth_end,
        ocean_depth_start,
        quota_type_id,
        start_latitude,
        start_longitude,
        stop_latitude,
        stop_longitude,
        fiskeridir_vessel_id,
        vessel_call_sign,
        vessel_call_sign_ers,
        vessel_name,
        vessel_name_ers,
        vessel_length,
        catch_location_start,
        catch_locations,
        gear_id,
        gear_group_id,
        catches,
        whale_catches
    )
SELECT
    message_id,
    MIN(message_timestamp),
    start_timestamp,
    stop_timestamp,
    MIN(ers_activity_id),
    duration,
    haul_distance,
    MIN(ocean_depth_end),
    MIN(ocean_depth_start),
    MIN(quota_type_id),
    start_latitude,
    start_longitude,
    stop_latitude,
    stop_longitude,
    MIN(fiskeridir_vessel_id),
    MIN(vessel_call_sign),
    MIN(vessel_call_sign_ers),
    MIN(vessel_name),
    MIN(vessel_name_ers),
    MIN(vessel_length),
    MIN(catch_location_id),
    CASE
        WHEN MIN(catch_location_id) IS NULL THEN NULL
        ELSE ARRAY[MIN(catch_location_id)]
    END,
    gear_id,
    MIN(gear_group_id),
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'living_weight',
                COALESCE(living_weight, 0),
                'species_fao_id',
                species_fao_id,
                'species_fiskeridir_id',
                COALESCE(species_fiskeridir_id, 0),
                'species_group_id',
                species_group_id,
                'species_main_group_id',
                species_main_group_id
            )
        ) FILTER (
            WHERE
                species_fao_id IS NOT NULL
        ),
        '[]'
    ),
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'grenade_number',
                whale_grenade_number,
                'blubber_measure_a',
                whale_blubber_measure_a,
                'blubber_measure_b',
                whale_blubber_measure_b,
                'blubber_measure_c',
                whale_blubber_measure_c,
                'circumference',
                whale_circumference,
                'fetus_length',
                whale_fetus_length,
                'gender_id',
                whale_gender_id,
                'individual_number',
                whale_individual_number,
                'length',
                whale_length
            )
        ) FILTER (
            WHERE
                whale_grenade_number IS NOT NULL
        ),
        '[]'
    )
FROM
    (
        SELECT
            e.*,
            start_timestamp,
            stop_timestamp,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            duration,
            haul_distance,
            gear_id,
            MIN(gear_group_id) AS gear_group_id,
            MIN(ocean_depth_end) AS ocean_depth_end,
            MIN(ocean_depth_start) AS ocean_depth_start,
            SUM(living_weight) AS living_weight,
            species_fao_id,
            MIN(species_fiskeridir_id) AS species_fiskeridir_id,
            MIN(species_group_id) AS species_group_id,
            MIN(species_main_group_id) AS species_main_group_id,
            MIN(whale_grenade_number) AS whale_grenade_number,
            MIN(whale_blubber_measure_a) AS whale_blubber_measure_a,
            MIN(whale_blubber_measure_b) AS whale_blubber_measure_b,
            MIN(whale_blubber_measure_c) AS whale_blubber_measure_c,
            MIN(whale_circumference) AS whale_circumference,
            MIN(whale_fetus_length) AS whale_fetus_length,
            MIN(whale_gender_id) AS whale_gender_id,
            MIN(whale_individual_number) AS whale_individual_number,
            MIN(whale_length) AS whale_length,
            MIN(catch_location_id) AS catch_location_id
        FROM
            ers_dca e
            INNER JOIN ers_dca_bodies b ON e.message_id = b.message_id
            LEFT JOIN catch_locations l ON ST_CONTAINS (
                l.polygon,
                ST_POINT (start_longitude, start_latitude)
            )
        WHERE
            e.message_id = ANY ($1::BIGINT[])
            AND (
                species_fao_id IS NOT NULL
                OR whale_grenade_number IS NOT NULL
                OR gear_id != 0
            )
        GROUP BY
            e.message_id,
            start_timestamp,
            stop_timestamp,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            duration,
            haul_distance,
            gear_id,
            species_fao_id
    ) q
GROUP BY
    message_id,
    start_timestamp,
    stop_timestamp,
    start_latitude,
    start_longitude,
    stop_latitude,
    stop_longitude,
    duration,
    haul_distance,
    gear_id
RETURNING
    vessel_event_id
            "#,
            message_ids,
        )
        .fetch(&mut **tx)
        .try_filter_map(|r| ready(Ok(r.vessel_event_id)))
        .try_collect()
        .await?;

        self.connect_trip_to_events(&event_ids, VesselEventType::Haul, tx)
            .await?;

        Ok(event_ids)
    }

    #[instrument(skip_all, fields(num_message_ids = message_ids.len()))]
    pub(crate) async fn add_hauls_matrix<'a>(
        &'a self,
        message_ids: &[i64],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        for chunk in message_ids.chunks(ADD_HAUL_MATRIX_CHUNK_SIZE) {
            sqlx::query!(
                r#"
INSERT INTO
    hauls_matrix (
        haul_id,
        catch_location_matrix_index,
        catch_location,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight
    )
SELECT
    h.haul_id,
    l.matrix_index,
    l.catch_location_id,
    HAULS_MATRIX_MONTH_BUCKET (h.start_timestamp),
    TO_VESSEL_LENGTH_GROUP (h.vessel_length) AS vessel_length_group,
    h.fiskeridir_vessel_id,
    MIN(b.gear_group_id),
    b.species_group_id,
    COALESCE(SUM(b.living_weight), 0)
FROM
    ers_dca_bodies b
    INNER JOIN hauls h ON h.message_id = b.message_id
    AND h.start_timestamp = b.start_timestamp
    AND h.stop_timestamp = b.stop_timestamp
    AND h.start_latitude = b.start_latitude
    AND h.start_longitude = b.start_longitude
    AND h.stop_latitude = b.stop_latitude
    AND h.stop_longitude = b.stop_longitude
    AND h.duration = b.duration
    AND h.haul_distance IS NOT DISTINCT FROM b.haul_distance
    AND h.gear_id = b.gear_id
    INNER JOIN catch_locations l ON h.catch_location_start = l.catch_location_id
WHERE
    b.message_id = ANY ($1::BIGINT[])
    AND HAULS_MATRIX_MONTH_BUCKET (h.start_timestamp) >= 2010 * 12
GROUP BY
    h.haul_id,
    b.species_group_id,
    l.catch_location_id;
            "#,
                chunk,
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HaulsMatrixArgs {
    pub months: Vec<i32>,
    pub catch_locations: Vec<CatchLocationId>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
    pub bycatch_percentage: Option<f64>,
    pub majority_species_group: bool,
}

impl From<HaulsMatrixQuery> for HaulsMatrixArgs {
    fn from(v: HaulsMatrixQuery) -> Self {
        let HaulsMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter: _,
            bycatch_percentage,
            majority_species_group,
        } = v;

        HaulsMatrixArgs {
            months: months.into_iter().map(|m| m as i32).collect(),
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids: vessel_ids,
            bycatch_percentage,
            majority_species_group,
        }
    }
}
