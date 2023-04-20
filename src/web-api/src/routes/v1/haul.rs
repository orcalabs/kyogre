use std::ops::Bound;

use crate::{
    error::ApiError,
    response::Response,
    routes::utils::{
        self, deserialize_range_list, deserialize_string_list, DateTimeUtc, GearGroupId, Month,
        SpeciesGroupId,
    },
    to_streaming_response, Database,
};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use fiskeridir_rs::{Gear, GearGroup, VesselLengthGroup, WhaleGender};
use futures::TryStreamExt;
use kyogre_core::{
    ActiveHaulsFilter, CatchLocationId, FiskeridirVesselId, HaulId, HaulsMatrixQuery, HaulsQuery,
    Range,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsParams {
    #[param(value_type = Option<String>, example = "2023-01-01T00:00:00Z,2023-02-01T00:00:00Z")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<DateTimeUtc>>,
    #[param(value_type = Option<String>, example = "05-24,15-10")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "[0,11);[15,)")]
    #[serde(deserialize_with = "deserialize_range_list", default)]
    pub vessel_length_ranges: Option<Vec<Range<f64>>>,
    #[param(value_type = Option<String>, example = "2000013801,2001015304")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct HaulsMatrixParams {
    #[param(value_type = Option<String>, example = "24278,24280")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub months: Option<Vec<Month>>,
    #[param(value_type = Option<String>, example = "05-24,15-10")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub catch_locations: Option<Vec<CatchLocationId>>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "1,3")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub vessel_length_groups: Option<Vec<utils::VesselLengthGroup>>,
    #[param(value_type = Option<String>, example = "2000013801,2001015304")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
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
#[tracing::instrument(skip(db))]
pub async fn hauls<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<HaulsParams>,
) -> Result<HttpResponse, ApiError> {
    let query = params.into_inner().into();

    to_streaming_response! {
        db.hauls(query)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
                ApiError::InternalServerError
            })?
            .map_ok(Haul::from)
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve hauls: {:?}", e);
                ApiError::InternalServerError
            })
    }
}

#[utoipa::path(
    get,
    path = "/hauls_matrix/{active_filter}",
    params(
        HaulsMatrixParams,
        ("active_filter" = ActiveHaulsFilter, Path, description = "What feature to group by on the y-axis of the output matrices"),
    ),
    responses(
        (status = 200, description = "an aggregated matrix view of haul living weights", body = HaulsMatrix),
        (status = 400, description = "the provided parameters were invalid"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn hauls_matrix<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<HaulsMatrixParams>,
    active_filter: Path<ActiveHaulsFilter>,
) -> Result<Response<HaulsMatrix>, ApiError> {
    let query = matrix_params_to_query(params.into_inner(), active_filter.into_inner());

    let matrix = db.hauls_matrix(query).await.map_err(|e| {
        event!(Level::ERROR, "failed to retrieve hauls matrix: {:?}", e);
        ApiError::InternalServerError
    })?;

    Ok(Response::new(HaulsMatrix::from(matrix)))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Haul {
    #[schema(value_type = String)]
    pub haul_id: HaulId,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    #[schema(value_type = Option<String>, example = "05-24")]
    pub catch_location_start: Option<CatchLocationId>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_latitude: f64,
    pub start_longitude: f64,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    #[schema(value_type = String, example = "2023-02-24T11:08:20.409416682Z")]
    pub stop_timestamp: DateTime<Utc>,
    #[schema(value_type = i32)]
    pub gear_id: Gear,
    #[schema(value_type = i32)]
    pub gear_group_id: GearGroup,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: f64,
    #[schema(value_type = i32)]
    pub vessel_length_group: VesselLengthGroup,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub catches: Vec<HaulCatch>,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fiskeridir_id: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HaulsMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    #[schema(value_type = Option<i32>)]
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
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
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v.catch_location_start,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: v.vessel_length,
            vessel_length_group: v.vessel_length_group,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            catches: v.catches.into_iter().map(HaulCatch::from).collect(),
            whale_catches: v.whale_catches.into_iter().map(WhaleCatch::from).collect(),
        }
    }
}

impl From<kyogre_core::HaulCatch> for HaulCatch {
    fn from(v: kyogre_core::HaulCatch) -> Self {
        Self {
            living_weight: v.living_weight,
            species_fiskeridir_id: v.species_fiskeridir_id,
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

fn utc_from_naive(naive_date: NaiveDate) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(naive_date.and_hms_opt(0, 0, 0).unwrap(), Utc)
}

impl From<HaulsParams> for HaulsQuery {
    fn from(v: HaulsParams) -> Self {
        let ranges = v.months.map(|mut months| {
            let mut vec = Vec::with_capacity(months.len());

            months.sort();

            let mut start_naive = None;
            let mut end_naive = None;
            for m in months {
                if let (Some(start), Some(end)) = (start_naive, end_naive) {
                    let naive = NaiveDate::from_ymd_opt(m.0.year(), m.0.month(), 1).unwrap();
                    if end != naive {
                        vec.push(Range {
                            start: Bound::Included(utc_from_naive(start)),
                            end: Bound::Excluded(utc_from_naive(end)),
                        });
                        start_naive = Some(naive);
                    }
                    end_naive = Some(naive.checked_add_months(Months::new(1)).unwrap());
                } else {
                    let start = NaiveDate::from_ymd_opt(m.0.year(), m.0.month(), 1).unwrap();
                    end_naive = Some(start.checked_add_months(Months::new(1)).unwrap());
                    start_naive = Some(start);
                }
            }

            if let (Some(start), Some(end)) = (start_naive, end_naive) {
                vec.push(Range {
                    start: Bound::Included(utc_from_naive(start)),
                    end: Bound::Excluded(utc_from_naive(end)),
                });
            }

            vec
        });

        Self {
            ranges,
            catch_locations: v.catch_locations,
            gear_group_ids: v
                .gear_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            species_group_ids: v
                .species_group_ids
                .map(|gs| gs.into_iter().map(|g| g.0).collect()),
            vessel_length_ranges: v.vessel_length_ranges,
            vessel_ids: v.fiskeridir_vessel_ids,
        }
    }
}

fn matrix_params_to_query(
    params: HaulsMatrixParams,
    active_filter: ActiveHaulsFilter,
) -> HaulsMatrixQuery {
    HaulsMatrixQuery {
        months: params
            .months
            .map(|ms| ms.into_iter().map(|m| m.0).collect()),
        catch_locations: params.catch_locations,
        gear_group_ids: params
            .gear_group_ids
            .map(|gs| gs.into_iter().map(|g| g.0).collect()),
        species_group_ids: params
            .species_group_ids
            .map(|gs| gs.into_iter().map(|g| g.0).collect()),
        vessel_length_groups: params
            .vessel_length_groups
            .map(|lgs| lgs.into_iter().map(|l| l.0).collect()),
        active_filter,
        vessel_ids: params.fiskeridir_vessel_ids,
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use chrono::{DateTime, Utc};
    use kyogre_core::HaulsQuery;

    use crate::routes::utils::DateTimeUtc;

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
            months: Some(vec![
                DateTimeUtc(month1),
                DateTimeUtc(month2),
                DateTimeUtc(month3),
                DateTimeUtc(month4),
                DateTimeUtc(month5),
                DateTimeUtc(month6),
            ]),
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
