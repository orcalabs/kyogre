mod matrix_cache {
    tonic::include_proto!("matrix_cache");
}

pub use matrix_cache::*;

use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, HaulsMatrixQuery,
    LandingMatrixQuery,
};
use num_traits::FromPrimitive;

use crate::error::{Error, Result, error::InvalidParametersSnafu};

impl From<LandingMatrix> for kyogre_core::LandingMatrix {
    fn from(value: LandingMatrix) -> Self {
        let LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        kyogre_core::LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<LandingMatrixQuery> for LandingFeatures {
    fn from(value: LandingMatrixQuery) -> Self {
        let LandingMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter,
        } = value;

        LandingFeatures {
            active_filter: active_filter as u32,
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocation {
                    main_area_id: v.main_area() as u32,
                    catch_area_id: v.catch_area() as u32,
                })
                .collect(),
            species_group_ids: species_group_ids.into_iter().map(|v| v as u32).collect(),
            gear_group_ids: gear_group_ids.into_iter().map(|v| v as u32).collect(),
            vessel_length_groups: vessel_length_groups.into_iter().map(|v| v as u32).collect(),
            fiskeridir_vessel_ids: vessel_ids.into_iter().map(|v| v.into_inner()).collect(),
        }
    }
}

impl From<kyogre_core::LandingMatrix> for LandingMatrix {
    fn from(value: kyogre_core::LandingMatrix) -> Self {
        let kyogre_core::LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        LandingMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl From<HaulMatrix> for kyogre_core::HaulsMatrix {
    fn from(value: HaulMatrix) -> Self {
        let HaulMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        kyogre_core::HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl TryFrom<LandingFeatures> for LandingMatrixQuery {
    type Error = Error;

    fn try_from(value: LandingFeatures) -> Result<Self> {
        let LandingFeatures {
            active_filter,
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
        } = value;

        Ok(Self {
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocationId::new(v.main_area_id as i32, v.catch_area_id as i32))
                .collect(),
            gear_group_ids: gear_group_ids
                .into_iter()
                .map(|v| {
                    GearGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            species_group_ids: species_group_ids
                .into_iter()
                .map(|v| {
                    SpeciesGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_length_groups: vessel_length_groups
                .into_iter()
                .map(|v| {
                    VesselLengthGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_ids: fiskeridir_vessel_ids
                .into_iter()
                .map(FiskeridirVesselId::new)
                .collect(),
            active_filter: ActiveLandingFilter::from_u32(active_filter).ok_or_else(|| {
                InvalidParametersSnafu {
                    value: active_filter,
                }
                .build()
            })?,
        })
    }
}

impl From<HaulsMatrixQuery> for HaulFeatures {
    fn from(value: HaulsMatrixQuery) -> Self {
        let HaulsMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter,
            bycatch_percentage,
            majority_species_group,
        } = value;

        HaulFeatures {
            active_filter: active_filter as u32,
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocation {
                    main_area_id: v.main_area() as u32,
                    catch_area_id: v.catch_area() as u32,
                })
                .collect(),
            species_group_ids: species_group_ids.into_iter().map(|v| v as u32).collect(),
            gear_group_ids: gear_group_ids.into_iter().map(|v| v as u32).collect(),
            vessel_length_groups: vessel_length_groups.into_iter().map(|v| v as u32).collect(),
            fiskeridir_vessel_ids: vessel_ids.into_iter().map(|v| v.into_inner()).collect(),
            bycatch_percentage,
            majority_species_group,
        }
    }
}

impl From<kyogre_core::HaulsMatrix> for HaulMatrix {
    fn from(value: kyogre_core::HaulsMatrix) -> Self {
        let kyogre_core::HaulsMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        } = value;

        HaulMatrix {
            dates,
            length_group,
            gear_group,
            species_group,
        }
    }
}

impl TryFrom<HaulFeatures> for HaulsMatrixQuery {
    type Error = Error;

    fn try_from(value: HaulFeatures) -> Result<Self> {
        let HaulFeatures {
            active_filter,
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
            bycatch_percentage,
            majority_species_group,
        } = value;

        Ok(Self {
            months,
            catch_locations: catch_locations
                .into_iter()
                .map(|v| CatchLocationId::new(v.main_area_id as i32, v.catch_area_id as i32))
                .collect(),
            gear_group_ids: gear_group_ids
                .into_iter()
                .map(|v| {
                    GearGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            species_group_ids: species_group_ids
                .into_iter()
                .map(|v| {
                    SpeciesGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_length_groups: vessel_length_groups
                .into_iter()
                .map(|v| {
                    VesselLengthGroup::from_u32(v)
                        .ok_or_else(|| InvalidParametersSnafu { value: v }.build())
                })
                .collect::<Result<Vec<_>>>()?,
            vessel_ids: fiskeridir_vessel_ids
                .into_iter()
                .map(FiskeridirVesselId::new)
                .collect(),
            active_filter: ActiveHaulsFilter::from_u32(active_filter).ok_or_else(|| {
                InvalidParametersSnafu {
                    value: active_filter,
                }
                .build()
            })?,
            bycatch_percentage,
            majority_species_group,
        })
    }
}
