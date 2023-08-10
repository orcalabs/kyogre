use crate::{error::PostgresError, models::*};
use error_stack::Result;
use fiskeridir_rs::LandingId;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct LandingSet {
    species: HashMap<i32, Species>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    landings: HashMap<LandingId, NewLanding>,
    landing_entries: Vec<NewLandingEntry>,
    vessels: HashMap<i64, fiskeridir_rs::Vessel>,
    delivery_points: HashSet<NewDeliveryPointId>,
    catch_areas: HashMap<u32, NewCatchArea>,
    catch_main_areas: HashMap<u32, NewCatchMainArea>,
    catch_main_area_fao: HashMap<i32, NewCatchMainAreaFao>,
    area_groupings: HashMap<String, NewAreaGrouping>,
    counties: HashMap<i32, NewCounty>,
    municipalities: HashMap<i32, NewMunicipality>,
}

pub struct PreparedLandingSet {
    pub species: Vec<Species>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub vessels: Vec<fiskeridir_rs::Vessel>,
    pub delivery_points: Vec<NewDeliveryPointId>,
    pub catch_areas: Vec<NewCatchArea>,
    pub catch_main_areas: Vec<NewCatchMainArea>,
    pub catch_main_area_fao: Vec<NewCatchMainAreaFao>,
    pub area_groupings: Vec<NewAreaGrouping>,
    pub landings: Vec<NewLanding>,
    pub landing_entries: Vec<NewLandingEntry>,
    pub counties: Vec<NewCounty>,
    pub municipalities: Vec<NewMunicipality>,
}

impl LandingSet {
    pub(crate) fn prepare(self) -> PreparedLandingSet {
        let species = self.species.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let delivery_points = self.delivery_points.into_iter().collect();
        let catch_areas = self.catch_areas.into_values().collect();
        let catch_main_areas = self.catch_main_areas.into_values().collect();
        let area_groupings = self.area_groupings.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let landings = self.landings.into_values().collect();
        let counties = self.counties.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let catch_main_area_fao = self.catch_main_area_fao.into_values().collect();

        PreparedLandingSet {
            species,
            landings,
            landing_entries: self.landing_entries,
            vessels,
            delivery_points,
            species_fao,
            catch_areas,
            catch_main_areas,
            area_groupings,
            species_fiskeridir,
            counties,
            municipalities,
            catch_main_area_fao,
        }
    }

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::Landing>>(
        landings: T,
        data_year: u32,
    ) -> Result<LandingSet, PostgresError> {
        let mut set = LandingSet::default();
        for l in landings.into_iter() {
            set.add_vessel(&l);
            set.add_species(&l);
            set.add_species_fao(&l);
            set.add_species_fiskeridir(&l);
            set.add_delivery_point(&l);
            set.add_catch_area(&l)?;
            set.add_main_catch_area(&l)?;
            set.add_main_catch_area_fao(&l);
            set.add_fishing_region(&l);
            set.add_municipality(&l);
            set.add_county(&l);
            set.add_landing(&l, data_year)?;
            set.add_landing_entry(l)?;
        }
        Ok(set)
    }

    fn add_municipality(&mut self, landing: &fiskeridir_rs::Landing) {
        for m in NewMunicipality::municipalities_from_landing(landing) {
            self.municipalities.entry(m.id).or_insert_with(|| m);
        }
    }

    fn add_county(&mut self, landing: &fiskeridir_rs::Landing) {
        for c in NewCounty::counties_from_landing(landing) {
            self.counties.entry(c.id).or_insert_with(|| c);
        }
    }

    fn add_delivery_point(&mut self, landing: &fiskeridir_rs::Landing) {
        if let Some(id) = &landing.delivery_point.id {
            self.delivery_points.insert(id.clone().into());
        }
        if let Some(id) = &landing.partial_landing_next_delivery_point_id {
            self.delivery_points.insert(id.clone().into());
        }
        if let Some(id) = &landing.partial_landing_previous_delivery_point_id {
            self.delivery_points.insert(id.clone().into());
        }
    }

    fn add_vessel(&mut self, landing: &fiskeridir_rs::Landing) {
        if let Some(vessel_id) = landing.vessel.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| landing.vessel.clone());
        }
    }

    fn add_fishing_region(&mut self, landing: &fiskeridir_rs::Landing) {
        if let Some(region) = NewAreaGrouping::from_landing(landing) {
            self.area_groupings
                .entry(region.id.clone())
                .or_insert(region);
        }
    }

    fn add_catch_area(&mut self, landing: &fiskeridir_rs::Landing) -> Result<(), PostgresError> {
        if let Some(catch_area) = NewCatchArea::from_landing(landing)? {
            self.catch_areas
                .entry(catch_area.id as u32)
                .or_insert(catch_area);
        }
        Ok(())
    }

    fn add_main_catch_area_fao(&mut self, landing: &fiskeridir_rs::Landing) {
        if let Some(area) = NewCatchMainAreaFao::from_landing(landing) {
            self.catch_main_area_fao.entry(area.id).or_insert(area);
        }
    }

    fn add_main_catch_area(
        &mut self,
        landing: &fiskeridir_rs::Landing,
    ) -> Result<(), PostgresError> {
        if let Some(catch_area) = NewCatchMainArea::from_landing(landing)? {
            self.catch_main_areas
                .entry(catch_area.id as u32)
                .or_insert(catch_area);
        }
        Ok(())
    }

    fn add_landing(
        &mut self,
        landing: &fiskeridir_rs::Landing,
        data_year: u32,
    ) -> Result<(), PostgresError> {
        if self.landings.contains_key(&landing.id) {
            Ok(())
        } else {
            let new_landing = NewLanding::from_fiskeridir_landing(landing.clone(), data_year)?;
            self.landings.insert(landing.id.clone(), new_landing);
            Ok(())
        }
    }

    fn add_landing_entry(&mut self, landing: fiskeridir_rs::Landing) -> Result<(), PostgresError> {
        self.landing_entries
            .push(NewLandingEntry::try_from(landing)?);
        Ok(())
    }

    fn add_species(&mut self, landing: &fiskeridir_rs::Landing) {
        let species = Species::from(&landing.product.species);
        self.species.entry(species.id).or_insert(species);
    }

    fn add_species_fao(&mut self, landing: &fiskeridir_rs::Landing) {
        if let Some(species_fao) = SpeciesFao::from_landing_species(&landing.product.species) {
            self.species_fao
                .entry(species_fao.id.clone())
                .or_insert(species_fao);
        }
    }

    fn add_species_fiskeridir(&mut self, landing: &fiskeridir_rs::Landing) {
        let species_fiskeridir = SpeciesFiskeridir::from(&landing.product.species);
        self.species_fiskeridir
            .entry(species_fiskeridir.id)
            .or_insert(species_fiskeridir);
    }
}
