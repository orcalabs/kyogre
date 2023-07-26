use crate::models::LandingMatrixQueryOutput;
use crate::{error::PostgresError, models::LandingMatrixArgs, PostgresAdapter};
use error_stack::IntoReport;
use error_stack::{Result, ResultExt};
use kyogre_core::{
    calculate_landing_sum_area_table, ActiveLandingFilter, LandingMatrixXFeature,
    LandingMatrixYFeature,
};
use sqlx::{Pool, Postgres};

impl PostgresAdapter {
    pub(crate) async fn landing_matrix_impl(
        pool: Pool<Postgres>,
        args: LandingMatrixArgs,
        active_filter: ActiveLandingFilter,
        x_feature: LandingMatrixXFeature,
    ) -> Result<Vec<u64>, PostgresError> {
        let y_feature = if x_feature == active_filter {
            LandingMatrixYFeature::CatchLocation
        } else {
            LandingMatrixYFeature::from(active_filter)
        };

        let data: Vec<LandingMatrixQueryOutput> = sqlx::query_as!(
            LandingMatrixQueryOutput,
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
    landing_matrix h
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
        OR h.catch_location_id = ANY ($4)
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
GROUP BY
    1,
    2
            "#,
            x_feature as i32,
            y_feature as i32,
            args.months as _,
            args.catch_locations as _,
            args.gear_group_ids as _,
            args.species_group_ids as _,
            args.vessel_length_groups as _,
            args.fiskeridir_vessel_ids as _,
        )
        .fetch_all(&pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        let converted: Vec<kyogre_core::LandingMatrixQueryOutput> = data
            .into_iter()
            .map(kyogre_core::LandingMatrixQueryOutput::from)
            .collect();

        calculate_landing_sum_area_table(x_feature, y_feature, converted)
            .change_context(PostgresError::DataConversion)
    }
}
