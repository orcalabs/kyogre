use super::opt_float_to_decimal;
use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_vessels_from_landings<'a>(
        &'a self,
        vessels: Vec<fiskeridir_rs::Vessel>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = vessels.len();
        let mut fiskeridir_vessel_id = Vec::with_capacity(len);
        let mut call_signs = Vec::with_capacity(len);
        let mut registration_ids = Vec::with_capacity(len);
        let mut names = Vec::with_capacity(len);
        let mut lengths = Vec::with_capacity(len);
        let mut building_years = Vec::with_capacity(len);
        let mut engine_powers = Vec::with_capacity(len);
        let mut engine_building_years = Vec::with_capacity(len);
        let mut fiskedir_vessel_types = Vec::with_capacity(len);
        let mut norwegian_municipality_ids = Vec::with_capacity(len);
        let mut norwegian_county_ids = Vec::with_capacity(len);
        let mut fiskedir_nation_group_ids = Vec::with_capacity(len);
        let mut nation_ids = Vec::with_capacity(len);
        let mut gross_tonnage_1969 = Vec::with_capacity(len);
        let mut gross_tonnage_other = Vec::with_capacity(len);
        let mut rebuilding_year = Vec::with_capacity(len);
        let mut fiskeridir_length_group_id = Vec::with_capacity(len);

        for v in vessels {
            if let Some(vessel_id) = v.id {
                fiskeridir_vessel_id.push(vessel_id);
                call_signs.push(v.call_sign.map(|c| c.into_inner()));
                registration_ids.push(v.registration_id);
                names.push(v.name);
                lengths.push(
                    opt_float_to_decimal(v.length).change_context(PostgresError::DataConversion)?,
                );
                building_years.push(v.building_year.map(|b| b as i32));
                engine_powers.push(v.engine_power.map(|e| e as i32));
                engine_building_years.push(v.engine_building_year.map(|e| e as i32));
                fiskedir_vessel_types.push(v.type_code.map(|v| v as i32));
                norwegian_municipality_ids.push(v.municipality_code.map(|v| v as i32));
                norwegian_county_ids.push(v.county_code.map(|v| v as i32));
                fiskedir_nation_group_ids.push(v.nation_group);
                nation_ids.push(v.nationality_code.alpha3().to_string());
                gross_tonnage_1969.push(v.gross_tonnage_1969.map(|v| v as i32));
                gross_tonnage_other.push(v.gross_tonnage_other.map(|v| v as i32));
                rebuilding_year.push(v.rebuilding_year.map(|v| v as i32));
                fiskeridir_length_group_id.push(v.length_group_code.map(|v| v as i32));
            }
        }

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_vessels (
        fiskeridir_vessel_id,
        call_sign,
        registration_id,
        "name",
        "length",
        building_year,
        engine_power,
        engine_building_year,
        fiskeridir_vessel_type_id,
        norwegian_municipality_id,
        norwegian_county_id,
        fiskeridir_nation_group_id,
        nation_id,
        gross_tonnage_1969,
        gross_tonnage_other,
        rebuilding_year,
        fiskeridir_length_group_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::VARCHAR[],
        $3::VARCHAR[],
        $4::VARCHAR[],
        $5::DECIMAL[],
        $6::INT[],
        $7::INT[],
        $8::INT[],
        $9::INT[],
        $10::INT[],
        $11::INT[],
        $12::VARCHAR[],
        $13::VARCHAR[],
        $14::INT[],
        $15::INT[],
        $16::INT[],
        $17::INT[]
    )
ON CONFLICT (fiskeridir_vessel_id) DO NOTHING
            "#,
            fiskeridir_vessel_id.as_slice(),
            call_signs.as_slice() as _,
            registration_ids.as_slice() as _,
            names.as_slice() as _,
            lengths.as_slice() as _,
            building_years.as_slice() as _,
            engine_powers.as_slice() as _,
            engine_building_years.as_slice() as _,
            fiskedir_vessel_types.as_slice() as _,
            norwegian_municipality_ids.as_slice() as _,
            norwegian_county_ids.as_slice() as _,
            fiskedir_nation_group_ids.as_slice() as _,
            nation_ids.as_slice(),
            gross_tonnage_1969.as_slice() as _,
            gross_tonnage_other.as_slice() as _,
            rebuilding_year.as_slice() as _,
            fiskeridir_length_group_id.as_slice() as _,
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
