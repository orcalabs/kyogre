use crate::{
    error::PostgresError,
    ers_por_set::ErsPorSet,
    models::{NewErsPor, NewErsPorCatch},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_ers_por_set(&self, set: ErsPorSet) -> Result<(), PostgresError> {
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

        self.add_ers_por(prepared_set.ers_por, &mut tx).await?;

        self.add_ers_por_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_por<'a>(
        &'a self,
        ers_por: Vec<NewErsPor>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = ers_por.len();

        let mut message_id = Vec::with_capacity(len);
        let mut message_date = Vec::with_capacity(len);
        let mut message_number = Vec::with_capacity(len);
        let mut message_time = Vec::with_capacity(len);
        let mut message_timestamp = Vec::with_capacity(len);
        let mut ers_message_type_id = Vec::with_capacity(len);
        let mut message_year = Vec::with_capacity(len);
        let mut relevant_year = Vec::with_capacity(len);
        let mut sequence_number = Vec::with_capacity(len);
        let mut arrival_date = Vec::with_capacity(len);
        let mut arrival_time = Vec::with_capacity(len);
        let mut arrival_timestamp = Vec::with_capacity(len);
        let mut landing_facility = Vec::with_capacity(len);
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

        for e in ers_por {
            message_id.push(e.message_id);
            message_date.push(e.message_date);
            message_number.push(e.message_number);
            message_time.push(e.message_time);
            message_timestamp.push(e.message_timestamp);
            ers_message_type_id.push(e.ers_message_type_id);
            message_year.push(e.message_year);
            relevant_year.push(e.relevant_year);
            sequence_number.push(e.sequence_number);
            arrival_date.push(e.arrival_date);
            arrival_time.push(e.arrival_time);
            arrival_timestamp.push(e.arrival_timestamp);
            landing_facility.push(e.landing_facility);
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
    ers_arrivals (
        message_id,
        message_date,
        message_number,
        message_time,
        message_timestamp,
        ers_message_type_id,
        message_year,
        relevant_year,
        sequence_number,
        arrival_date,
        arrival_time,
        arrival_timestamp,
        landing_facility,
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
        $2::date[],
        $3::INT[],
        $4::TIME[],
        $5::timestamptz[],
        $6::VARCHAR[],
        $7::INT[],
        $8::INT[],
        $9::INT[],
        $10::date[],
        $11::TIME[],
        $12::timestamptz[],
        $13::VARCHAR[],
        $14::VARCHAR[],
        $15::INT[],
        $16::INT[],
        $17::VARCHAR[],
        $18::VARCHAR[],
        $19::INT[],
        $20::INT[],
        $21::INT[],
        $22::INT[],
        $23::VARCHAR[],
        $24::INT[],
        $25::DECIMAL[],
        $26::VARCHAR[],
        $27::DECIMAL[],
        $28::VARCHAR[],
        $29::INT[],
        $30::VARCHAR[],
        $31::VARCHAR[],
        $32::INT[],
        $33::VARCHAR[],
        $34::VARCHAR[],
        $35::VARCHAR[],
        $36::INT[],
        $37::INT[],
        $38::VARCHAR[],
        $39::VARCHAR[],
        $40::date[],
        $41::DECIMAL[]
    )
ON CONFLICT (message_id) DO NOTHING
            "#,
            message_id.as_slice(),
            message_date.as_slice(),
            message_number.as_slice(),
            message_time.as_slice(),
            message_timestamp.as_slice(),
            ers_message_type_id.as_slice(),
            message_year.as_slice(),
            relevant_year.as_slice(),
            sequence_number.as_slice() as _,
            arrival_date.as_slice(),
            arrival_time.as_slice(),
            arrival_timestamp.as_slice(),
            landing_facility.as_slice() as _,
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

    pub(crate) async fn add_ers_por_catches<'a>(
        &self,
        catches: Vec<NewErsPorCatch>,
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
    ers_arrival_catches (
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
ON CONFLICT (message_id, species_fao_id) DO NOTHING
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
}
