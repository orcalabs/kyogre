use crate::{
    error::PostgresError,
    ers_dep_set::ErsDepSet,
    models::{NewErsDep, NewErsDepCatch},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_ers_dep_set(&self, set: ErsDepSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        let mut tx = self.begin().await?;

        self.add_ers_message_types(prepared_set.ers_message_types, &mut tx)
            .await?;
        self.add_species_fao(prepared_set.species_fao, &mut tx)
            .await?;
        self.add_species_fiskeridir(prepared_set.species_fiskeridir, &mut tx)
            .await?;
        self.add_municipalities(prepared_set.municipalities, &mut tx)
            .await?;
        self.add_counties(prepared_set.counties, &mut tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, &mut tx)
            .await?;
        self.add_ports(prepared_set.ports, &mut tx).await?;
        self.add_species_groups(prepared_set.species_groups, &mut tx)
            .await?;
        self.add_species_main_groups(prepared_set.species_main_groups, &mut tx)
            .await?;

        self.add_ers_dep(prepared_set.ers_dep, &mut tx).await?;

        self.add_ers_dep_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_dep<'a>(
        &'a self,
        ers_dep: Vec<NewErsDep>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ers_dep.len();

        let mut message_id = Vec::with_capacity(len);
        let mut message_number = Vec::with_capacity(len);
        let mut message_timestamp = Vec::with_capacity(len);
        let mut ers_message_type_id = Vec::with_capacity(len);
        let mut message_year = Vec::with_capacity(len);
        let mut relevant_year = Vec::with_capacity(len);
        let mut sequence_number = Vec::with_capacity(len);
        let mut ers_activity_id = Vec::with_capacity(len);
        let mut departure_timestamp = Vec::with_capacity(len);
        let mut fishing_timestamp = Vec::with_capacity(len);
        let mut start_latitude = Vec::with_capacity(len);
        let mut start_latitude_sggdd = Vec::with_capacity(len);
        let mut start_longitude = Vec::with_capacity(len);
        let mut start_longitude_sggdd = Vec::with_capacity(len);
        let mut target_species_fao_id = Vec::with_capacity(len);
        let mut target_species_fiskeridir_id = Vec::with_capacity(len);
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

        for e in ers_dep {
            message_id.push(e.message_id);
            message_number.push(e.message_number);
            message_timestamp.push(e.message_timestamp);
            ers_message_type_id.push(e.ers_message_type_id);
            message_year.push(e.message_year);
            relevant_year.push(e.relevant_year);
            sequence_number.push(e.sequence_number);
            ers_activity_id.push(e.ers_activity_id);
            departure_timestamp.push(e.departure_timestamp);
            fishing_timestamp.push(e.fishing_timestamp);
            start_latitude.push(e.start_latitude);
            start_latitude_sggdd.push(e.start_latitude_sggdd);
            start_longitude.push(e.start_longitude);
            start_longitude_sggdd.push(e.start_longitude_sggdd);
            target_species_fao_id.push(e.target_species_fao_id);
            target_species_fiskeridir_id.push(e.target_species_fiskeridir_id);
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
    ers_departures (
        message_id,
        message_number,
        message_timestamp,
        ers_message_type_id,
        message_year,
        relevant_year,
        sequence_number,
        ers_activity_id,
        departure_timestamp,
        fishing_timestamp,
        start_latitude,
        start_latitude_sggdd,
        start_longitude,
        start_longitude_sggdd,
        target_species_fao_id,
        target_species_fiskeridir_id,
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
        $8::VARCHAR[],
        $9::timestamptz[],
        $10::timestamptz[],
        $11::DECIMAL[],
        $12::VARCHAR[],
        $13::DECIMAL[],
        $14::VARCHAR[],
        $15::VARCHAR[],
        $16::INT[],
        $17::VARCHAR[],
        $18::INT[],
        $19::INT[],
        $20::VARCHAR[],
        $21::VARCHAR[],
        $22::INT[],
        $23::INT[],
        $24::INT[],
        $25::INT[],
        $26::VARCHAR[],
        $27::INT[],
        $28::DECIMAL[],
        $29::VARCHAR[],
        $30::DECIMAL[],
        $31::VARCHAR[],
        $32::INT[],
        $33::VARCHAR[],
        $34::VARCHAR[],
        $35::INT[],
        $36::VARCHAR[],
        $37::VARCHAR[],
        $38::VARCHAR[],
        $39::INT[],
        $40::INT[],
        $41::VARCHAR[],
        $42::VARCHAR[],
        $43::date[],
        $44::DECIMAL[]
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
            ers_activity_id.as_slice() as _,
            departure_timestamp.as_slice(),
            fishing_timestamp.as_slice(),
            start_latitude.as_slice(),
            start_latitude_sggdd.as_slice(),
            start_longitude.as_slice(),
            start_longitude_sggdd.as_slice(),
            target_species_fao_id.as_slice(),
            target_species_fiskeridir_id.as_slice() as _,
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

    pub(crate) async fn add_ers_dep_catches<'a>(
        &self,
        catches: Vec<NewErsDepCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = catches.len();

        let mut message_id = Vec::with_capacity(len);
        let mut ers_quantum_type_id = Vec::with_capacity(len);
        let mut living_weight = Vec::with_capacity(len);
        let mut species_fao_id = Vec::with_capacity(len);
        let mut species_fiskeridir_id = Vec::with_capacity(len);
        let mut species_group_id = Vec::with_capacity(len);
        let mut species_main_group_id = Vec::with_capacity(len);

        for c in catches {
            message_id.push(c.message_id);
            ers_quantum_type_id.push(c.ers_quantum_type_id);
            living_weight.push(c.living_weight);
            species_fao_id.push(c.species_fao_id);
            species_fiskeridir_id.push(c.species_fiskeridir_id);
            species_group_id.push(c.species_group_id);
            species_main_group_id.push(c.species_main_group_id);
        }

        sqlx::query!(
            r#"
INSERT INTO
    ers_departure_catches (
        message_id,
        ers_quantum_type_id,
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
        $2::VARCHAR[],
        $3::INT[],
        $4::VARCHAR[],
        $5::INT[],
        $6::INT[],
        $7::INT[]
    )
            "#,
            message_id.as_slice(),
            ers_quantum_type_id.as_slice() as _,
            living_weight.as_slice() as _,
            species_fao_id.as_slice() as _,
            species_fiskeridir_id.as_slice() as _,
            species_group_id.as_slice() as _,
            species_main_group_id.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn delete_ers_dep_catches_impl(&self, year: u32) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
DELETE FROM ers_departure_catches c USING ers_departures e
WHERE
    e.message_id = c.message_id
    AND e.relevant_year = $1
            "#,
            year as i32
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
