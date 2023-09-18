use crate::{
    error::PostgresError,
    models::{NewAreaGrouping, NewCatchArea, NewCatchMainArea, NewCatchMainAreaFao},
    PostgresAdapter,
};
use error_stack::{Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_catch_areas<'a>(
        &'a self,
        areas: Vec<NewCatchArea>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewCatchArea::unnest_insert(areas, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_catch_main_areas<'a>(
        &'a self,
        areas: Vec<NewCatchMainArea>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewCatchMainArea::unnest_insert(areas, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_area_groupings<'a>(
        &'a self,
        regions: Vec<NewAreaGrouping>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewAreaGrouping::unnest_insert(regions, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_catch_main_area_fao<'a>(
        &'a self,
        areas: Vec<NewCatchMainAreaFao>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewCatchMainAreaFao::unnest_insert(areas, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
