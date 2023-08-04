use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::{FiskeridirVesselId, LandingsQuery};
use sqlx::{postgres::types::PgRange, Row};
use std::collections::HashSet;
use unnest_insert::UnnestInsert;

use crate::models::NewLandingEntry;
use crate::{
    error::{ErrorWrapper, PostgresError},
    landing_set::LandingSet,
    models::{Landing, NewLanding},
    PostgresAdapter,
};
use error_stack::{report, IntoReport, Report, Result, ResultExt};
use fiskeridir_rs::{LandingId, VesselLengthGroup};

use super::bound_float_to_decimal;

impl PostgresAdapter {
    pub(crate) fn landings_impl(
        &self,
        query: LandingsQuery,
    ) -> Result<impl Stream<Item = Result<Landing, PostgresError>> + '_, PostgresError> {
        let args = LandingsArgs::try_from(query)?;

        let stream = sqlx::query_as!(
            Landing,
            r#"
SELECT
    l.landing_id,
    l.landing_timestamp,
    l.catch_area_id,
    l.catch_main_area_id,
    l.gear_id,
    l.gear_group_id,
    l.fiskeridir_vessel_id,
    l.vessel_call_sign,
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
    )::TEXT AS "catches!"
FROM
    landings l
    INNER JOIN landing_entries le ON l.landing_id = le.landing_id
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
        $5::numrange[] IS NULL
        OR l.vessel_length <@ ANY ($5::numrange[])
    )
    AND (
        $6::BIGINT[] IS NULL
        OR fiskeridir_vessel_id = ANY ($6)
    )
GROUP BY
    l.landing_id
HAVING
    (
        $7::INT[] IS NULL
        OR ARRAY_AGG(le.species_group_id) && $7
    )
ORDER BY
    CASE
        WHEN $8 = 1
        AND $9 = 1 THEN l.landing_timestamp
    END ASC,
    CASE
        WHEN $8 = 1
        AND $9 = 2 THEN SUM(le.living_weight)
    END ASC,
    CASE
        WHEN $8 = 2
        AND $9 = 1 THEN l.landing_timestamp
    END DESC,
    CASE
        WHEN $8 = 2
        AND $9 = 2 THEN SUM(le.living_weight)
    END DESC
            "#,
            args.ranges,
            args.catch_area_ids as _,
            args.catch_main_area_ids as _,
            args.gear_group_ids as _,
            args.vessel_length_ranges as _,
            args.fiskeridir_vessel_ids as _,
            args.species_group_ids as _,
            args.ordering,
            args.sorting,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn sum_landing_weight_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Option<f64>, PostgresError> {
        let weight = sqlx::query!(
            r#"
SELECT
    SUM(le.living_weight) AS weight
FROM
    landings AS l
    INNER JOIN landing_entries AS le ON l.landing_id = le.landing_id
WHERE
    fiskeridir_vessel_id = $1
            "#,
            id.0,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(weight
            .weight
            .map(|v| v.to_f64().ok_or(PostgresError::DataConversion))
            .transpose()?)
    }
    pub(crate) async fn add_landing_set(&self, set: LandingSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();
        let mut tx = self.begin().await?;

        self.add_delivery_points(prepared_set.delivery_points, &mut tx)
            .await?;

        self.add_municipalities(prepared_set.municipalities, &mut tx)
            .await?;
        self.add_counties(prepared_set.counties, &mut tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, &mut tx)
            .await?;

        self.add_species_fiskeridir(prepared_set.species_fiskeridir, &mut tx)
            .await?;
        self.add_species(prepared_set.species, &mut tx).await?;
        self.add_species_fao(prepared_set.species_fao, &mut tx)
            .await?;
        self.add_catch_areas(prepared_set.catch_areas, &mut tx)
            .await?;
        self.add_catch_main_areas(prepared_set.catch_main_areas, &mut tx)
            .await?;
        self.add_catch_main_area_fao(prepared_set.catch_main_area_fao, &mut tx)
            .await?;
        self.add_area_groupings(prepared_set.area_groupings, &mut tx)
            .await?;
        self.add_landings(prepared_set.landings, &mut tx).await?;
        self.add_landing_entries(prepared_set.landing_entries, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_landings<'a>(
        &'a self,
        landings: Vec<NewLanding>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewLanding::unnest_insert(landings, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_landing_entries<'a>(
        &'a self,
        entries: Vec<NewLandingEntry>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewLandingEntry::unnest_insert(entries, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn delete_removed_landings_impl(
        &self,
        existing_landing_ids: HashSet<LandingId>,
        data_year: u32,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        let ids: Vec<String> = existing_landing_ids
            .into_iter()
            .map(|v| v.into_inner())
            .collect();

        // With the naive sql approach `DELETE WHERE landing_id NOT IN (ALL($1)) the query takes
        // forever as the input vector will be about 360k long.
        // Instead we create a temporary table and index it and do a join operation to filter the
        // rows to delete.
        // (see https://dba.stackexchange.com/questions/91247/optimizing-a-postgres-query-with-a-large-in/91539#91539 for more details)
        // SQLX does not like us referencing temporary tables in the query macros for type checking
        // so we use the normal versions here.
        sqlx::query(
            r#"
CREATE TEMPORARY TABLE
    existing_landing_ids (landing_id VARCHAR NOT NULL, data_year int not null, PRIMARY KEY (landing_id, data_year)) ON
COMMIT
DROP;
            "#,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        sqlx::query(
            r#"
INSERT INTO
    existing_landing_ids (landing_id, data_year) (
        SELECT
            ids, $2
        FROM
            UNNEST($1::VARCHAR[]) as ids
    );
            "#,
        )
        .bind(ids.as_slice())
        .bind(data_year as i32)
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        let rows = sqlx::query(
            r#"
SELECT
    l.landing_id
FROM
    landings AS l
    LEFT JOIN existing_landing_ids AS e
        ON l.landing_id = e.landing_id
        AND l.data_year = e.data_year
WHERE
    e.landing_id IS NULL
AND
    l.data_year = $1
            "#,
        )
        .bind(data_year as i32)
        .fetch_all(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        let mut ids_to_delete = Vec::with_capacity(rows.len());

        for r in rows {
            let id = r
                .try_get_raw(0)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .as_str()
                .map_err(|e| ErrorWrapper(e.to_string()))
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .to_string();
            ids_to_delete.push(id)
        }

        tracing::Span::current().record("landings_to_delete", ids_to_delete.len());

        sqlx::query!(
            r#"
DELETE FROM landings
WHERE
    landing_id = ANY ($1)
            "#,
            &ids_to_delete,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)
    }
}

pub struct LandingsArgs {
    pub ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub catch_area_ids: Option<Vec<i32>>,
    pub catch_main_area_ids: Option<Vec<i32>>,
    pub gear_group_ids: Option<Vec<i32>>,
    pub species_group_ids: Option<Vec<i32>>,
    pub vessel_length_ranges: Option<Vec<PgRange<BigDecimal>>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
    pub sorting: Option<i32>,
    pub ordering: Option<i32>,
}

impl TryFrom<LandingsQuery> for LandingsArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: LandingsQuery) -> std::result::Result<Self, Self::Error> {
        let (catch_area_ids, catch_main_area_ids) = if let Some(cls) = v.catch_locations {
            let mut catch_areas = Vec::with_capacity(cls.len());
            let mut main_areas = Vec::with_capacity(cls.len());

            for c in cls {
                catch_areas.push(c.catch_area());
                main_areas.push(c.main_area());
            }

            (Some(catch_areas), Some(main_areas))
        } else {
            (None, None)
        };

        Ok(LandingsArgs {
            ranges: v.ranges.map(|ranges| {
                ranges
                    .into_iter()
                    .map(|m| PgRange {
                        start: m.start,
                        end: m.end,
                    })
                    .collect()
            }),
            catch_area_ids,
            catch_main_area_ids,
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
