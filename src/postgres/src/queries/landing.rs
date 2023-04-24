use sqlx::Row;
use std::collections::HashSet;

use crate::{
    error::{ErrorWrapper, PostgresError},
    landing_set::LandingSet,
    models::NewLanding,
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use fiskeridir_rs::LandingId;
use kyogre_core::FiskeridirVesselId;

impl PostgresAdapter {
    pub(crate) async fn add_landing_set(
        &self,
        set: LandingSet,
        data_year: u32,
    ) -> Result<(), PostgresError> {
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
        self.add_landings(prepared_set.landings, data_year, &mut tx)
            .await?;
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
        data_year: u32,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = landings.len();

        let mut landing_id = Vec::with_capacity(len);
        let mut document_id = Vec::with_capacity(len);
        let mut fiskeridir_vessel_id = Vec::with_capacity(len);
        let mut fiskeridir_vessel_type_id = Vec::with_capacity(len);
        let mut vessel_call_sign = Vec::with_capacity(len);
        let mut vessel_registration_id = Vec::with_capacity(len);
        let mut vessel_length_group_id = Vec::with_capacity(len);
        let mut vessel_nation_group_id = Vec::with_capacity(len);
        let mut vessel_nation_id = Vec::with_capacity(len);
        let mut vessel_norwegian_municipality_id = Vec::with_capacity(len);
        let mut vessel_norwegian_county_id = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_1969 = Vec::with_capacity(len);
        let mut vessel_gross_tonnage_other = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut vessel_length = Vec::with_capacity(len);
        let mut vessel_engine_building_year = Vec::with_capacity(len);
        let mut vessel_engine_power = Vec::with_capacity(len);
        let mut vessel_building_year = Vec::with_capacity(len);
        let mut vessel_rebuilding_year = Vec::with_capacity(len);
        let mut gear_id = Vec::with_capacity(len);
        let mut gear_group_id = Vec::with_capacity(len);
        let mut gear_main_group_id = Vec::with_capacity(len);
        let mut document_type_id = Vec::with_capacity(len);
        let mut sales_team_id = Vec::with_capacity(len);
        let mut sales_team_tax = Vec::with_capacity(len);
        let mut delivery_point_id = Vec::with_capacity(len);
        let mut document_sale_date = Vec::with_capacity(len);
        let mut document_version_date = Vec::with_capacity(len);
        let mut landing_timestamp = Vec::with_capacity(len);
        let mut landing_time = Vec::with_capacity(len);
        let mut landing_month_id = Vec::with_capacity(len);
        let mut version = Vec::with_capacity(len);
        let mut last_catch_date = Vec::with_capacity(len);
        let mut num_crew_members = Vec::with_capacity(len);
        let mut fisher_org_id = Vec::with_capacity(len);
        let mut fisher_nation_id = Vec::with_capacity(len);
        let mut fisher_municipality_id = Vec::with_capacity(len);
        let mut catch_field = Vec::with_capacity(len);
        let mut catch_area_id = Vec::with_capacity(len);
        let mut catch_main_area_id = Vec::with_capacity(len);
        let mut catch_main_area_fao_id = Vec::with_capacity(len);
        let mut fishing_region_id = Vec::with_capacity(len);
        let mut delivery_point_municipality_id = Vec::with_capacity(len);
        let mut landing_norwegian_county_id = Vec::with_capacity(len);
        let mut landing_nation_id = Vec::with_capacity(len);
        let mut north_south_62_degrees_id = Vec::with_capacity(len);
        let mut within_12_mile_border = Vec::with_capacity(len);
        let mut physical_trip_diary_number = Vec::with_capacity(len);
        let mut physical_trip_diary_trip_number = Vec::with_capacity(len);
        let mut economic_zone_id = Vec::with_capacity(len);
        let mut part_delivery = Vec::with_capacity(len);
        let mut part_delivery_next_delivery_point_id = Vec::with_capacity(len);
        let mut part_delivery_previous_delivery_point_id = Vec::with_capacity(len);
        let mut data_update_timestamp = Vec::with_capacity(len);
        let mut catch_year = Vec::with_capacity(len);
        let mut production_facility = Vec::with_capacity(len);
        let mut production_facility_municipality_id = Vec::with_capacity(len);
        let mut product_quality_id = Vec::with_capacity(len);
        let mut quota_type_id = Vec::with_capacity(len);
        let mut quota_vessel_registration_id = Vec::with_capacity(len);
        let mut buyer_org_id = Vec::with_capacity(len);
        let mut buyer_nation_id = Vec::with_capacity(len);
        let mut receiving_vessel_registration_id = Vec::with_capacity(len);
        let mut receiving_vessel_mmsi_or_call_sign = Vec::with_capacity(len);
        let mut receiving_vessel_type = Vec::with_capacity(len);
        let mut receiving_vessel_nation_id = Vec::with_capacity(len);
        let mut receiving_vessel_nation = Vec::with_capacity(len);
        let mut data_year_vec = Vec::with_capacity(len);

        for l in landings {
            landing_id.push(l.landing_id);
            document_id.push(l.document_id);
            fiskeridir_vessel_id.push(l.fiskeridir_vessel_id);
            fiskeridir_vessel_type_id.push(l.fiskeridir_vessel_type_id);
            vessel_call_sign.push(l.vessel_call_sign);
            vessel_registration_id.push(l.vessel_registration_id);
            vessel_length_group_id.push(l.vessel_length_group_id);
            vessel_nation_group_id.push(l.vessel_nation_group_id);
            vessel_nation_id.push(l.vessel_nation_id);
            vessel_norwegian_municipality_id.push(l.vessel_norwegian_municipality_id);
            vessel_norwegian_county_id.push(l.vessel_norwegian_county_id);
            vessel_gross_tonnage_1969.push(l.vessel_gross_tonnage_1969);
            vessel_gross_tonnage_other.push(l.vessel_gross_tonnage_other);
            vessel_name.push(l.vessel_name);
            vessel_length.push(l.vessel_length);
            vessel_engine_building_year.push(l.vessel_engine_building_year);
            vessel_engine_power.push(l.vessel_engine_power);
            vessel_building_year.push(l.vessel_building_year);
            vessel_rebuilding_year.push(l.vessel_rebuilding_year);
            gear_id.push(l.gear_id);
            gear_group_id.push(l.gear_group_id);
            gear_main_group_id.push(l.gear_main_group_id);
            document_type_id.push(l.document_type_id);
            sales_team_id.push(l.sales_team_id);
            sales_team_tax.push(l.sales_team_tax);
            delivery_point_id.push(l.delivery_point_id);
            document_sale_date.push(l.document_sale_date);
            document_version_date.push(l.document_version_date);
            landing_timestamp.push(l.landing_timestamp);
            landing_time.push(l.landing_time);
            landing_month_id.push(l.landing_month_id);
            version.push(l.version);
            last_catch_date.push(l.last_catch_date);
            num_crew_members.push(l.num_crew_members);
            fisher_org_id.push(l.fisher_org_id);
            fisher_nation_id.push(l.fisher_nation_id);
            fisher_municipality_id.push(l.fisher_municipality_id);
            catch_field.push(l.catch_field);
            catch_area_id.push(l.catch_area_id);
            catch_main_area_id.push(l.catch_main_area_id);
            catch_main_area_fao_id.push(l.catch_main_area_fao_id);
            fishing_region_id.push(l.area_grouping_id);
            delivery_point_municipality_id.push(l.delivery_point_municipality_id);
            landing_norwegian_county_id.push(l.landing_norwegian_county_id);
            landing_nation_id.push(l.landing_nation_id);
            north_south_62_degrees_id.push(l.north_south_62_degrees_id);
            within_12_mile_border.push(l.within_12_mile_border);
            physical_trip_diary_number.push(l.fishing_diary_number);
            physical_trip_diary_trip_number.push(l.fishing_diary_trip_number);
            economic_zone_id.push(l.economic_zone_id);
            part_delivery.push(l.partial_landing);
            part_delivery_next_delivery_point_id.push(l.partial_landing_next_delivery_point_id);
            part_delivery_previous_delivery_point_id
                .push(l.partial_landing_previous_delivery_point_id);
            data_update_timestamp.push(l.data_update_timestamp);
            catch_year.push(l.catch_year);
            production_facility.push(l.production_facility);
            production_facility_municipality_id.push(l.production_facility_municipality_id);
            product_quality_id.push(l.product_quality_id);
            quota_type_id.push(l.quota_type_id);
            quota_vessel_registration_id.push(l.quota_vessel_registration_id);
            buyer_org_id.push(l.buyer_org_id);
            buyer_nation_id.push(l.buyer_nation_id);
            receiving_vessel_registration_id.push(l.receiving_vessel_registration_id);
            receiving_vessel_mmsi_or_call_sign.push(l.receiving_vessel_mmsi_or_call_sign);
            receiving_vessel_type.push(l.receiving_vessel_type);
            receiving_vessel_nation_id.push(l.receiving_vessel_nation_id);
            receiving_vessel_nation.push(l.receiving_vessel_nation);
            data_year_vec.push(data_year as i32);
        }

        sqlx::query!(
            r#"
INSERT INTO
    landings (
        landing_id,
        document_id,
        fiskeridir_vessel_id,
        fiskeridir_vessel_type_id,
        vessel_call_sign,
        vessel_registration_id,
        vessel_length_group_id,
        vessel_nation_group_id,
        vessel_nation_id,
        vessel_norwegian_municipality_id,
        vessel_norwegian_county_id,
        vessel_gross_tonnage_1969,
        vessel_gross_tonnage_other,
        vessel_name,
        vessel_length,
        vessel_engine_building_year,
        vessel_engine_power,
        vessel_building_year,
        vessel_rebuilding_year,
        gear_id,
        gear_group_id,
        gear_main_group_id,
        document_type_id,
        sales_team_id,
        sales_team_tax,
        delivery_point_id,
        document_sale_date,
        document_version_date,
        landing_timestamp,
        landing_time,
        landing_month_id,
        "version",
        last_catch_date,
        num_crew_members,
        fisher_org_id,
        fisher_nation_id,
        fisher_municipality_id,
        catch_field,
        catch_area_id,
        catch_main_area_id,
        catch_main_area_fao_id,
        area_grouping_id,
        delivery_point_municipality_id,
        landing_norwegian_county_id,
        landing_nation_id,
        north_south_62_degrees_id,
        within_12_mile_border,
        fishing_diary_number,
        fishing_diary_trip_number,
        economic_zone_id,
        partial_landing,
        partial_landing_next_delivery_point_id,
        partial_landing_previous_delivery_point_id,
        data_update_timestamp,
        catch_year,
        production_facility,
        production_facility_municipality_id,
        product_quality_id,
        quota_type_id,
        quota_vessel_registration_id,
        buyer_org_id,
        buyer_nation_id,
        receiving_vessel_registration_id,
        receiving_vessel_mmsi_or_call_sign,
        receiving_vessel_type,
        receiving_vessel_nation_id,
        receiving_vessel_nation,
        data_year
    )
SELECT
    *
FROM
    UNNEST(
        $1::VARCHAR[],
        $2::BIGINT[],
        $3::BIGINT[],
        $4::INT[],
        $5::VARCHAR[],
        $6::VARCHAR[],
        $7::INT[],
        $8::VARCHAR[],
        $9::VARCHAR[],
        $10::INT[],
        $11::INT[],
        $12::INT[],
        $13::INT[],
        $14::VARCHAR[],
        $15::DECIMAL[],
        $16::INT[],
        $17::INT[],
        $18::INT[],
        $19::INT[],
        $20::INT[],
        $21::INT[],
        $22::INT[],
        $23::INT[],
        $24::INT[],
        $25::DECIMAL[],
        $26::VARCHAR[],
        $27::date[],
        $28::timestamptz[],
        $29::timestamptz[],
        $30::TIME[],
        $31::INT[],
        $32::INT[],
        $33::date[],
        $34::INT[],
        $35::INT[],
        $36::VARCHAR[],
        $37::INT[],
        $38::VARCHAR[],
        $39::INT[],
        $40::INT[],
        $41::INT[],
        $42::VARCHAR[],
        $43::INT[],
        $44::INT[],
        $45::VARCHAR[],
        $46::VARCHAR[],
        $47::INT[],
        $48::INT[],
        $49::INT[],
        $50::VARCHAR[],
        $51::BOOLEAN[],
        $52::VARCHAR[],
        $53::VARCHAR[],
        $54::timestamptz[],
        $55::INT[],
        $56::VARCHAR[],
        $57::INT[],
        $58::INT[],
        $59::INT[],
        $60::VARCHAR[],
        $61::INT[],
        $62::VARCHAR[],
        $63::VARCHAR[],
        $64::VARCHAR[],
        $65::INT[],
        $66::VARCHAR[],
        $67::VARCHAR[],
        $68::INT[]
    )
ON CONFLICT (landing_id, "version") DO
UPDATE
SET
    data_year = excluded.data_year
            "#,
            landing_id.as_slice(),
            document_id.as_slice(),
            fiskeridir_vessel_id.as_slice() as _,
            fiskeridir_vessel_type_id.as_slice() as _,
            vessel_call_sign.as_slice() as _,
            vessel_registration_id.as_slice() as _,
            vessel_length_group_id.as_slice() as _,
            vessel_nation_group_id.as_slice() as _,
            vessel_nation_id.as_slice(),
            vessel_norwegian_municipality_id.as_slice() as _,
            vessel_norwegian_county_id.as_slice() as _,
            vessel_gross_tonnage_1969.as_slice() as _,
            vessel_gross_tonnage_other.as_slice() as _,
            vessel_name.as_slice() as _,
            vessel_length.as_slice() as _,
            vessel_engine_building_year.as_slice() as _,
            vessel_engine_power.as_slice() as _,
            vessel_building_year.as_slice() as _,
            vessel_rebuilding_year.as_slice() as _,
            gear_id.as_slice(),
            gear_group_id.as_slice() as _,
            gear_main_group_id.as_slice(),
            document_type_id.as_slice(),
            sales_team_id.as_slice(),
            sales_team_tax.as_slice() as _,
            delivery_point_id.as_slice() as _,
            document_sale_date.as_slice() as _,
            document_version_date.as_slice(),
            landing_timestamp.as_slice(),
            landing_time.as_slice(),
            landing_month_id.as_slice(),
            version.as_slice(),
            last_catch_date.as_slice(),
            num_crew_members.as_slice() as _,
            fisher_org_id.as_slice() as _,
            fisher_nation_id.as_slice() as _,
            fisher_municipality_id.as_slice() as _,
            catch_field.as_slice(),
            catch_area_id.as_slice() as _,
            catch_main_area_id.as_slice() as _,
            catch_main_area_fao_id.as_slice() as _,
            fishing_region_id.as_slice() as _,
            delivery_point_municipality_id.as_slice() as _,
            landing_norwegian_county_id.as_slice() as _,
            landing_nation_id.as_slice() as _,
            north_south_62_degrees_id.as_slice(),
            within_12_mile_border.as_slice(),
            physical_trip_diary_number.as_slice() as _,
            physical_trip_diary_trip_number.as_slice() as _,
            economic_zone_id.as_slice() as _,
            part_delivery.as_slice(),
            part_delivery_next_delivery_point_id.as_slice() as _,
            part_delivery_previous_delivery_point_id.as_slice() as _,
            data_update_timestamp.as_slice(),
            catch_year.as_slice(),
            production_facility.as_slice() as _,
            production_facility_municipality_id.as_slice() as _,
            product_quality_id.as_slice(),
            quota_type_id.as_slice() as _,
            quota_vessel_registration_id.as_slice() as _,
            buyer_org_id.as_slice() as _,
            buyer_nation_id.as_slice() as _,
            receiving_vessel_registration_id.as_slice() as _,
            receiving_vessel_mmsi_or_call_sign.as_slice() as _,
            receiving_vessel_type.as_slice() as _,
            receiving_vessel_nation_id.as_slice() as _,
            receiving_vessel_nation.as_slice() as _,
            data_year_vec.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn landing_dates_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, PostgresError> {
        struct Intermediate {
            landing_timestamp: DateTime<Utc>,
        }
        Ok(sqlx::query_as!(
            Intermediate,
            r#"
SELECT
    landing_timestamp
FROM
    landings
WHERE
    fiskeridir_vessel_id = $1
    AND landing_timestamp >= $2
            "#,
            vessel_id.0,
            start,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|v| v.landing_timestamp)
        .collect())
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
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)
    }
}
