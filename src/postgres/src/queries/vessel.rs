use crate::chunk::Chunks;
use crate::{
    PostgresAdapter,
    error::Result,
    models::{
        FiskeridirAisVesselCombination, NewMunicipality, NewOrg, NewOrgVessel, NewRegisterVessel,
    },
};
use fiskeridir_rs::{CallSign, GearGroup, OrgId, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::Draught;
use kyogre_core::{
    ActiveVesselConflict, EngineType, FiskeridirVesselId, HasTrack, Mmsi, TripAssemblerId, Vessel,
    VesselSource,
};
use std::collections::{HashMap, HashSet};

impl PostgresAdapter {
    pub(crate) async fn queue_vessel_trip_reset(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) fn vessels_with_trips_impl(
        &self,
        num_trips: u32,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination>> + '_ {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.preferred_trip_assembler AS "preferred_trip_assembler!: TripAssemblerId",
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    f.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    v.call_sign AS "fiskeridir_call_sign: CallSign",
    f."name" AS fiskeridir_name,
    f.registration_id AS fiskeridir_registration_id,
    f."length" AS fiskeridir_length,
    f."width" AS fiskeridir_width,
    f.owners::TEXT AS "fiskeridir_owners!",
    f.engine_building_year_final AS fiskeridir_engine_building_year,
    f.engine_power_final AS fiskeridir_engine_power,
    f.building_year AS fiskeridir_building_year,
    f.auxiliary_engine_power AS fiskeridir_auxiliary_engine_power,
    f.auxiliary_engine_building_year AS fiskeridir_auxiliary_engine_building_year,
    f.engine_version AS fiskeridir_engine_version,
    f.boiler_engine_power AS fiskeridir_boiler_engine_power,
    f.boiler_engine_building_year AS fiskeridir_boiler_engine_building_year,
    f.engine_type_manual AS "fiskeridir_engine_type: EngineType",
    f.engine_rpm_manual AS "fiskeridir_engine_rpm",
    f.service_speed AS fiskeridir_service_speed,
    f.degree_of_electrification AS fiskeridir_degree_of_electrification,
    f.gear_group_ids AS "gear_group_ids!: Vec<GearGroup>",
    f.species_group_ids AS "species_group_ids!: Vec<SpeciesGroup>",
    a.mmsi AS "ais_mmsi?: Mmsi",
    a.call_sign AS "ais_call_sign: CallSign",
    a.name AS ais_name,
    a.draught AS "ais_draught: Draught",
    a.ship_length AS ais_length,
    a.ship_width AS ais_width,
    c.departure_timestamp AS "current_trip_departure_timestamp?",
    c.target_species_fiskeridir_id AS "current_trip_target_species_fiskeridir_id?",
    v.is_active AS "is_active!"
FROM
    all_vessels AS v
    INNER JOIN fiskeridir_vessels AS f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON v.mmsi = a.mmsi
    LEFT JOIN current_trips AS c ON v.fiskeridir_vessel_id = c.fiskeridir_vessel_id
WHERE
    (
        SELECT
            COUNT(*)
        FROM
            (
                SELECT
                    1
                FROM
                    trips_detailed t
                WHERE
                    t.fiskeridir_vessel_id = v.fiskeridir_vessel_id
                    AND t.has_track > $1
                LIMIT
                    $2
            )
    ) >= $2
            "#,
            HasTrack::NoTrack as i32,
            num_trips as i32
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn update_vessel_impl(
        &self,
        call_sign: &CallSign,
        update: &kyogre_core::UpdateVessel,
    ) -> Result<Option<Vessel>> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query!(
            r#"
UPDATE fiskeridir_vessels
SET
    engine_power_manual = $1,
    engine_building_year_manual = $2,
    auxiliary_engine_power = $3,
    auxiliary_engine_building_year = $4,
    boiler_engine_power = $5,
    boiler_engine_building_year = $6,
    service_speed = $7,
    degree_of_electrification = $8,
    engine_type_manual = $9,
    engine_rpm_manual = $10,
    engine_version = engine_version + 1
WHERE
    call_sign = $11
RETURNING
    fiskeridir_vessel_id AS "fiskeridir_vessel_id: FiskeridirVesselId"
            "#,
            update.engine_power.map(|e| e as i32),
            update.engine_building_year.map(|e| e as i32),
            update.auxiliary_engine_power.map(|e| e as i32),
            update.auxiliary_engine_building_year.map(|e| e as i32),
            update.boiler_engine_power.map(|e| e as i32),
            update.boiler_engine_building_year.map(|e| e as i32),
            update.service_speed,
            update.degree_of_electrification,
            update.engine_type.map(|v| v as i32),
            update.engine_rpm.map(|v| v as i32),
            call_sign
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(res) = res {
            let out = self
                .fiskeridir_ais_vessel_combinations_impl(Some(res.fiskeridir_vessel_id), &mut *tx)
                .try_collect::<Vec<FiskeridirAisVesselCombination>>()
                .await?
                .pop()
                .map(Vessel::try_from)
                .transpose()?;

            self.reset_bencmarks(res.fiskeridir_vessel_id, &mut *tx)
                .await?;

            self.queue_vessel_trip_reset(res.fiskeridir_vessel_id, &mut tx)
                .await?;
            self.reset_fuel_estimation(res.fiskeridir_vessel_id, &mut tx)
                .await?;
            tx.commit().await?;

            Ok(out)
        } else {
            tx.rollback().await?;
            Ok(None)
        }
    }

    pub(crate) fn active_vessel_conflicts_impl(
        &self,
    ) -> impl Stream<Item = Result<ActiveVesselConflict>> + '_ {
        sqlx::query_as!(
            ActiveVesselConflict,
            r#"
SELECT
    call_sign AS "call_sign!: CallSign",
    mmsis AS "mmsis!: Vec<Option<Mmsi>>",
    fiskeridir_vessel_ids AS "vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    fiskeridir_vessel_source_ids AS "sources!: Vec<Option<VesselSource>>"
FROM
    vessel_conflicts
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn refresh_vessel_mappings(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM all_vessels
WHERE
    is_manual = FALSE;
            "#,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM vessel_conflicts
            "#,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    all_vessels (fiskeridir_vessel_id, mmsi, call_sign, is_active)
SELECT
    (ARRAY_AGG(f.fiskeridir_vessel_id)) [1],
    (ARRAY_AGG(a.mmsi)) [1],
    f.call_sign,
    TRUE
FROM
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
WHERE
    f.call_sign IS NOT NULL
    AND NOT (f.call_sign = ANY ($1::VARCHAR[]))
GROUP BY
    f.call_sign
HAVING
    COUNT(*) = 1
ON CONFLICT DO NOTHING;
            "#,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    all_vessels (fiskeridir_vessel_id, is_active)
SELECT
    f.fiskeridir_vessel_id,
    TRUE
FROM
    fiskeridir_vessels AS f
WHERE
    f.call_sign IS NULL
    OR f.call_sign = ANY ($1::VARCHAR[])
ON CONFLICT DO NOTHING
            "#,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        let conflicts = sqlx::query!(
            r#"
SELECT
    ARRAY_AGG(DISTINCT f.fiskeridir_vessel_id) AS "fiskeridir_vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    f.call_sign AS "call_sign!: CallSign",
    COALESCE(ARRAY_AGG(DISTINCT a.mmsi), '{}') AS "mmsis!: Vec<Option<Mmsi>>",
    COALESCE(ARRAY_AGG(DISTINCT a.name), '{}') AS "ais_vessel_names!: Vec<Option<String>>",
    COALESCE(ARRAY_AGG(DISTINCT f.name), '{}') AS "fiskeridir_vessel_names!: Vec<Option<String>>",
    COALESCE(
        ARRAY_AGG(DISTINCT f.fiskeridir_vessel_source_id),
        '{}'
    ) AS "fiskeridir_vessel_source_ids!: Vec<Option<VesselSource>>"
FROM
    fiskeridir_vessels AS f
    LEFT JOIN all_vessels w ON f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
WHERE
    (
        w.is_manual = FALSE
        OR w.is_manual IS NULL
    )
    AND f.call_sign IS NOT NULL
    AND NOT (f.call_sign = ANY ($1::VARCHAR[]))
GROUP BY
    f.call_sign
HAVING
    COUNT(*) > 1
            "#,
            &self.ignored_conflict_call_signs
        )
        .fetch_all(&mut **tx)
        .await?;

        for c in &conflicts {
            sqlx::query!(
                r#"
INSERT INTO
    vessel_conflicts (
        call_sign,
        mmsis,
        fiskeridir_vessel_ids,
        ais_vessel_names,
        fiskeridir_vessel_names,
        fiskeridir_vessel_source_ids
    )
VALUES
    ($1, $2, $3, $4, $5, $6)
                "#,
                c.call_sign.as_ref(),
                &c.mmsis as &[Option<Mmsi>],
                &c.fiskeridir_vessel_ids as &[Option<FiskeridirVesselId>],
                &c.ais_vessel_names as &[Option<String>],
                &c.fiskeridir_vessel_names as &[Option<String>],
                &c.fiskeridir_vessel_source_ids as &[Option<VesselSource>],
            )
            .execute(&mut **tx)
            .await?;
        }

        sqlx::query!(
            r#"
UPDATE all_vessels v
SET
    length = q.length,
    ship_type = q.ship_type
FROM
    (
        SELECT
            v.fiskeridir_vessel_id,
            a.ship_type,
            COALESCE(a.ship_length, f.length) AS length
        FROM
            all_vessels v
            INNER JOIN fiskeridir_vessels f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
            LEFT JOIN ais_vessels a ON v.mmsi = a.mmsi
    ) q
WHERE
    v.fiskeridir_vessel_id = q.fiskeridir_vessel_id;
            "#
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn set_landing_vessels_call_signs(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels
SET
    call_sign = q.call_sign,
    overriden_call_sign = fiskeridir_vessels.call_sign,
    call_sign_override = TRUE
FROM
    (
        SELECT
            qi.fiskeridir_vessel_id,
            (
                ARRAY_AGG(
                    vessel_call_sign
                    ORDER BY
                        qi.landing_ids DESC
                )
            ) [1] AS call_sign
        FROM
            (
                SELECT
                    fiskeridir_vessel_id,
                    vessel_call_sign,
                    COUNT(landing_id) AS landing_ids
                FROM
                    landings
                WHERE
                    vessel_call_sign IS NOT NULL
                    AND NOT (vessel_call_sign = ANY ($2::VARCHAR[]))
                GROUP BY
                    fiskeridir_vessel_id,
                    vessel_call_sign
            ) qi
        GROUP BY
            qi.fiskeridir_vessel_id
        HAVING
            COUNT(DISTINCT vessel_call_sign) > 1
    ) q
WHERE
    fiskeridir_vessels.fiskeridir_vessel_id = q.fiskeridir_vessel_id
    AND fiskeridir_vessel_source_id = $1;
            "#,
            VesselSource::Landings as i32,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_register_vessels_full(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<()> {
        let municipalitis: HashMap<i32, NewMunicipality<'_>> = vessels
            .iter()
            .map(|v| {
                (
                    v.municipality_code,
                    NewMunicipality {
                        id: v.municipality_code,
                        name: None,
                    },
                )
            })
            .collect();

        let org_vessels: Vec<NewOrgVessel> = vessels
            .iter()
            .flat_map(|v| {
                v.owners.iter().filter_map(|o| {
                    o.id.map(|org_id| NewOrgVessel {
                        org_id,
                        fiskeridir_vessel_id: v.id,
                    })
                })
            })
            .collect();

        let orgs: HashMap<OrgId, NewOrg<'_>> = vessels
            .iter()
            .flat_map(|v| {
                v.owners.iter().filter_map(|o| {
                    o.id.map(|org_id| {
                        (
                            org_id,
                            NewOrg {
                                org_id,
                                entity_type: o.entity_type,
                                city: o.city.as_deref(),
                                name: &o.name,
                                postal_code: o.postal_code,
                            },
                        )
                    })
                })
            })
            .collect();

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(municipalitis.into_values(), &mut *tx)
            .await?;
        self.unnest_insert(orgs.into_values(), &mut *tx).await?;
        self.unnest_insert_try_from::<_, _, NewRegisterVessel>(vessels, &mut *tx)
            .await?;
        self.unnest_insert(org_vessels, &mut *tx).await?;

        self.set_landing_vessels_call_signs(&mut tx).await?;
        self.refresh_vessel_mappings(&mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations(
        &self,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination>> + '_ {
        self.fiskeridir_ais_vessel_combinations_impl(None, &self.pool)
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations_impl<'a>(
        &'a self,
        vessel_id: Option<FiskeridirVesselId>,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres> + 'a,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination>> + 'a {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.preferred_trip_assembler AS "preferred_trip_assembler!: TripAssemblerId",
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    f.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    v.call_sign AS "fiskeridir_call_sign: CallSign",
    f."name" AS fiskeridir_name,
    f.registration_id AS fiskeridir_registration_id,
    f."length" AS fiskeridir_length,
    f."width" AS fiskeridir_width,
    f.owners::TEXT AS "fiskeridir_owners!",
    f.engine_building_year_final AS fiskeridir_engine_building_year,
    f.engine_power_final AS fiskeridir_engine_power,
    f.building_year AS fiskeridir_building_year,
    f.auxiliary_engine_power AS fiskeridir_auxiliary_engine_power,
    f.auxiliary_engine_building_year AS fiskeridir_auxiliary_engine_building_year,
    f.engine_version AS fiskeridir_engine_version,
    f.boiler_engine_power AS fiskeridir_boiler_engine_power,
    f.boiler_engine_building_year AS fiskeridir_boiler_engine_building_year,
    f.engine_type_manual AS "fiskeridir_engine_type: EngineType",
    f.engine_rpm_manual AS "fiskeridir_engine_rpm",
    f.service_speed AS fiskeridir_service_speed,
    f.degree_of_electrification AS fiskeridir_degree_of_electrification,
    f.gear_group_ids AS "gear_group_ids!: Vec<GearGroup>",
    f.species_group_ids AS "species_group_ids!: Vec<SpeciesGroup>",
    a.mmsi AS "ais_mmsi?: Mmsi",
    a.call_sign AS "ais_call_sign: CallSign",
    a.name AS ais_name,
    a.draught AS "ais_draught: Draught",
    a.ship_length AS ais_length,
    a.ship_width AS ais_width,
    c.departure_timestamp AS "current_trip_departure_timestamp?",
    c.target_species_fiskeridir_id AS current_trip_target_species_fiskeridir_id,
    v.is_active
FROM
    all_vessels AS v
    INNER JOIN fiskeridir_vessels AS f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON v.mmsi = a.mmsi
    LEFT JOIN current_trips AS c ON v.fiskeridir_vessel_id = c.fiskeridir_vessel_id
WHERE
    (
        $1::BIGINT IS NULL
        OR v.fiskeridir_vessel_id = $1
    )
            "#,
            vessel_id as Option<FiskeridirVesselId>,
        )
        .fetch(executor)
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_vessel_gear_and_species_groups<'a>(
        &'a self,
        vessel_ids: HashSet<FiskeridirVesselId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let mut iter = vessel_ids.chunks(100);
        while let Some(chunk) = iter.next() {
            sqlx::query!(
                r#"
UPDATE fiskeridir_vessels v
SET
    gear_group_ids = q.gear_group_ids,
    species_group_ids = q.species_group_ids
FROM
    (
        SELECT
            fiskeridir_vessel_id,
            ARRAY_AGG(DISTINCT gear_group_id) AS gear_group_ids,
            ARRAY_AGG(DISTINCT species_group_id) AS species_group_ids
        FROM
            landings l
            LEFT JOIN landing_entries e ON l.landing_id = e.landing_id
        WHERE
            fiskeridir_vessel_id = ANY ($1)
        GROUP BY
            fiskeridir_vessel_id
    ) q
WHERE
    v.fiskeridir_vessel_id = q.fiskeridir_vessel_id
            "#,
                &chunk as &[FiskeridirVesselId],
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn update_preferred_trip_assemblers_impl(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let vessel_ids = sqlx::query!(
            r#"
UPDATE fiskeridir_vessels f
SET
    preferred_trip_assembler = $1
FROM
    (
        SELECT
            v.fiskeridir_vessel_id
        FROM
            fiskeridir_vessels v
            INNER JOIN landings l ON v.fiskeridir_vessel_id = l.fiskeridir_vessel_id
            INNER JOIN ers_arrivals e ON v.fiskeridir_vessel_id = e.fiskeridir_vessel_id
        GROUP BY
            v.fiskeridir_vessel_id
        HAVING
            MAX(l.landing_timestamp) - MAX(e.arrival_timestamp) > INTERVAL '1 year'
    ) q
WHERE
    f.fiskeridir_vessel_id = q.fiskeridir_vessel_id
RETURNING
    f.fiskeridir_vessel_id AS "id!: FiskeridirVesselId"
            "#,
            TripAssemblerId::Landings as i32,
        )
        .fetch(&mut *tx)
        .map_ok(|r| r.id)
        .try_collect::<Vec<_>>()
        .await?;

        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels f
SET
    preferred_trip_assembler = $1
FROM
    (
        SELECT DISTINCT
            v.fiskeridir_vessel_id
        FROM
            fiskeridir_vessels v
            INNER JOIN ers_departures e ON v.fiskeridir_vessel_id = e.fiskeridir_vessel_id
    ) q
WHERE
    f.fiskeridir_vessel_id = q.fiskeridir_vessel_id
    AND NOT (f.fiskeridir_vessel_id = ANY ($2::BIGINT[]))
            "#,
            TripAssemblerId::Ers as i32,
            &vessel_ids as &[FiskeridirVesselId],
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn vessel_max_cargo_weight_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<f64> {
        let res = sqlx::query!(
            r#"
SELECT
    COALESCE(MAX(landing_total_living_weight), 0) AS "weight!"
FROM
    trips_detailed
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(res.map(|v| v.weight).unwrap_or(0.))
    }
}
