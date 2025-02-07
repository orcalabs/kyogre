use actix_web::web::{self, Path};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear, GearGroup, SpeciesGroup, VesselLengthGroup, WhaleGender};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveHaulsFilter, CatchLocationId, FiskeridirVesselId, HaulId, HaulsMatrixQuery, HaulsQuery,
    HaulsSorting, Ordering, TripId,
};
use oasgen::{oasgen, OaSchema};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use tracing::error;

use crate::{
    error::Result,
    response::{Response, ResponseOrStream, StreamResponse},
    routes::utils::*,
    stream_response, Cache, Database, Meilisearch,
};

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HaulsParams {
    #[oasgen(rename = "months[]")]
    pub months: Option<Vec<DateTime<Utc>>>,
    #[oasgen(rename = "catchLocations[]")]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[oasgen(rename = "gearGroupIds[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[oasgen(rename = "speciesGroupIds[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[oasgen(rename = "vesselLengthGroups[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    #[oasgen(rename = "fiskeridirVesselIds[]")]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub sorting: Option<HaulsSorting>,
    pub ordering: Option<Ordering>,
}

#[serde_as]
#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HaulsMatrixParams {
    #[oasgen(rename = "months[]")]
    pub months: Option<Vec<u32>>,
    #[oasgen(rename = "catchLocations[]")]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[oasgen(rename = "gearGroupIds[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[oasgen(rename = "speciesGroupIds[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[oasgen(rename = "vesselLengthGroups[]")]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    #[oasgen(rename = "fiskeridirVesselIds[]")]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub bycatch_percentage: Option<f64>,
    pub majority_species_group: Option<bool>,
}

/// Returns all hauls matching the provided parameters.
#[oasgen(skip(db, meilisearch), tags("Haul"))]
#[tracing::instrument(skip(db, meilisearch))]
pub async fn hauls<T: Database + Send + Sync + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    params: Query<HaulsParams>,
) -> Result<ResponseOrStream<Haul>> {
    let query: HaulsQuery = params.into_inner().into();

    if let Some(meilisearch) = meilisearch.as_ref() {
        match meilisearch.hauls(&query).await {
            Ok(v) => {
                return Ok(Response::new(v.into_iter().map(Haul::from).collect::<Vec<_>>()).into())
            }
            Err(e) => {
                error!("meilisearch cache returned error: {e:?}");
            }
        }
    }

    let response = stream_response! {
        db.hauls(query).map_ok(Haul::from)
    };

    Ok(response.into())
}

#[serde_as]
#[derive(Debug, Deserialize, OaSchema)]
pub struct HaulsMatrixPath {
    #[serde_as(as = "DisplayFromStr")]
    pub active_filter: ActiveHaulsFilter,
}

/// Returns an aggregated matrix view of haul living weights.
#[oasgen(skip(db, cache), tags("Haul"))]
#[tracing::instrument(skip(db, cache))]
pub async fn hauls_matrix<T: Database + 'static, S: Cache>(
    db: web::Data<T>,
    cache: web::Data<Option<S>>,
    params: Query<HaulsMatrixParams>,
    path: Path<HaulsMatrixPath>,
) -> Result<Response<HaulsMatrix>> {
    let query = matrix_params_to_query(params.into_inner(), path.active_filter);

    if let Some(cache) = cache.as_ref() {
        match cache.hauls_matrix(&query).await {
            Ok(matrix) => return Ok(Response::new(HaulsMatrix::from(matrix))),
            Err(e) => {
                error!("matrix cache returned error: {e:?}");
            }
        }
    }

    let matrix = db.hauls_matrix(&query).await?;
    Ok(Response::new(HaulsMatrix::from(matrix)))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    pub id: HaulId,
    pub trip_id: Option<TripId>,
    pub haul_distance: Option<i32>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_group_id: GearGroup,
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub catches: Vec<HaulCatch>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_name: Option<String>,
    pub call_sign: CallSign,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fiskeridir_id: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HaulsMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulOceanClimate {
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub water_temperature: Option<f64>,
    pub ocean_climate_depth: Option<f64>,
    pub sea_floor_depth: Option<f64>,
}

impl From<kyogre_core::HaulsMatrix> for HaulsMatrix {
    fn from(v: kyogre_core::HaulsMatrix) -> Self {
        let kyogre_core::HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = v;

        HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<kyogre_core::Haul> for Haul {
    fn from(v: kyogre_core::Haul) -> Self {
        let kyogre_core::Haul {
            id,
            trip_id,
            cache_version: _,
            catch_locations,
            gear_group_id,
            gear_id,
            species_group_ids: _,
            fiskeridir_vessel_id,
            haul_distance,
            start_latitude,
            start_longitude,
            stop_latitude,
            stop_longitude,
            start_timestamp,
            stop_timestamp,
            vessel_length_group: _,
            catches,
            vessel_name,
            call_sign,
        } = v;

        Haul {
            id,
            trip_id,
            haul_distance,
            catch_locations,
            start_latitude,
            start_longitude,
            start_timestamp,
            stop_timestamp,
            gear_group_id,
            vessel_name,
            catches: catches.into_iter().map(HaulCatch::from).collect(),
            gear: gear_id,
            fiskeridir_vessel_id,
            call_sign,
            stop_latitude,
            stop_longitude,
        }
    }
}

impl From<kyogre_core::HaulWeather> for HaulWeather {
    fn from(v: kyogre_core::HaulWeather) -> Self {
        let kyogre_core::HaulWeather {
            wind_speed_10m,
            wind_direction_10m,
            air_temperature_2m,
            relative_humidity_2m,
            air_pressure_at_sea_level,
            precipitation_amount,
            cloud_area_fraction,
        } = v;

        Self {
            wind_speed_10m,
            wind_direction_10m,
            air_temperature_2m,
            relative_humidity_2m,
            air_pressure_at_sea_level,
            precipitation_amount,
            cloud_area_fraction,
        }
    }
}

impl From<kyogre_core::HaulOceanClimate> for HaulOceanClimate {
    fn from(v: kyogre_core::HaulOceanClimate) -> Self {
        let kyogre_core::HaulOceanClimate {
            water_speed,
            water_direction,
            salinity,
            water_temperature,
            ocean_climate_depth,
            sea_floor_depth,
        } = v;

        Self {
            water_speed,
            water_direction,
            salinity,
            water_temperature,
            ocean_climate_depth,
            sea_floor_depth,
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
        let kyogre_core::HaulCatch {
            living_weight,
            species_fao_id: _,
            species_fiskeridir_id,
            species_group_id,
            species_main_group_id: _,
        } = v;

        Self {
            living_weight,
            species_fiskeridir_id,
            species_group_id,
        }
    }
}

impl From<kyogre_core::WhaleCatch> for WhaleCatch {
    fn from(v: kyogre_core::WhaleCatch) -> Self {
        let kyogre_core::WhaleCatch {
            blubber_measure_a,
            blubber_measure_b,
            blubber_measure_c,
            circumference,
            fetus_length,
            gender_id,
            grenade_number,
            individual_number,
            length,
        } = v;

        Self {
            blubber_measure_a,
            blubber_measure_b,
            blubber_measure_c,
            circumference,
            fetus_length,
            gender_id,
            grenade_number,
            individual_number,
            length,
        }
    }
}

impl From<HaulsParams> for HaulsQuery {
    fn from(v: HaulsParams) -> Self {
        let HaulsParams {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
            sorting,
            ordering,
        } = v;

        Self {
            ranges: months_to_date_ranges(months.unwrap_or_default()),
            catch_locations: catch_locations.unwrap_or_default(),
            gear_group_ids: gear_group_ids.unwrap_or_default(),
            species_group_ids: species_group_ids.unwrap_or_default(),
            vessel_length_groups: vessel_length_groups.unwrap_or_default(),
            vessel_ids: fiskeridir_vessel_ids.unwrap_or_default(),
            sorting,
            ordering,
        }
    }
}

pub fn matrix_params_to_query(
    params: HaulsMatrixParams,
    active_filter: ActiveHaulsFilter,
) -> HaulsMatrixQuery {
    let HaulsMatrixParams {
        months,
        catch_locations,
        gear_group_ids,
        species_group_ids,
        vessel_length_groups,
        fiskeridir_vessel_ids,
        bycatch_percentage,
        majority_species_group,
    } = params;

    HaulsMatrixQuery {
        months: months.unwrap_or_default(),
        catch_locations: catch_locations.unwrap_or_default(),
        gear_group_ids: gear_group_ids.unwrap_or_default(),
        species_group_ids: species_group_ids.unwrap_or_default(),
        vessel_length_groups: vessel_length_groups.unwrap_or_default(),
        active_filter,
        vessel_ids: fiskeridir_vessel_ids.unwrap_or_default(),
        bycatch_percentage,
        majority_species_group: majority_species_group.unwrap_or(false),
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
        let ranges = query.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, Bound::Included(month1));
        assert_eq!(ranges[0].end, Bound::Excluded(res1));
        assert_eq!(ranges[1].start, Bound::Included(month4));
        assert_eq!(ranges[1].end, Bound::Excluded(res2));
    }
}
