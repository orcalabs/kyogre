use super::opt_float_to_decimal;
use crate::{
    error::PostgresError,
    models::{NewAreaGrouping, NewCatchArea, NewCatchMainArea, NewCatchMainAreaFao},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_catch_areas<'a>(
        &'a self,
        areas: Vec<NewCatchArea>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = areas.len();

        let mut catch_area_id = Vec::with_capacity(len);
        let mut latitude = Vec::with_capacity(len);
        let mut longitude = Vec::with_capacity(len);

        for a in areas {
            catch_area_id.push(a.id);
            latitude.push(
                opt_float_to_decimal(a.latitude).change_context(PostgresError::DataConversion)?,
            );
            longitude.push(
                opt_float_to_decimal(a.longitude).change_context(PostgresError::DataConversion)?,
            );
        }

        sqlx::query!(
            r#"
INSERT INTO
    catch_areas (catch_area_id, latitude, longitude)
SELECT
    *
FROM
    UNNEST($1::INT[], $2::DECIMAL[], $3::DECIMAL[])
ON CONFLICT (catch_area_id) DO
UPDATE
SET
    latitude = excluded.latitude,
    longitude = excluded.longitude
WHERE
    catch_areas.latitude IS NULL
    AND catch_areas.longitude IS NULL
                "#,
            catch_area_id.as_slice(),
            latitude.as_slice() as _,
            longitude.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_catch_main_areas<'a>(
        &'a self,
        areas: Vec<NewCatchMainArea>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = areas.len();

        let mut catch_main_area_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);
        let mut latitude = Vec::with_capacity(len);
        let mut longitude = Vec::with_capacity(len);

        for a in areas {
            catch_main_area_id.push(a.id);
            name.push(a.name);
            latitude.push(
                opt_float_to_decimal(a.latitude).change_context(PostgresError::DataConversion)?,
            );
            longitude.push(
                opt_float_to_decimal(a.longitude).change_context(PostgresError::DataConversion)?,
            );
        }

        sqlx::query!(
            r#"
INSERT INTO
    catch_main_areas (catch_main_area_id, "name", latitude, longitude)
SELECT
    *
FROM
    UNNEST(
        $1::INT[],
        $2::VARCHAR[],
        $3::DECIMAL[],
        $4::DECIMAL[]
    )
ON CONFLICT (catch_main_area_id) DO
UPDATE
SET
    "name" = CASE
        WHEN catch_main_areas.name IS NULL THEN excluded.name
    END,
    latitude = excluded.latitude,
    longitude = excluded.longitude
WHERE
    catch_main_areas.latitude IS NULL
    AND catch_main_areas.longitude IS NULL
                "#,
            catch_main_area_id.as_slice(),
            name.as_slice() as _,
            latitude.as_slice() as _,
            longitude.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_area_groupings<'a>(
        &'a self,
        regions: Vec<NewAreaGrouping>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = regions.len();

        let mut fishing_region_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for a in regions {
            fishing_region_id.push(a.id);
            name.push(a.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    area_groupings (area_grouping_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (area_grouping_id) DO NOTHING
                "#,
            fishing_region_id.as_slice(),
            name.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_catch_main_area_fao<'a>(
        &'a self,
        areas: Vec<NewCatchMainAreaFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = areas.len();

        let mut catch_main_area_fao_id = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);

        for a in areas {
            catch_main_area_fao_id.push(a.id);
            name.push(a.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    catch_main_area_fao (catch_main_area_fao_id, "name")
SELECT
    *
FROM
    UNNEST($1::INT[], $2::VARCHAR[])
ON CONFLICT (catch_main_area_fao_id) DO NOTHING
                "#,
            catch_main_area_fao_id.as_slice(),
            name.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
