use std::ops::Bound;

use crate::{error::PostgresError, models::Haul, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::HaulsQuery;
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn hauls(&self, query: HaulsQuery) -> Result<Vec<Haul>, PostgresError> {
        let mut conn = self.acquire().await?;

        let ranges = query.ranges.map(|ranges| {
            ranges
                .into_iter()
                .map(|m| PgRange {
                    start: Bound::Included(m.start()),
                    end: Bound::Included(m.end()),
                })
                .collect::<Vec<_>>()
        });

        sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.message_id AS "message_id!",
    h.message_date AS "message_date!",
    h.message_number AS "message_number!",
    h.message_time AS "message_time!",
    h.message_timestamp AS "message_timestamp!",
    h.ers_message_type_id AS "ers_message_type_id!",
    h.message_year AS "message_year!",
    h.relevant_year AS "relevant_year!",
    h.sequence_number AS sequence_number,
    h.message_version AS "message_version!",
    h.ers_activity_id AS "ers_activity_id!",
    h.area_grouping_end_id AS area_grouping_end_id,
    h.area_grouping_start_id AS area_grouping_start_id,
    h.call_sign_of_loading_vessel AS call_sign_of_loading_vessel,
    h.catch_year AS catch_year,
    h.duration AS duration,
    h.economic_zone_id AS economic_zone_id,
    h.haul_distance AS haul_distance,
    h.herring_population_id AS herring_population_id,
    h.herring_population_fiskeridir_id AS herring_population_fiskeridir_id,
    h.location_end_code AS location_end_code,
    h.location_start_code AS location_start_code,
    h.main_area_end_id AS main_area_end_id,
    h.main_area_start_id AS main_area_start_id,
    h.ocean_depth_end AS ocean_depth_end,
    h.ocean_depth_start AS ocean_depth_start,
    h.quota_type_id AS "quota_type_id!",
    h.start_date AS start_date,
    h.start_latitude AS start_latitude,
    h.start_longitude AS start_longitude,
    h.start_time AS start_time,
    h.start_timestamp AS start_timestamp,
    h.stop_date AS stop_date,
    h.stop_latitude AS stop_latitude,
    h.stop_longitude AS stop_longitude,
    h.stop_time AS stop_time,
    h.stop_timestamp AS stop_timestamp,
    h.gear_amount AS gear_amount,
    h.gear_fao_id AS gear_fao_id,
    h.gear_fiskeridir_id AS gear_fiskeridir_id,
    h.gear_group_id AS gear_group_id,
    h.gear_main_group_id AS gear_main_group_id,
    h.gear_mesh_width AS gear_mesh_width,
    h.gear_problem_id AS gear_problem_id,
    h.gear_specification_id AS gear_specification_id,
    h.port_id AS port_id,
    h.fiskeridir_vessel_id AS fiskeridir_vessel_id,
    h.vessel_building_year AS vessel_building_year,
    h.vessel_call_sign AS vessel_call_sign,
    h.vessel_call_sign_ers AS "vessel_call_sign_ers!",
    h.vessel_engine_building_year AS vessel_engine_building_year,
    h.vessel_engine_power AS vessel_engine_power,
    h.vessel_gross_tonnage_1969 AS vessel_gross_tonnage_1969,
    h.vessel_gross_tonnage_other AS vessel_gross_tonnage_other,
    h.vessel_county AS vessel_county,
    h.vessel_county_code AS vessel_county_code,
    h.vessel_greatest_length AS vessel_greatest_length,
    h.vessel_identification AS "vessel_identification!",
    h.vessel_length AS "vessel_length!",
    h.vessel_length_group AS vessel_length_group,
    h.vessel_length_group_code AS vessel_length_group_code,
    h.vessel_material_code AS vessel_material_code,
    h.vessel_municipality AS vessel_municipality,
    h.vessel_municipality_code AS vessel_municipality_code,
    h.vessel_name AS vessel_name,
    h.vessel_name_ers AS vessel_name_ers,
    h.vessel_nationality_code AS "vessel_nationality_code!",
    h.vessel_nationality_group_id AS "vessel_nationality_group_id!",
    h.vessel_rebuilding_year AS vessel_rebuilding_year,
    h.vessel_registration_id AS vessel_registration_id,
    h.vessel_registration_id_ers AS vessel_registration_id_ers,
    h.vessel_valid_until AS vessel_valid_until,
    h.vessel_width AS vessel_width,
    h.catches AS "catches!",
    h.whale_catches AS "whale_catches!"
FROM
    hauls_view h
WHERE
    (
        $1::tstzrange[] IS NULL
        OR tstzrange (h.start_timestamp, h.stop_timestamp, '[]') && ANY ($1)
    )
            "#,
            ranges
        )
        .fetch_all(&mut conn)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
