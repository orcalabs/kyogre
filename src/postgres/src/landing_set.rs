use crate::{error::PostgresErrorWrapper, models::*};
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
    data_year: u32,
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
    pub(crate) fn prepare(&mut self) -> PreparedLandingSet {
        let species = self.species.drain().map(|(_, v)| v).collect();
        let species_fao = self.species_fao.drain().map(|(_, v)| v).collect();
        let vessels = self.vessels.drain().map(|(_, v)| v).collect();
        let delivery_points = self.delivery_points.drain().collect();
        let catch_areas = self.catch_areas.drain().map(|(_, v)| v).collect();
        let catch_main_areas = self.catch_main_areas.drain().map(|(_, v)| v).collect();
        let area_groupings = self.area_groupings.drain().map(|(_, v)| v).collect();
        let species_fiskeridir = self.species_fiskeridir.drain().map(|(_, v)| v).collect();
        let landings = self.landings.drain().map(|(_, v)| v).collect();
        let landing_entries = self.landing_entries.drain(0..).collect();
        let counties = self.counties.drain().map(|(_, v)| v).collect();
        let municipalities = self.municipalities.drain().map(|(_, v)| v).collect();
        let catch_main_area_fao = self.catch_main_area_fao.drain().map(|(_, v)| v).collect();

        PreparedLandingSet {
            species,
            landings,
            landing_entries,
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

    pub(crate) fn with_capacity(capacity: usize, data_year: u32) -> Self {
        Self {
            species: HashMap::with_capacity(capacity),
            species_fao: HashMap::with_capacity(capacity),
            species_fiskeridir: HashMap::with_capacity(capacity),
            landings: HashMap::with_capacity(capacity),
            landing_entries: Vec::with_capacity(capacity),
            vessels: HashMap::with_capacity(capacity),
            delivery_points: HashSet::with_capacity(capacity),
            catch_areas: HashMap::with_capacity(capacity),
            catch_main_areas: HashMap::with_capacity(capacity),
            catch_main_area_fao: HashMap::with_capacity(capacity),
            area_groupings: HashMap::with_capacity(capacity),
            counties: HashMap::with_capacity(capacity),
            municipalities: HashMap::with_capacity(capacity),
            data_year,
        }
    }

    pub(crate) fn add_landing(
        &mut self,
        landing: fiskeridir_rs::Landing,
    ) -> Result<(), PostgresErrorWrapper> {
        self.add_vessel(&landing);
        self.add_species(&landing);
        self.add_species_fao(&landing);
        self.add_species_fiskeridir(&landing);
        self.add_delivery_point(&landing);
        self.add_catch_area(&landing)?;
        self.add_main_catch_area(&landing)?;
        self.add_main_catch_area_fao(&landing);
        self.add_fishing_region(&landing);
        self.add_municipality(&landing);
        self.add_county(&landing);
        self.add_landing_impl(&landing)?;
        self.add_landing_entry(&landing)?;
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.landings.len()
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

    fn add_catch_area(
        &mut self,
        landing: &fiskeridir_rs::Landing,
    ) -> Result<(), PostgresErrorWrapper> {
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
    ) -> Result<(), PostgresErrorWrapper> {
        if let Some(catch_area) = NewCatchMainArea::from_landing(landing)? {
            self.catch_main_areas
                .entry(catch_area.id as u32)
                .or_insert(catch_area);
        }
        Ok(())
    }

    fn add_landing_impl(
        &mut self,
        landing: &fiskeridir_rs::Landing,
    ) -> Result<(), PostgresErrorWrapper> {
        if self.landings.contains_key(&landing.id) {
            Ok(())
        } else {
            let new_landing = NewLanding::from_fiskeridir_landing(landing.clone(), self.data_year)?;
            self.landings.insert(landing.id.clone(), new_landing);
            Ok(())
        }
    }

    fn add_landing_entry(
        &mut self,
        landing: &fiskeridir_rs::Landing,
    ) -> Result<(), PostgresErrorWrapper> {
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
