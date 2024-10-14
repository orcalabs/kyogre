use actix_web::web::{self, Path};
use chrono::{DateTime, Datelike, Utc};
use fiskeridir_rs::{CallSign, Gear, GearGroup, SpeciesGroup, VesselLengthGroup, WhaleGender};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveHaulsFilter, CatchLocationId, FiskeridirVesselId, HaulId, HaulMatrixXFeature,
    HaulMatrixYFeature, HaulsMatrixQuery, HaulsQuery, HaulsSorting, Ordering,
};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::Result,
    response::{Response, ResponseOrStream, StreamResponse},
    routes::utils::*,
    stream_response, Cache, Database, Meilisearch,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsParams {
    #[param(rename = "months[]")]
    pub months: Option<Vec<DateTime<Utc>>>,
    #[param(rename = "catchLocations[]", value_type = Option<Vec<String>>)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(rename = "gearGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[param(rename = "speciesGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[param(rename = "vesselLengthGroups[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub sorting: Option<HaulsSorting>,
    pub ordering: Option<Ordering>,
}

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsMatrixParams {
    #[param(rename = "months[]")]
    pub months: Option<Vec<u32>>,
    #[param(rename = "catchLocations[]", value_type = Option<Vec<String>>)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(rename = "gearGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[param(rename = "speciesGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[param(rename = "vesselLengthGroups[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub bycatch_percentage: Option<f64>,
    pub majority_species_group: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/hauls",
    params(HaulsParams),
    responses(
        (status = 200, description = "all hauls", body = [Haul]),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db, meilisearch))]
pub async fn hauls<T: Database + Send + Sync + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    params: Query<HaulsParams>,
) -> Result<ResponseOrStream<Haul>> {
    let query: HaulsQuery = params.into_inner().into();

    if let Some(meilisearch) = meilisearch.as_ref() {
        return Ok(Response::new(
            meilisearch
                .hauls(&query)
                .await?
                .into_iter()
                .map(Haul::from)
                .collect::<Vec<_>>(),
        )
        .into());
    }

    let response = stream_response! {
        db.hauls(query).map_ok(Haul::from)
    };

    Ok(response.into())
}

#[serde_as]
#[derive(Debug, Deserialize, IntoParams)]
pub struct HaulsMatrixPath {
    #[serde_as(as = "DisplayFromStr")]
    pub active_filter: ActiveHaulsFilter,
}

#[utoipa::path(
    get,
    path = "/hauls_matrix/{active_filter}",
    params(
        HaulsMatrixParams,
        HaulsMatrixPath,
    ),
    responses(
        (status = 200, description = "an aggregated matrix view of haul living weights", body = HaulsMatrix),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db, cache))]
pub async fn hauls_matrix<T: Database + 'static, S: Cache>(
    db: web::Data<T>,
    cache: web::Data<Option<S>>,
    params: Query<HaulsMatrixParams>,
    path: Path<HaulsMatrixPath>,
) -> Result<Response<HaulsMatrix>> {
    let query = matrix_params_to_query(params.into_inner(), path.active_filter);

    if let Some(cache) = cache.as_ref() {
        if let Some(matrix) = cache.hauls_matrix(&query).await? {
            return Ok(Response::new(HaulsMatrix::from(matrix)));
        }
    }

    // Requests for prior month's data or newer will not exist in the database, but the query will
    // still take over 10s to complete which we want to avoid.
    if let Some(months) = &query.months {
        let current_time = Utc::now();
        let month_cutoff = (current_time.year() * 12 + current_time.month0() as i32 - 1) as u32;

        if !months.iter().any(|v| *v < month_cutoff) {
            return Ok(Response::new(HaulsMatrix::empty(path.active_filter)));
        }
    }

    let matrix = db.hauls_matrix(&query).await?;

    Ok(Response::new(HaulsMatrix::from(matrix)))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    #[schema(value_type = i64)]
    pub haul_id: HaulId,
    pub haul_distance: Option<i32>,
    #[schema(value_type = Option<Vec<String>>, example = "[05-24,01-01]")]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub start_timestamp: DateTime<Utc>,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub stop_timestamp: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_group_id: GearGroup,
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub catches: Vec<HaulCatch>,
    #[schema(value_type = Option<i64>)]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_name: Option<String>,
    #[schema(value_type = String)]
    pub call_sign: CallSign,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fiskeridir_id: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HaulsMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulWeather {
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulOceanClimate {
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub water_temperature: Option<f64>,
    pub ocean_climate_depth: Option<f64>,
    pub sea_floor_depth: Option<f64>,
}

impl HaulsMatrix {
    fn empty(active_filter: ActiveHaulsFilter) -> HaulsMatrix {
        let x_feature: HaulMatrixXFeature = active_filter.into();
        let dates_size = if x_feature == HaulMatrixXFeature::Date {
            HaulMatrixYFeature::Date.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::Date.size() * x_feature.size()
        };

        let length_group_size = if x_feature == HaulMatrixXFeature::VesselLength {
            HaulMatrixYFeature::VesselLength.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::VesselLength.size() * x_feature.size()
        };

        let gear_group_size = if x_feature == HaulMatrixXFeature::GearGroup {
            HaulMatrixYFeature::GearGroup.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::GearGroup.size() * x_feature.size()
        };

        let species_group_size = if x_feature == HaulMatrixXFeature::SpeciesGroup {
            HaulMatrixYFeature::SpeciesGroup.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::SpeciesGroup.size() * x_feature.size()
        };

        HaulsMatrix {
            dates: vec![0; dates_size],
            length_group: vec![0; length_group_size],
            gear_group: vec![0; gear_group_size],
            species_group: vec![0; species_group_size],
        }
    }
}

impl From<kyogre_core::HaulsMatrix> for HaulsMatrix {
    fn from(v: kyogre_core::HaulsMatrix) -> Self {
        HaulsMatrix {
            dates: v.dates,
            length_group: v.length_group,
            gear_group: v.gear_group,
            species_group: v.species_group,
        }
    }
}

impl From<kyogre_core::Haul> for Haul {
    fn from(v: kyogre_core::Haul) -> Self {
        Haul {
            haul_id: v.haul_id,
            haul_distance: v.haul_distance,
            catch_locations: v.catch_locations,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_timestamp: v.stop_timestamp,
            gear_group_id: v.gear_group_id,
            vessel_name: v.vessel_name,
            catches: v.catches.into_iter().map(HaulCatch::from).collect(),
            gear: v.gear_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            call_sign: v.call_sign,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
        }
    }
}

impl From<kyogre_core::HaulWeather> for HaulWeather {
    fn from(v: kyogre_core::HaulWeather) -> Self {
        Self {
            wind_speed_10m: v.wind_speed_10m,
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
            cloud_area_fraction: v.cloud_area_fraction,
        }
    }
}

impl From<kyogre_core::HaulOceanClimate> for HaulOceanClimate {
    fn from(v: kyogre_core::HaulOceanClimate) -> Self {
        Self {
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            salinity: v.salinity,
            water_temperature: v.water_temperature,
            ocean_climate_depth: v.ocean_climate_depth,
            sea_floor_depth: v.sea_floor_depth,
        }
    }
}

impl PartialEq<Haul> for kyogre_core::Haul {
    fn eq(&self, other: &Haul) -> bool {
        let converted: Haul = self.clone().into();
        converted.eq(other)
    }
}

impl PartialEq<kyogre_core::Haul> for Haul {
    fn eq(&self, other: &kyogre_core::Haul) -> bool {
        other.eq(self)
    }
}

impl From<kyogre_core::HaulCatch> for HaulCatch {
    fn from(v: kyogre_core::HaulCatch) -> Self {
        Self {
            living_weight: v.living_weight,
            species_fiskeridir_id: v.species_fiskeridir_id,
            species_group_id: v.species_group_id,
        }
    }
}

impl From<kyogre_core::WhaleCatch> for WhaleCatch {
    fn from(v: kyogre_core::WhaleCatch) -> Self {
        Self {
            blubber_measure_a: v.blubber_measure_a,
            blubber_measure_b: v.blubber_measure_b,
            blubber_measure_c: v.blubber_measure_c,
            circumference: v.circumference,
            fetus_length: v.fetus_length,
            gender_id: v.gender_id,
            grenade_number: v.grenade_number,
            individual_number: v.individual_number,
            length: v.length,
        }
    }
}

impl From<HaulsParams> for HaulsQuery {
    fn from(v: HaulsParams) -> Self {
        Self {
            ranges: v.months.map(months_to_date_ranges),
            catch_locations: v.catch_locations,
            gear_group_ids: v.gear_group_ids,
            species_group_ids: v.species_group_ids,
            vessel_length_groups: v.vessel_length_groups,
            vessel_ids: v.fiskeridir_vessel_ids,
            sorting: v.sorting,
            ordering: v.ordering,
        }
    }
}

pub fn matrix_params_to_query(
    params: HaulsMatrixParams,
    active_filter: ActiveHaulsFilter,
) -> HaulsMatrixQuery {
    HaulsMatrixQuery {
        months: params.months,
        catch_locations: params.catch_locations,
        gear_group_ids: params.gear_group_ids,
        species_group_ids: params.species_group_ids,
        vessel_length_groups: params.vessel_length_groups,
        active_filter,
        vessel_ids: params.fiskeridir_vessel_ids,
        bycatch_percentage: params.bycatch_percentage,
        majority_species_group: params.majority_species_group.unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use chrono::{DateTime, Utc};
    use kyogre_core::HaulsQuery;

    use super::HaulsParams;

    #[test]
    fn hauls_params_merges_consecutive_date_ranges() {
        let month1: DateTime<Utc> = "2001-11-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2001-12-1T00:00:00Z".parse().unwrap();
        let month3: DateTime<Utc> = "2002-01-1T00:00:00Z".parse().unwrap();
        let res1: DateTime<Utc> = "2002-02-1T00:00:00Z".parse().unwrap();

        let month4: DateTime<Utc> = "2002-06-1T00:00:00Z".parse().unwrap();
        let month5: DateTime<Utc> = "2002-07-1T00:00:00Z".parse().unwrap();
        let month6: DateTime<Utc> = "2002-08-1T00:00:00Z".parse().unwrap();
        let res2: DateTime<Utc> = "2002-09-1T00:00:00Z".parse().unwrap();

        let params = HaulsParams {
            months: Some(vec![month1, month2, month3, month4, month5, month6]),
            ..Default::default()
        };

        let query = HaulsQuery::from(params);
        let ranges = query.ranges.unwrap();

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, Bound::Included(month1));
        assert_eq!(ranges[0].end, Bound::Excluded(res1));
        assert_eq!(ranges[1].start, Bound::Included(month4));
        assert_eq!(ranges[1].end, Bound::Excluded(res2));
    }
}
