use crate::{models::Haul, PostgresAdapter};
use chrono::{DateTime, Datelike, Duration, Utc};
use fiskeridir_rs::{
    CallSign, ErsDca, ErsDep, ErsPor, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup,
    Vms,
};
use futures::TryStreamExt;
use kyogre_core::*;
use rand::random;

/// Wrapper with additional methods inteded for testing purposes.
#[derive(Debug, Clone)]
pub struct TestDb {
    pub db: PostgresAdapter,
}

impl TestDb {
    pub async fn drop_db(&self, db_name: &str) {
        {
            sqlx::query(&format!(
                "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE);"
            ))
            .execute(&self.db.pool)
            .await
            .unwrap();
        }
        self.db.pool.close().await;
    }

    pub async fn all_ais_positions(&self) -> Vec<AisPosition> {
        self.ais_positions(None, None).await
    }

    pub async fn haul_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
        message_id: u64,
    ) -> kyogre_core::Haul {
        let haul = sqlx::query_as!(
            Haul,
            r#"
SELECT
    h.haul_id AS "haul_id!: HaulId",
    h.ers_activity_id,
    h.duration,
    h.haul_distance,
    h.catch_location_start,
    h.catch_locations,
    h.ocean_depth_end,
    h.ocean_depth_start,
    h.quota_type_id,
    h.start_latitude,
    h.start_longitude,
    h.start_timestamp,
    h.stop_timestamp,
    h.stop_latitude,
    h.stop_longitude,
    h.total_living_weight,
    h.gear_id AS "gear_id!: Gear",
    h.gear_group_id AS "gear_group_id!: GearGroup",
    h.fiskeridir_vessel_id AS "fiskeridir_vessel_id?: FiskeridirVesselId",
    h.vessel_call_sign,
    h.vessel_call_sign_ers,
    h.vessel_length,
    h.vessel_length_group AS "vessel_length_group!: VesselLengthGroup",
    h.vessel_name,
    h.vessel_name_ers,
    h.wind_speed_10m,
    h.wind_direction_10m,
    h.air_temperature_2m,
    h.relative_humidity_2m,
    h.air_pressure_at_sea_level,
    h.precipitation_amount,
    h.cloud_area_fraction,
    h.water_speed,
    h.water_direction,
    h.salinity,
    h.water_temperature,
    h.ocean_climate_depth,
    h.sea_floor_depth,
    h.catches::TEXT AS "catches!",
    h.whale_catches::TEXT AS "whale_catches!",
    h.cache_version
FROM
    hauls h
WHERE
    h.fiskeridir_vessel_id = $1
    AND h.message_id = $2
            "#,
            vessel_id.into_inner(),
            message_id as i64,
        )
        .fetch_one(&self.db.pool)
        .await
        .unwrap();

        kyogre_core::Haul::try_from(haul).unwrap()
    }

    pub async fn hauls_matrix(
        &self,
        active_filter: ActiveHaulsFilter,
        x_feature: HaulMatrixXFeature,
    ) -> Vec<HaulMatrixQueryOutput> {
        let y_feature = if x_feature == active_filter {
            HaulMatrixYFeature::CatchLocation
        } else {
            HaulMatrixYFeature::from(active_filter)
        };

        sqlx::query_as!(
            HaulMatrixQueryOutput,
            r#"
SELECT
    CASE
        WHEN $1 = 0 THEN matrix_month_bucket
        WHEN $1 = 1 THEN gear_group_id
        WHEN $1 = 2 THEN species_group_id
        WHEN $1 = 3 THEN vessel_length_group
    END AS "x_index!",
    CASE
        WHEN $2 = 0 THEN matrix_month_bucket
        WHEN $2 = 1 THEN gear_group_id
        WHEN $2 = 2 THEN species_group_id
        WHEN $2 = 3 THEN vessel_length_group
        WHEN $2 = 4 THEN catch_location_matrix_index
    END AS "y_index!",
    COALESCE(SUM(living_weight), 0)::BIGINT AS "sum_living!"
FROM
    hauls_matrix
GROUP BY
    1,
    2
            "#,
            x_feature as i32,
            y_feature as i32,
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    pub async fn ais_positions(
        &self,
        mmsi: Option<Mmsi>,
        range: Option<&DateRange>,
    ) -> Vec<AisPosition> {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi AS "mmsi!: Mmsi",
    "timestamp" AS msgtime,
    course_over_ground,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    ais_positions
WHERE
    (
        $1::INT IS NULL
        OR mmsi = $1
    )
    AND (
        (
            $2::timestamptz IS NULL
            AND $3::timestamptz IS NULL
        )
        OR "timestamp" BETWEEN $2 AND $3
    )
ORDER BY
    "timestamp" ASC
            "#,
            mmsi.map(|v| v.into_inner()),
            range.map(|r| r.start()),
            range.map(|r| r.end()),
        )
        .fetch_all(self.db.ais_pool())
        .await
        .unwrap()
    }

    pub async fn all_current_ais_positions(&self) -> Vec<AisPosition> {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    mmsi AS "mmsi!: Mmsi",
    latitude,
    longitude,
    course_over_ground,
    rate_of_turn,
    true_heading,
    speed_over_ground,
    TIMESTAMP AS msgtime,
    navigation_status_id AS "navigational_status: NavigationStatus",
    distance_to_shore
FROM
    current_ais_positions
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    pub async fn all_historic_static_ais_messages(&self) -> Vec<AisVesselHistoric> {
        sqlx::query_as!(
            AisVesselHistoric,
            r#"
SELECT
    mmsi AS "mmsi!: Mmsi",
    imo_number,
    message_type_id,
    message_timestamp,
    call_sign,
    "name",
    ship_width,
    ship_length,
    ship_type,
    eta,
    draught,
    destination,
    dimension_a,
    dimension_b,
    dimension_c,
    dimension_d,
    position_fixing_device_type,
    report_class
FROM
    ais_vessels_historic
ORDER BY
    message_timestamp
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    pub async fn all_ais_vessels(&self) -> Vec<AisVessel> {
        let positions = sqlx::query_as!(
            crate::models::AisVessel,
            r#"
SELECT
    mmsi AS "mmsi!: Mmsi",
    imo_number,
    call_sign,
    NAME,
    ship_width,
    ship_length,
    eta,
    destination
FROM
    ais_vessels
            "#
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap();

        let mut converted = Vec::with_capacity(positions.len());

        for p in positions {
            let core_model = AisVessel::try_from(p).unwrap();
            converted.push(core_model);
        }

        converted
    }

    pub async fn create_test_database_from_template(&self, db_name: &str) {
        sqlx::query(&format!("CREATE DATABASE \"{db_name}\" TEMPLATE postgres;"))
            .execute(&self.db.pool)
            .await
            .unwrap();
    }

    pub async fn vessel(&self, vessel_id: FiskeridirVesselId) -> Vessel {
        Vessel::try_from(
            self.db
                .single_fiskeridir_ais_vessel_combination(vessel_id)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    pub async fn trips_of_vessel(&self, vessel_id: FiskeridirVesselId) -> Vec<Trip> {
        sqlx::query_as!(
            crate::models::Trip,
            r#"
SELECT
    trip_id AS "trip_id!: TripId",
    period,
    period_precision,
    landing_coverage,
    distance,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    start_port_id,
    end_port_id,
    target_species_fiskeridir_id,
    target_species_fao_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| Trip::try_from(v).unwrap())
        .collect()
    }

    pub async fn all_detailed_trips_of_vessels(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Vec<TripDetailed> {
        sqlx::query_as!(
            crate::models::TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!: TripId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    t.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    COALESCE(t.num_landings::BIGINT, 0) AS "num_deliveries!",
    COALESCE(t.landing_total_living_weight, 0.0) AS "total_living_weight!",
    COALESCE(t.landing_total_gross_weight, 0.0) AS "total_gross_weight!",
    COALESCE(t.landing_total_product_weight, 0.0) AS "total_product_weight!",
    COALESCE(t.delivery_point_ids, '{}') AS "delivery_points!",
    COALESCE(t.landing_gear_ids, '{}') AS "gear_ids!: Vec<Gear>",
    COALESCE(t.landing_gear_group_ids, '{}') AS "gear_group_ids!: Vec<GearGroup>",
    COALESCE(t.landing_species_group_ids, '{}') AS "species_group_ids!: Vec<SpeciesGroup>",
    t.most_recent_landing AS latest_landing_timestamp,
    COALESCE(t.landings::TEXT, '[]') AS "catches!",
    t.start_port_id,
    t.end_port_id,
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    COALESCE(t.vessel_events, '[]')::TEXT AS "vessel_events!",
    COALESCE(t.hauls, '[]')::TEXT AS "hauls!",
    COALESCE(t.fishing_facilities, '[]')::TEXT AS "fishing_facilities!",
    COALESCE(t.landing_ids, '{}') AS "landing_ids!",
    t.distance,
    t.cache_version,
    t.target_species_fiskeridir_id,
    t.target_species_fao_id
FROM
    trips_detailed t
WHERE
    t.fiskeridir_vessel_id = $1;
            "#,
            vessel_id.into_inner(),
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| TripDetailed::try_from(v).unwrap())
        .collect()
    }

    pub async fn generate_haul(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> kyogre_core::Haul {
        let message_id = random();
        let mut dca = ErsDca::test_default(message_id, Some(vessel_id));
        dca.message_info.set_message_timestamp(*start);
        dca.start_date = Some(start.date_naive());
        dca.start_time = Some(start.time());
        dca.stop_date = Some(end.date_naive());
        dca.stop_time = Some(end.time());
        self.add_ers_dca_value(dca).await;
        self.haul_of_vessel(vessel_id, message_id).await
    }

    pub async fn generate_ais_vessel(&self, mmsi: Mmsi, call_sign: &str) -> AisVessel {
        let val = NewAisStatic::test_default(mmsi, call_sign);

        self.db.add_ais_vessels(&[val]).await.unwrap();

        let mut vessels = self
            .all_ais_vessels()
            .await
            .into_iter()
            .filter(|v| v.mmsi == mmsi)
            .collect::<Vec<AisVessel>>();
        assert_eq!(vessels.len(), 1);

        vessels.pop().unwrap()
    }

    pub async fn generate_ais_position(
        &self,
        mmsi: Mmsi,
        timestamp: DateTime<chrono::Utc>,
    ) -> AisPosition {
        let pos = NewAisPosition::test_default(mmsi, timestamp);
        self.add_ais_position(pos).await
    }

    pub async fn generate_vms_position(
        &self,
        message_id: u32,
        call_sign: &CallSign,
        timestamp: DateTime<chrono::Utc>,
    ) -> VmsPosition {
        let pos = Vms::test_default(message_id, call_sign.clone(), timestamp);
        self.db.add_vms(vec![pos]).await.unwrap();
        self.single_vms_position(message_id).await
    }

    pub async fn add_landings(&self, landings: Vec<fiskeridir_rs::Landing>) {
        let year = landings[0].landing_timestamp.year() as u32;
        self.db
            .add_landings(Box::new(landings.into_iter().map(Ok)), year)
            .await
            .unwrap();
    }

    pub async fn generate_landings(
        &self,
        landings: Vec<(i64, FiskeridirVesselId, DateTime<Utc>)>,
    ) -> Vec<Landing> {
        let landings = landings
            .into_iter()
            .map(|(landing_id, vessel_id, timestamp)| {
                let mut landing = fiskeridir_rs::Landing::test_default(landing_id, Some(vessel_id));
                landing.landing_timestamp = timestamp;
                landing
            })
            .collect::<Vec<_>>();

        self.add_landings(landings.clone()).await;

        self.db
            .landings(Default::default())
            .unwrap()
            .try_collect()
            .await
            .unwrap()
    }

    pub async fn generate_landing(
        &self,
        landing_id: i64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
    ) -> Landing {
        self.generate_landings(vec![(landing_id, vessel_id, timestamp)])
            .await
            .pop()
            .unwrap()
    }

    pub async fn generate_tra(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
    ) {
        let tra = fiskeridir_rs::ErsTra::test_default(message_id, Some(vessel_id), timestamp);
        self.db.add_ers_tra(vec![tra]).await.unwrap();
    }

    pub async fn generate_fiskeridir_vessel(
        &self,
        id: FiskeridirVesselId,
        imo: Option<i64>,
        call_sign: Option<CallSign>,
    ) -> kyogre_core::Vessel {
        let mut vessel = fiskeridir_rs::RegisterVessel::test_default(id);
        vessel.imo_number = imo;
        vessel.radio_call_sign = call_sign;

        self.db.add_register_vessels(vec![vessel]).await.unwrap();

        self.vessel(id).await
    }

    pub async fn generate_ais_position_with_coordinates(
        &self,
        mmsi: Mmsi,
        timestamp: DateTime<chrono::Utc>,
        lat: f64,
        lon: f64,
    ) -> AisPosition {
        let mut pos = NewAisPosition::test_default(mmsi, timestamp);
        pos.latitude = lat;
        pos.longitude = lon;
        self.add_ais_position(pos).await
    }

    pub async fn generate_ers_dca(
        &self,
        message_id: u64,
        vessel_id: Option<FiskeridirVesselId>,
    ) -> ErsDca {
        let ers_dca = ErsDca::test_default(message_id, vessel_id);
        self.add_ers_dca_value(ers_dca.clone()).await;
        ers_dca
    }

    pub async fn set_port_coordinate(&self, port_id: &str, latitude: f64, longitude: f64) {
        sqlx::query!(
            r#"
UPDATE ports
SET
    latitude = $1,
    longitude = $2
WHERE
    port_id = $3
            "#,
            latitude,
            longitude,
            port_id,
        )
        .execute(&self.db.pool)
        .await
        .unwrap();
    }
    pub async fn set_dock_point_coordinate(
        &self,
        port_id: &str,
        dock_point_id: u32,
        latitude: f64,
        longitude: f64,
    ) {
        sqlx::query!(
            r#"
UPDATE port_dock_points
SET
    latitude = $1,
    longitude = $2
WHERE
    port_id = $3
    AND port_dock_point_id = $4
            "#,
            latitude,
            longitude,
            port_id,
            dock_point_id as i32,
        )
        .execute(&self.db.pool)
        .await
        .unwrap();
    }

    pub async fn generate_ais_vms_vessel_trail(
        &self,
        mmsi: Mmsi,
        call_sign: &CallSign,
        num_positions: usize,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<AisVmsPosition> {
        let time_step = (end - start).num_milliseconds() / num_positions as i64;

        let range = DateRange::new(start, end).unwrap();

        let mut ais_positions = Vec::with_capacity(num_positions / 2);
        let mut vms_positions = Vec::with_capacity(num_positions / 2);
        let mut last_generated_ais = true;
        for i in 0..num_positions {
            if last_generated_ais {
                let mut pos = Vms::test_default(
                    i as u32,
                    call_sign.clone(),
                    start + Duration::milliseconds(time_step * i as i64),
                );
                pos.latitude = Some(0.001 * i as f64);
                pos.longitude = Some(0.001 * i as f64);
                vms_positions.push(pos);
                last_generated_ais = false;
            } else {
                let mut pos = NewAisPosition::test_default(
                    mmsi,
                    start + Duration::milliseconds(time_step * i as i64),
                );
                pos.latitude = 0.001 * i as f64;
                pos.longitude = 0.001 * i as f64;
                ais_positions.push(pos);
                last_generated_ais = true;
            }
        }

        self.db.add_ais_positions(&ais_positions).await.unwrap();
        self.db.add_vms(vms_positions).await.unwrap();

        let db_positions = kyogre_core::TripPrecisionOutboundPort::ais_vms_positions(
            &self.db,
            Some(mmsi),
            Some(call_sign),
            &range,
        )
        .await
        .unwrap();

        assert_eq!(db_positions.len(), num_positions);
        db_positions
    }

    pub async fn add_ers_dca_value(&self, val: ErsDca) {
        self.add_ers_dca(vec![val]).await;
    }

    pub async fn add_ers_dca(&self, values: Vec<ErsDca>) {
        self.db
            .add_ers_dca(Box::new(values.into_iter().map(Ok)))
            .await
            .unwrap();
    }

    async fn add_ais_position(&self, pos: NewAisPosition) -> AisPosition {
        let timestamp = pos.msgtime;
        let mmsi = pos.mmsi;
        self.db.add_ais_positions(&[pos]).await.unwrap();

        let mut positions = self
            .ais_positions(
                Some(mmsi),
                Some(&DateRange::new(timestamp, timestamp).unwrap()),
            )
            .await;

        assert_eq!(positions.len(), 1);
        positions.pop().unwrap()
    }

    pub async fn generate_ers_departure_with_port(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        message_number: u32,
        port_id: &str,
    ) {
        let mut departure = ErsDep::test_default(message_id, vessel_id, timestamp, message_number);
        departure.port.code = Some(port_id.parse().unwrap());
        self.db.add_ers_dep(vec![departure]).await.unwrap();
    }

    pub async fn landing_ids_of_vessel(&self, vessel_id: FiskeridirVesselId) -> Vec<LandingId> {
        sqlx::query!(
            r#"
SELECT
    landing_id
FROM
    landings
WHERE
    fiskeridir_vessel_id = $1
ORDER BY
    landing_id
            "#,
            vessel_id.into_inner(),
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|v| LandingId::try_from(v.landing_id).unwrap())
        .collect()
    }

    pub async fn generate_ers_arrival_with_port(
        &self,
        message_id: u64,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        message_number: u32,
        port_id: &str,
    ) {
        let mut arrival = ErsPor::test_default(message_id, vessel_id, timestamp, message_number);
        arrival.port.code = Some(port_id.parse().unwrap());
        self.db.add_ers_por(vec![arrival]).await.unwrap();
    }

    pub async fn generate_fishing_facility(&self) -> FishingFacility {
        let facility = FishingFacility::test_default();
        self.db
            .add_fishing_facilities(vec![facility.clone()])
            .await
            .unwrap();
        facility
    }

    pub async fn add_fishing_facilities(&self, facilities: Vec<FishingFacility>) {
        self.db.add_fishing_facilities(facilities).await.unwrap();
    }

    pub async fn benchmark(
        &self,
        vessel_id: FiskeridirVesselId,
        benchmark: VesselBenchmarkId,
    ) -> f64 {
        sqlx::query!(
            r#"
SELECT
    output
FROM
    vessel_benchmark_outputs
WHERE
    fiskeridir_vessel_id = $1
    AND vessel_benchmark_id = $2
            "#,
            vessel_id.into_inner(),
            benchmark as i32,
        )
        .fetch_one(&self.db.pool)
        .await
        .unwrap()
        .output
    }

    async fn single_vms_position(&self, message_id: u32) -> VmsPosition {
        let pos = sqlx::query_as!(
            crate::models::VmsPosition,
            r#"
SELECT
    call_sign,
    course,
    latitude,
    longitude,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type,
    distance_to_shore
FROM
    vms_positions
WHERE
    message_id = $1
            "#,
            message_id as i32
        )
        .fetch_one(&self.db.pool)
        .await
        .unwrap();

        VmsPosition::try_from(pos).unwrap()
    }
}
