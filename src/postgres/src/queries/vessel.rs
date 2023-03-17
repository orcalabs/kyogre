use super::{float_to_decimal, opt_float_to_decimal};
use crate::{error::PostgresError, models::FiskeridirAisVesselCombination, PostgresAdapter};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::{FiskeridirVesselId, FiskeridirVesselSource};

impl PostgresAdapter {
    pub(crate) async fn add_fiskeridir_vessels<'a>(
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
        let mut fiskeridir_vessel_types = Vec::with_capacity(len);
        let mut norwegian_municipality_ids = Vec::with_capacity(len);
        let mut norwegian_county_ids = Vec::with_capacity(len);
        let mut fiskeridir_nation_group_ids = Vec::with_capacity(len);
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
                fiskeridir_vessel_types.push(v.type_code.map(|v| v as i32));
                norwegian_municipality_ids.push(v.municipality_code.map(|v| v as i32));
                norwegian_county_ids.push(v.county_code.map(|v| v as i32));
                fiskeridir_nation_group_ids.push(v.nation_group);
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
            fiskeridir_vessel_types.as_slice() as _,
            norwegian_municipality_ids.as_slice() as _,
            norwegian_county_ids.as_slice() as _,
            fiskeridir_nation_group_ids.as_slice() as _,
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

    pub(crate) async fn add_register_vessels_impl(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<(), PostgresError> {
        let len = vessels.len();

        let mut id = Vec::with_capacity(len);
        let mut engine_power = Vec::with_capacity(len);
        let mut imo_number = Vec::with_capacity(len);
        let mut length = Vec::with_capacity(len);
        let mut municipality_code = Vec::with_capacity(len);
        let mut name = Vec::with_capacity(len);
        let mut radio_call_sign = Vec::with_capacity(len);
        let mut registration_mark = Vec::with_capacity(len);
        let mut width = Vec::with_capacity(len);
        let mut owners = Vec::with_capacity(len);
        let mut source_ids = Vec::with_capacity(len);

        for v in vessels {
            id.push(v.id);
            engine_power.push(v.engine_power);
            imo_number.push(v.imo_number);
            length.push(float_to_decimal(v.length).change_context(PostgresError::DataConversion)?);
            municipality_code.push(v.municipality_code);
            name.push(v.name);
            radio_call_sign.push(v.radio_call_sign.map(|c| c.into_inner()));
            registration_mark.push(v.registration_mark);
            width
                .push(opt_float_to_decimal(v.width).change_context(PostgresError::DataConversion)?);
            owners.push(
                serde_json::to_value(&v.owners)
                    .into_report()
                    .change_context(PostgresError::DataConversion)
                    .attach_printable_lazy(|| {
                        format!("could not serialize vessel owners: {:?}", v.owners)
                    })?,
            );
            source_ids.push(FiskeridirVesselSource::FiskeridirVesselRegister as i32);
        }

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_vessels (
        fiskeridir_vessel_id,
        norwegian_municipality_id,
        call_sign,
        "name",
        registration_id,
        "length",
        "width",
        engine_power,
        imo_number,
        owners,
        fiskeridir_vessel_source_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::BIGINT[],
        $2::INT[],
        $3::VARCHAR[],
        $4::VARCHAR[],
        $5::VARCHAR[],
        $6::DECIMAL[],
        $7::DECIMAL[],
        $8::INT[],
        $9::BIGINT[],
        $10::JSON[],
        $11::INT[]
    )
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    norwegian_municipality_id = EXCLUDED.norwegian_municipality_id,
    call_sign = EXCLUDED.call_sign,
    "name" = EXCLUDED.name,
    registration_id = EXCLUDED.registration_id,
    "length" = EXCLUDED.length,
    "width" = EXCLUDED.width,
    engine_power = EXCLUDED.engine_power,
    imo_number = EXCLUDED.imo_number,
    owners = EXCLUDED.owners,
    fiskeridir_vessel_source_id = EXCLUDED.fiskeridir_vessel_source_id
            "#,
            id.as_slice(),
            municipality_code.as_slice(),
            radio_call_sign.as_slice() as _,
            name.as_slice(),
            registration_mark.as_slice(),
            length.as_slice(),
            width.as_slice() as _,
            engine_power.as_slice() as _,
            imo_number.as_slice() as _,
            owners.as_slice(),
            source_ids.as_slice(),
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations(
        &self,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination, PostgresError>> + '_ {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    f.fiskeridir_vessel_type_id,
    f.fiskeridir_length_group_id,
    f.fiskeridir_nation_group_id,
    f.norwegian_municipality_id AS fiskeridir_norwegian_municipality_id,
    f.norwegian_county_id AS fiskeridir_norwegian_county_id,
    f.nation_id AS "fiskeridir_nation_id!",
    f.gross_tonnage_1969 AS fiskeridir_gross_tonnage_1969,
    f.gross_tonnage_other AS fiskeridir_gross_tonnage_other,
    f.call_sign AS fiskeridir_call_sign,
    f."name" AS fiskeridir_name,
    f.registration_id AS fiskeridir_registration_id,
    f."length" AS fiskeridir_length,
    f."width" AS fiskeridir_width,
    f."owner" AS fiskeridir_owner,
    f.engine_building_year AS fiskeridir_engine_building_year,
    f.engine_power AS fiskeridir_engine_power,
    f.building_year AS fiskeridir_building_year,
    f.rebuilding_year AS fiskeridir_rebuilding_year,
    a.mmsi AS "ais_mmsi?",
    a.imo_number AS ais_imo_number,
    a.call_sign AS ais_call_sign,
    a.name AS ais_name,
    a.ship_length AS ais_ship_length,
    a.ship_width AS ais_ship_width,
    a.eta AS ais_eta,
    a.destination AS ais_destination
FROM
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn single_fiskeridir_ais_vessel_combination(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Option<FiskeridirAisVesselCombination>, PostgresError> {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    f.fiskeridir_vessel_type_id,
    f.fiskeridir_length_group_id,
    f.fiskeridir_nation_group_id,
    f.norwegian_municipality_id AS fiskeridir_norwegian_municipality_id,
    f.norwegian_county_id AS fiskeridir_norwegian_county_id,
    f.nation_id AS "fiskeridir_nation_id!",
    f.gross_tonnage_1969 AS fiskeridir_gross_tonnage_1969,
    f.gross_tonnage_other AS fiskeridir_gross_tonnage_other,
    f.call_sign AS fiskeridir_call_sign,
    f."name" AS fiskeridir_name,
    f.registration_id AS fiskeridir_registration_id,
    f."length" AS fiskeridir_length,
    f."width" AS fiskeridir_width,
    f."owner" AS fiskeridir_owner,
    f.engine_building_year AS fiskeridir_engine_building_year,
    f.engine_power AS fiskeridir_engine_power,
    f.building_year AS fiskeridir_building_year,
    f.rebuilding_year AS fiskeridir_rebuilding_year,
    a.mmsi AS "ais_mmsi?",
    a.imo_number AS ais_imo_number,
    a.call_sign AS ais_call_sign,
    a.name AS ais_name,
    a.ship_length AS ais_ship_length,
    a.ship_width AS ais_ship_width,
    a.eta AS ais_eta,
    a.destination AS ais_destination
FROM
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
WHERE
    f.fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
