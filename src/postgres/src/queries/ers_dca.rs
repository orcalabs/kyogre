use crate::{
    error::PostgresError,
    ers_dca_set::ErsDcaSet,
    models::{
        NewErsDca, NewErsDcaCatch, NewErsDcaOther, NewErsDcaWhaleCatch, NewHerringPopulation,
    },
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_ers_dca_set(&self, set: ErsDcaSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        let mut tx = self.begin().await?;

        self.add_ers_message_types(prepared_set.ers_message_types, &mut tx)
            .await?;
        self.add_area_groupings(prepared_set.area_groupings, &mut tx)
            .await?;
        self.add_herring_populations(prepared_set.herring_populations, &mut tx)
            .await?;
        self.add_catch_main_areas(prepared_set.main_areas, &mut tx)
            .await?;
        self.add_catch_areas(prepared_set.catch_areas, &mut tx)
            .await?;
        self.add_gear_fao(prepared_set.gear_fao, &mut tx).await?;
        self.add_gear_problems(prepared_set.gear_problems, &mut tx)
            .await?;
        self.add_municipalities(prepared_set.municipalities, &mut tx)
            .await?;
        self.add_economic_zones(prepared_set.economic_zones, &mut tx)
            .await?;
        self.add_counties(prepared_set.counties, &mut tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, &mut tx)
            .await?;
        self.add_ports(prepared_set.ports, &mut tx).await?;
        self.add_species_fao(prepared_set.species_fao, &mut tx)
            .await?;
        self.add_species_fiskeridir(prepared_set.species_fiskeridir, &mut tx)
            .await?;
        self.add_ers_dca(prepared_set.ers_dca, &mut tx).await?;
        self.add_ers_dca_other(prepared_set.ers_dca_other, &mut tx)
            .await?;

        self.add_ers_dca_catches(prepared_set.catches, &mut tx)
            .await?;
        self.add_ers_dca_whale_catches(prepared_set.whale_catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_dca<'a>(
        &'a self,
        ers_dca: Vec<NewErsDca>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ers_dca.len();

        let mut message_id = Vec::with_capacity(len);
        let mut message_number = Vec::with_capacity(len);
        let mut message_timestamp = Vec::with_capacity(len);
        let mut ers_message_type_id = Vec::with_capacity(len);
        let mut message_year = Vec::with_capacity(len);
        let mut relevant_year = Vec::with_capacity(len);
        let mut sequence_number = Vec::with_capacity(len);
        let mut message_version = Vec::with_capacity(len);
        let mut ers_activity_id = Vec::with_capacity(len);
        let mut area_grouping_end_id = Vec::with_capacity(len);
        let mut area_grouping_start_id = Vec::with_capacity(len);
        let mut call_sign_of_loading_vessel = Vec::with_capacity(len);
        let mut catch_year = Vec::with_capacity(len);
        let mut duration = Vec::with_capacity(len);
        let mut economic_zone_id = Vec::with_capacity(len);
        let mut haul_distance = Vec::with_capacity(len);
        let mut herring_population_id = Vec::with_capacity(len);
        let mut herring_population_fiskeridir_id = Vec::with_capacity(len);
        let mut location_end_code = Vec::with_capacity(len);
        let mut location_start_code = Vec::with_capacity(len);
        let mut main_area_end_id = Vec::with_capacity(len);
        let mut main_area_start_id = Vec::with_capacity(len);
        let mut ocean_depth_end = Vec::with_capacity(len);
        let mut ocean_depth_start = Vec::with_capacity(len);
        let mut quota_type_id = Vec::with_capacity(len);
        let mut start_latitude = Vec::with_capacity(len);
        let mut start_longitude = Vec::with_capacity(len);
        let mut start_timestamp = Vec::with_capacity(len);
        let mut stop_latitude = Vec::with_capacity(len);
        let mut stop_longitude = Vec::with_capacity(len);
        let mut stop_timestamp = Vec::with_capacity(len);
        let mut gear_amount = Vec::with_capacity(len);
        let mut gear_fao_id = Vec::with_capacity(len);
        let mut gear_id = Vec::with_capacity(len);
        let mut gear_group_id = Vec::with_capacity(len);
        let mut gear_main_group_id = Vec::with_capacity(len);
        let mut gear_mesh_width = Vec::with_capacity(len);
        let mut gear_problem_id = Vec::with_capacity(len);
        let mut gear_specification_id = Vec::with_capacity(len);
        let mut port_id = Vec::with_capacity(len);
        let mut fiskeridir_vessel_id = Vec::with_capacity(len);
        let mut vessel_building_year = Vec::with_capacity(len);
        let mut vessel_call_sign = Vec::with_capacity(len);
        let mut vessel_call_sign_ers = Vec::with_capacity(len);
        let mut vessel_engine_building_year = Vec::with_capacity(len);
        let mut vessel_engine_power = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_1969 = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_other = Vec::with_capacity(len);
        let mut vessel_county = Vec::with_capacity(len);
        let mut vessel_county_code = Vec::with_capacity(len);
        let mut vessel_greatest_length = Vec::with_capacity(len);
        let mut vessel_identification = Vec::with_capacity(len);
        let mut vessel_length = Vec::with_capacity(len);
        let mut vessel_length_group = Vec::with_capacity(len);
        let mut vessel_length_group_code = Vec::with_capacity(len);
        let mut vessel_material_code = Vec::with_capacity(len);
        let mut vessel_municipality = Vec::with_capacity(len);
        let mut vessel_municipality_code = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut vessel_name_ers = Vec::with_capacity(len);
        let mut vessel_nationality_code = Vec::with_capacity(len);
        let mut fiskeridir_vessel_nationality_group_id = Vec::with_capacity(len);
        let mut vessel_rebuilding_year = Vec::with_capacity(len);
        let mut vessel_registration_id = Vec::with_capacity(len);
        let mut vessel_registration_id_ers = Vec::with_capacity(len);
        let mut vessel_valid_until = Vec::with_capacity(len);
        let mut vessel_width = Vec::with_capacity(len);
        let mut majority_species_fao_id = Vec::with_capacity(len);
        let mut majority_species_fiskeridir_id = Vec::with_capacity(len);

        for e in ers_dca {
            message_id.push(e.message_id);
            message_number.push(e.message_number);
            message_timestamp.push(e.message_timestamp);
            ers_message_type_id.push(e.ers_message_type_id);
            message_year.push(e.message_year);
            relevant_year.push(e.relevant_year);
            sequence_number.push(e.sequence_number);
            message_version.push(e.message_version);
            ers_activity_id.push(e.ers_activity_id);
            area_grouping_end_id.push(e.area_grouping_end_id);
            area_grouping_start_id.push(e.area_grouping_start_id);
            call_sign_of_loading_vessel.push(e.call_sign_of_loading_vessel);
            catch_year.push(e.catch_year);
            duration.push(e.duration);
            economic_zone_id.push(e.economic_zone_id);
            haul_distance.push(e.haul_distance);
            herring_population_id.push(e.herring_population_id);
            herring_population_fiskeridir_id.push(e.herring_population_fiskeridir_id);
            location_end_code.push(e.location_end_code);
            location_start_code.push(e.location_start_code);
            main_area_end_id.push(e.main_area_end_id);
            main_area_start_id.push(e.main_area_start_id);
            ocean_depth_end.push(e.ocean_depth_end);
            ocean_depth_start.push(e.ocean_depth_start);
            quota_type_id.push(e.quota_type_id);
            start_latitude.push(e.start_latitude);
            start_longitude.push(e.start_longitude);
            start_timestamp.push(e.start_timestamp);
            stop_latitude.push(e.stop_latitude);
            stop_longitude.push(e.stop_longitude);
            stop_timestamp.push(e.stop_timestamp);
            gear_amount.push(e.gear_amount);
            gear_fao_id.push(e.gear_fao_id);
            gear_id.push(e.gear_id);
            gear_group_id.push(e.gear_group_id);
            gear_main_group_id.push(e.gear_main_group_id);
            gear_mesh_width.push(e.gear_mesh_width);
            gear_problem_id.push(e.gear_problem_id);
            gear_specification_id.push(e.gear_specification_id);
            port_id.push(e.port_id);
            fiskeridir_vessel_id.push(e.fiskeridir_vessel_id);
            vessel_building_year.push(e.vessel_building_year);
            vessel_call_sign.push(e.vessel_call_sign);
            vessel_call_sign_ers.push(e.vessel_call_sign_ers);
            vessel_engine_building_year.push(e.vessel_engine_building_year);
            vessel_engine_power.push(e.vessel_engine_power);
            vessel_gross_tonnage_1969.push(e.vessel_gross_tonnage_1969);
            vessel_gross_tonnage_other.push(e.vessel_gross_tonnage_other);
            vessel_county.push(e.vessel_county);
            vessel_county_code.push(e.vessel_county_code);
            vessel_greatest_length.push(e.vessel_greatest_length);
            vessel_identification.push(e.vessel_identification);
            vessel_length.push(e.vessel_length);
            vessel_length_group.push(e.vessel_length_group);
            vessel_length_group_code.push(e.vessel_length_group_code);
            vessel_material_code.push(e.vessel_material_code);
            vessel_municipality.push(e.vessel_municipality);
            vessel_municipality_code.push(e.vessel_municipality_code);
            vessel_name.push(e.vessel_name);
            vessel_name_ers.push(e.vessel_name_ers);
            vessel_nationality_code.push(e.vessel_nationality_code);
            fiskeridir_vessel_nationality_group_id.push(e.vessel_nationality_group_id as i32);
            vessel_rebuilding_year.push(e.vessel_rebuilding_year);
            vessel_registration_id.push(e.vessel_registration_id);
            vessel_registration_id_ers.push(e.vessel_registration_id_ers);
            vessel_valid_until.push(e.vessel_valid_until);
            vessel_width.push(e.vessel_width);
            majority_species_fao_id.push(e.majority_species_fao_id);
            majority_species_fiskeridir_id.push(e.majority_species_fiskeridir_id);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_dca (
        message_id,
        message_number,
        message_timestamp,
        ers_message_type_id,
        message_year,
        relevant_year,
        sequence_number,
        message_version,
        ers_activity_id,
        area_grouping_end_id,
        area_grouping_start_id,
        call_sign_of_loading_vessel,
        catch_year,
        duration,
        economic_zone_id,
        haul_distance,
        herring_population_id,
        herring_population_fiskeridir_id,
        location_end_code,
        location_start_code,
        main_area_end_id,
        main_area_start_id,
        ocean_depth_end,
        ocean_depth_start,
        quota_type_id,
        start_latitude,
        start_longitude,
        start_timestamp,
        stop_latitude,
        stop_longitude,
        stop_timestamp,
        gear_amount,
        gear_fao_id,
        gear_id,
        gear_group_id,
        gear_main_group_id,
        gear_mesh_width,
        gear_problem_id,
        gear_specification_id,
        port_id,
        fiskeridir_vessel_id,
        vessel_building_year,
        vessel_call_sign,
        vessel_call_sign_ers,
        vessel_engine_building_year,
        vessel_engine_power,
        vessel_gross_tonnage_1969,
        vessel_gross_tonnage_other,
        vessel_county,
        vessel_county_code,
        vessel_greatest_length,
        vessel_identification,
        vessel_length,
        vessel_length_group,
        vessel_length_group_code,
        vessel_material_code,
        vessel_municipality,
        vessel_municipality_code,
        vessel_name,
        vessel_name_ers,
        vessel_nationality_code,
        fiskeridir_vessel_nationality_group_id,
        vessel_rebuilding_year,
        vessel_registration_id,
        vessel_registration_id_ers,
        vessel_valid_until,
        vessel_width,
        majority_species_fao_id,
        majority_species_fiskeridir_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::INT[],
        $3::timestamptz[],
        $4::VARCHAR[],
        $5::INT[],
        $6::INT[],
        $7::INT[],
        $8::INT[],
        $9::VARCHAR[],
        $10::VARCHAR[],
        $11::VARCHAR[],
        $12::VARCHAR[],
        $13::INT[],
        $14::INT[],
        $15::VARCHAR[],
        $16::INT[],
        $17::VARCHAR[],
        $18::INT[],
        $19::INT[],
        $20::INT[],
        $21::INT[],
        $22::INT[],
        $23::INT[],
        $24::INT[],
        $25::INT[],
        $26::DECIMAL[],
        $27::DECIMAL[],
        $28::timestamptz[],
        $29::DECIMAL[],
        $30::DECIMAL[],
        $31::timestamptz[],
        $32::INT[],
        $33::VARCHAR[],
        $34::INT[],
        $35::INT[],
        $36::INT[],
        $37::INT[],
        $38::INT[],
        $39::INT[],
        $40::VARCHAR[],
        $41::INT[],
        $42::INT[],
        $43::VARCHAR[],
        $44::VARCHAR[],
        $45::INT[],
        $46::INT[],
        $47::INT[],
        $48::INT[],
        $49::VARCHAR[],
        $50::INT[],
        $51::DECIMAL[],
        $52::VARCHAR[],
        $53::DECIMAL[],
        $54::VARCHAR[],
        $55::INT[],
        $56::VARCHAR[],
        $57::VARCHAR[],
        $58::INT[],
        $59::VARCHAR[],
        $60::VARCHAR[],
        $61::VARCHAR[],
        $62::INT[],
        $63::INT[],
        $64::VARCHAR[],
        $65::VARCHAR[],
        $66::date[],
        $67::DECIMAL[],
        $68::VARCHAR[],
        $69::INT[]
    )
ON CONFLICT (message_id, start_timestamp, stop_timestamp) DO NOTHING
            "#,
            message_id.as_slice(),
            message_number.as_slice(),
            message_timestamp.as_slice(),
            ers_message_type_id.as_slice(),
            message_year.as_slice(),
            relevant_year.as_slice(),
            sequence_number.as_slice() as _,
            message_version.as_slice(),
            ers_activity_id.as_slice(),
            area_grouping_end_id.as_slice() as _,
            area_grouping_start_id.as_slice() as _,
            call_sign_of_loading_vessel.as_slice() as _,
            catch_year.as_slice() as _,
            duration.as_slice() as _,
            economic_zone_id.as_slice() as _,
            haul_distance.as_slice() as _,
            herring_population_id.as_slice() as _,
            herring_population_fiskeridir_id.as_slice() as _,
            location_end_code.as_slice() as _,
            location_start_code.as_slice() as _,
            main_area_end_id.as_slice() as _,
            main_area_start_id.as_slice() as _,
            ocean_depth_end.as_slice() as _,
            ocean_depth_start.as_slice() as _,
            quota_type_id.as_slice(),
            start_latitude.as_slice() as _,
            start_longitude.as_slice() as _,
            start_timestamp.as_slice() as _,
            stop_latitude.as_slice() as _,
            stop_longitude.as_slice() as _,
            stop_timestamp.as_slice() as _,
            gear_amount.as_slice() as _,
            gear_fao_id.as_slice() as _,
            gear_id.as_slice() as _,
            gear_group_id.as_slice() as _,
            gear_main_group_id.as_slice() as _,
            gear_mesh_width.as_slice() as _,
            gear_problem_id.as_slice() as _,
            gear_specification_id.as_slice() as _,
            port_id.as_slice() as _,
            fiskeridir_vessel_id.as_slice() as _,
            vessel_building_year.as_slice() as _,
            vessel_call_sign.as_slice() as _,
            vessel_call_sign_ers.as_slice(),
            vessel_engine_building_year.as_slice() as _,
            vessel_engine_power.as_slice() as _,
            vessel_gross_tonnage_1969.as_slice() as _,
            vessel_gross_tonnage_other.as_slice() as _,
            vessel_county.as_slice() as _,
            vessel_county_code.as_slice() as _,
            vessel_greatest_length.as_slice() as _,
            vessel_identification.as_slice(),
            vessel_length.as_slice(),
            vessel_length_group.as_slice() as _,
            vessel_length_group_code.as_slice() as _,
            vessel_material_code.as_slice() as _,
            vessel_municipality.as_slice() as _,
            vessel_municipality_code.as_slice() as _,
            vessel_name.as_slice() as _,
            vessel_name_ers.as_slice() as _,
            vessel_nationality_code.as_slice(),
            fiskeridir_vessel_nationality_group_id.as_slice() as _,
            vessel_rebuilding_year.as_slice() as _,
            vessel_registration_id.as_slice() as _,
            vessel_registration_id_ers.as_slice() as _,
            vessel_valid_until.as_slice() as _,
            vessel_width.as_slice() as _,
            majority_species_fao_id.as_slice() as _,
            majority_species_fiskeridir_id.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    async fn add_ers_dca_other<'a>(
        &'a self,
        ers_dca: Vec<NewErsDcaOther>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ers_dca.len();

        let mut message_id = Vec::with_capacity(len);
        let mut message_number = Vec::with_capacity(len);
        let mut message_timestamp = Vec::with_capacity(len);
        let mut ers_message_type_id = Vec::with_capacity(len);
        let mut message_year = Vec::with_capacity(len);
        let mut relevant_year = Vec::with_capacity(len);
        let mut sequence_number = Vec::with_capacity(len);
        let mut message_version = Vec::with_capacity(len);
        let mut ers_activity_id = Vec::with_capacity(len);
        let mut quota_type_id = Vec::with_capacity(len);
        let mut port_id = Vec::with_capacity(len);
        let mut fiskeridir_vessel_id = Vec::with_capacity(len);
        let mut vessel_building_year = Vec::with_capacity(len);
        let mut vessel_call_sign = Vec::with_capacity(len);
        let mut vessel_call_sign_ers = Vec::with_capacity(len);
        let mut vessel_engine_building_year = Vec::with_capacity(len);
        let mut vessel_engine_power = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_1969 = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_other = Vec::with_capacity(len);
        let mut vessel_county = Vec::with_capacity(len);
        let mut vessel_county_code = Vec::with_capacity(len);
        let mut vessel_greatest_length = Vec::with_capacity(len);
        let mut vessel_identification = Vec::with_capacity(len);
        let mut vessel_length = Vec::with_capacity(len);
        let mut vessel_length_group = Vec::with_capacity(len);
        let mut vessel_length_group_code = Vec::with_capacity(len);
        let mut vessel_material_code = Vec::with_capacity(len);
        let mut vessel_municipality = Vec::with_capacity(len);
        let mut vessel_municipality_code = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut vessel_name_ers = Vec::with_capacity(len);
        let mut vessel_nationality_code = Vec::with_capacity(len);
        let mut fiskeridir_vessel_nationality_group_id = Vec::with_capacity(len);
        let mut vessel_rebuilding_year = Vec::with_capacity(len);
        let mut vessel_registration_id = Vec::with_capacity(len);
        let mut vessel_registration_id_ers = Vec::with_capacity(len);
        let mut vessel_valid_until = Vec::with_capacity(len);
        let mut vessel_width = Vec::with_capacity(len);

        for e in ers_dca {
            message_id.push(e.message_id);
            message_number.push(e.message_number);
            message_timestamp.push(e.message_timestamp);
            ers_message_type_id.push(e.ers_message_type_id);
            message_year.push(e.message_year);
            relevant_year.push(e.relevant_year);
            sequence_number.push(e.sequence_number);
            message_version.push(e.message_version);
            ers_activity_id.push(e.ers_activity_id);
            quota_type_id.push(e.quota_type_id);
            port_id.push(e.port_id);
            fiskeridir_vessel_id.push(e.fiskeridir_vessel_id);
            vessel_building_year.push(e.vessel_building_year);
            vessel_call_sign.push(e.vessel_call_sign);
            vessel_call_sign_ers.push(e.vessel_call_sign_ers);
            vessel_engine_building_year.push(e.vessel_engine_building_year);
            vessel_engine_power.push(e.vessel_engine_power);
            vessel_gross_tonnage_1969.push(e.vessel_gross_tonnage_1969);
            vessel_gross_tonnage_other.push(e.vessel_gross_tonnage_other);
            vessel_county.push(e.vessel_county);
            vessel_county_code.push(e.vessel_county_code);
            vessel_greatest_length.push(e.vessel_greatest_length);
            vessel_identification.push(e.vessel_identification);
            vessel_length.push(e.vessel_length);
            vessel_length_group.push(e.vessel_length_group);
            vessel_length_group_code.push(e.vessel_length_group_code);
            vessel_material_code.push(e.vessel_material_code);
            vessel_municipality.push(e.vessel_municipality);
            vessel_municipality_code.push(e.vessel_municipality_code);
            vessel_name.push(e.vessel_name);
            vessel_name_ers.push(e.vessel_name_ers);
            vessel_nationality_code.push(e.vessel_nationality_code);
            fiskeridir_vessel_nationality_group_id.push(e.vessel_nationality_group_id as i32);
            vessel_rebuilding_year.push(e.vessel_rebuilding_year);
            vessel_registration_id.push(e.vessel_registration_id);
            vessel_registration_id_ers.push(e.vessel_registration_id_ers);
            vessel_valid_until.push(e.vessel_valid_until);
            vessel_width.push(e.vessel_width);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_dca_other (
        message_id,
        message_number,
        message_timestamp,
        ers_message_type_id,
        message_year,
        relevant_year,
        sequence_number,
        message_version,
        ers_activity_id,
        quota_type_id,
        port_id,
        fiskeridir_vessel_id,
        vessel_building_year,
        vessel_call_sign,
        vessel_call_sign_ers,
        vessel_engine_building_year,
        vessel_engine_power,
        vessel_gross_tonnage_1969,
        vessel_gross_tonnage_other,
        vessel_county,
        vessel_county_code,
        vessel_greatest_length,
        vessel_identification,
        vessel_length,
        vessel_length_group,
        vessel_length_group_code,
        vessel_material_code,
        vessel_municipality,
        vessel_municipality_code,
        vessel_name,
        vessel_name_ers,
        vessel_nationality_code,
        fiskeridir_vessel_nationality_group_id,
        vessel_rebuilding_year,
        vessel_registration_id,
        vessel_registration_id_ers,
        vessel_valid_until,
        vessel_width
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::INT[],
        $3::timestamptz[],
        $4::VARCHAR[],
        $5::INT[],
        $6::INT[],
        $7::INT[],
        $8::INT[],
        $9::VARCHAR[],
        $10::INT[],
        $11::VARCHAR[],
        $12::INT[],
        $13::INT[],
        $14::VARCHAR[],
        $15::VARCHAR[],
        $16::INT[],
        $17::INT[],
        $18::INT[],
        $19::INT[],
        $20::VARCHAR[],
        $21::INT[],
        $22::DECIMAL[],
        $23::VARCHAR[],
        $24::DECIMAL[],
        $25::VARCHAR[],
        $26::INT[],
        $27::VARCHAR[],
        $28::VARCHAR[],
        $29::INT[],
        $30::VARCHAR[],
        $31::VARCHAR[],
        $32::VARCHAR[],
        $33::INT[],
        $34::INT[],
        $35::VARCHAR[],
        $36::VARCHAR[],
        $37::date[],
        $38::DECIMAL[]
    )
ON CONFLICT (message_id) DO NOTHING
            "#,
            message_id.as_slice(),
            message_number.as_slice(),
            message_timestamp.as_slice(),
            ers_message_type_id.as_slice(),
            message_year.as_slice(),
            relevant_year.as_slice(),
            sequence_number.as_slice() as _,
            message_version.as_slice(),
            ers_activity_id.as_slice(),
            quota_type_id.as_slice(),
            port_id.as_slice() as _,
            fiskeridir_vessel_id.as_slice() as _,
            vessel_building_year.as_slice() as _,
            vessel_call_sign.as_slice() as _,
            vessel_call_sign_ers.as_slice(),
            vessel_engine_building_year.as_slice() as _,
            vessel_engine_power.as_slice() as _,
            vessel_gross_tonnage_1969.as_slice() as _,
            vessel_gross_tonnage_other.as_slice() as _,
            vessel_county.as_slice() as _,
            vessel_county_code.as_slice() as _,
            vessel_greatest_length.as_slice() as _,
            vessel_identification.as_slice(),
            vessel_length.as_slice(),
            vessel_length_group.as_slice() as _,
            vessel_length_group_code.as_slice() as _,
            vessel_material_code.as_slice() as _,
            vessel_municipality.as_slice() as _,
            vessel_municipality_code.as_slice() as _,
            vessel_name.as_slice() as _,
            vessel_name_ers.as_slice() as _,
            vessel_nationality_code.as_slice(),
            fiskeridir_vessel_nationality_group_id.as_slice() as _,
            vessel_rebuilding_year.as_slice() as _,
            vessel_registration_id.as_slice() as _,
            vessel_registration_id_ers.as_slice() as _,
            vessel_valid_until.as_slice() as _,
            vessel_width.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    async fn add_ers_dca_catches<'a>(
        &'a self,
        catches: Vec<NewErsDcaCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = catches.len();

        let mut message_id = Vec::with_capacity(len);
        let mut start_timestamp = Vec::with_capacity(len);
        let mut stop_timestamp = Vec::with_capacity(len);
        let mut message_version = Vec::with_capacity(len);
        let mut living_weight = Vec::with_capacity(len);
        let mut species_fao_id = Vec::with_capacity(len);
        let mut species_fiskeridir_id = Vec::with_capacity(len);
        let mut species_group_id = Vec::with_capacity(len);
        let mut species_main_group_id = Vec::with_capacity(len);

        for c in catches {
            message_id.push(c.message_id);
            start_timestamp.push(c.start_timestamp);
            stop_timestamp.push(c.stop_timestamp);
            message_version.push(c.message_version);
            living_weight.push(c.living_weight);
            species_fao_id.push(c.species_fao_id);
            species_fiskeridir_id.push(c.species_fiskeridir_id.unwrap_or(0));
            species_group_id.push(c.species_group_id);
            species_main_group_id.push(c.species_main_group_id);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_dca_catches (
        message_id,
        start_timestamp,
        stop_timestamp,
        message_version,
        living_weight,
        species_fao_id,
        species_fiskeridir_id,
        species_group_id,
        species_main_group_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::timestamptz[],
        $3::timestamptz[],
        $4::INT[],
        $5::INT[],
        $6::VARCHAR[],
        $7::INT[],
        $8::INT[],
        $9::INT[]
    )
ON CONFLICT (
    message_id,
    start_timestamp,
    stop_timestamp,
    species_fao_id
) DO NOTHING
            "#,
            message_id.as_slice(),
            start_timestamp.as_slice(),
            stop_timestamp.as_slice(),
            message_version.as_slice(),
            living_weight.as_slice() as _,
            species_fao_id.as_slice(),
            species_fiskeridir_id.as_slice(),
            species_group_id.as_slice(),
            species_main_group_id.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    async fn add_ers_dca_whale_catches<'a>(
        &'a self,
        whale_catches: Vec<NewErsDcaWhaleCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = whale_catches.len();

        let mut message_id = Vec::with_capacity(len);
        let mut start_timestamp = Vec::with_capacity(len);
        let mut stop_timestamp = Vec::with_capacity(len);
        let mut message_version = Vec::with_capacity(len);
        let mut whale_grenade_number = Vec::with_capacity(len);
        let mut whale_blubber_measure_a = Vec::with_capacity(len);
        let mut whale_blubber_measure_b = Vec::with_capacity(len);
        let mut whale_blubber_measure_c = Vec::with_capacity(len);
        let mut whale_circumference = Vec::with_capacity(len);
        let mut whale_fetus_length = Vec::with_capacity(len);
        let mut whale_gender_id = Vec::with_capacity(len);
        let mut whale_individual_number = Vec::with_capacity(len);
        let mut whale_length = Vec::with_capacity(len);

        for w in whale_catches {
            message_id.push(w.message_id);
            start_timestamp.push(w.start_timestamp);
            stop_timestamp.push(w.stop_timestamp);
            message_version.push(w.message_version);
            whale_grenade_number.push(w.whale_grenade_number);
            whale_blubber_measure_a.push(w.whale_blubber_measure_a);
            whale_blubber_measure_b.push(w.whale_blubber_measure_b);
            whale_blubber_measure_c.push(w.whale_blubber_measure_c);
            whale_circumference.push(w.whale_circumference);
            whale_fetus_length.push(w.whale_fetus_length);
            whale_gender_id.push(w.whale_gender_id);
            whale_individual_number.push(w.whale_individual_number);
            whale_length.push(w.whale_length);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_dca_whale_catches (
        message_id,
        start_timestamp,
        stop_timestamp,
        message_version,
        whale_grenade_number,
        whale_blubber_measure_a,
        whale_blubber_measure_b,
        whale_blubber_measure_c,
        whale_circumference,
        whale_fetus_length,
        whale_gender_id,
        whale_individual_number,
        whale_length
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::timestamptz[],
        $3::timestamptz[],
        $4::INT[],
        $5::VARCHAR[],
        $6::INT[],
        $7::INT[],
        $8::INT[],
        $9::INT[],
        $10::INT[],
        $11::INT[],
        $12::INT[],
        $13::INT[]
    )
ON CONFLICT (
    message_id,
    start_timestamp,
    stop_timestamp,
    whale_grenade_number
) DO NOTHING
            "#,
            message_id.as_slice(),
            start_timestamp.as_slice(),
            stop_timestamp.as_slice(),
            message_version.as_slice(),
            whale_grenade_number.as_slice(),
            whale_blubber_measure_a.as_slice() as _,
            whale_blubber_measure_b.as_slice() as _,
            whale_blubber_measure_c.as_slice() as _,
            whale_circumference.as_slice() as _,
            whale_fetus_length.as_slice() as _,
            whale_gender_id.as_slice() as _,
            whale_individual_number.as_slice() as _,
            whale_length.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_herring_populations<'a>(
        &self,
        herring_populations: Vec<NewHerringPopulation>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = herring_populations.len();

        let mut ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);

        for h in herring_populations {
            ids.push(h.id);
            names.push(h.name);
        }

        sqlx::query!(
            r#"
INSERT INTO
    herring_populations (herring_population_id, "name")
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::VARCHAR[])
ON CONFLICT (herring_population_id) DO NOTHING
            "#,
            ids.as_slice(),
            names.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn delete_ers_dca_impl(&self, year: u32) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
DELETE FROM ers_dca
WHERE
    relevant_year = $1
            "#,
            year as i32
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }
}
