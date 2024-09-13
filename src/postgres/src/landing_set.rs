use std::collections::{HashMap, HashSet};

use fiskeridir_rs::LandingId;
use kyogre_core::FiskeridirVesselId;

use crate::models::*;

#[derive(Default)]
pub struct LandingSet<'a> {
    species: HashMap<i32, NewSpecies<'a>>,
    species_fao: HashMap<&'a str, NewSpeciesFao<'a>>,
    species_fiskeridir: HashMap<i32, NewSpeciesFiskeridir<'a>>,
    landings: HashMap<&'a LandingId, NewLanding<'a>>,
    landing_entries: Vec<NewLandingEntry<'a>>,
    vessels: HashMap<FiskeridirVesselId, NewFiskeridirVessel<'a>>,
    delivery_points: HashSet<NewDeliveryPointId<'a>>,
    catch_areas: HashMap<u32, NewCatchArea>,
    catch_main_areas: HashMap<u32, NewCatchMainArea<'a>>,
    catch_main_area_fao: HashMap<i32, NewCatchMainAreaFao<'a>>,
    area_groupings: HashMap<&'a str, NewAreaGrouping<'a>>,
    counties: HashMap<i32, NewCounty<'a>>,
    municipalities: HashMap<i32, NewMunicipality<'a>>,
    data_year: u32,
}

pub struct PreparedLandingSet<'a> {
    pub species: Vec<NewSpecies<'a>>,
    pub species_fao: Vec<NewSpeciesFao<'a>>,
    pub species_fiskeridir: Vec<NewSpeciesFiskeridir<'a>>,
    pub vessels: Vec<NewFiskeridirVessel<'a>>,
    pub delivery_points: Vec<NewDeliveryPointId<'a>>,
    pub catch_areas: Vec<NewCatchArea>,
    pub catch_main_areas: Vec<NewCatchMainArea<'a>>,
    pub catch_main_area_fao: Vec<NewCatchMainAreaFao<'a>>,
    pub area_groupings: Vec<NewAreaGrouping<'a>>,
    pub landings: Vec<NewLanding<'a>>,
    pub landing_entries: Vec<NewLandingEntry<'a>>,
    pub counties: Vec<NewCounty<'a>>,
    pub municipalities: Vec<NewMunicipality<'a>>,
}

impl<'a> LandingSet<'a> {
    pub(crate) fn prepare(self) -> PreparedLandingSet<'a> {
        let species = self.species.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let delivery_points = self.delivery_points.into_iter().collect();
        let catch_areas = self.catch_areas.into_values().collect();
        let catch_main_areas = self.catch_main_areas.into_values().collect();
        let area_groupings = self.area_groupings.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let landings = self.landings.into_values().collect();
        let landing_entries = self.landing_entries.into_iter().collect();
        let counties = self.counties.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let catch_main_area_fao = self.catch_main_area_fao.into_values().collect();

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

    pub(crate) fn new(
        landings: impl Iterator<Item = &'a fiskeridir_rs::Landing>,
        data_year: u32,
    ) -> Self {
        let (min, max) = landings.size_hint();
        let mut set = Self::with_capacity(max.unwrap_or(min), data_year);

        for l in landings {
            set.add_vessel(l);
            set.add_species(l);
            set.add_species_fao(l);
            set.add_species_fiskeridir(l);
            set.add_delivery_point(l);
            set.add_catch_area(l);
            set.add_main_catch_area(l);
            set.add_main_catch_area_fao(l);
            set.add_fishing_region(l);
            set.add_municipality(l);
            set.add_county(l);
            set.add_landing_impl(l);
            set.add_landing_entry(l);
        }

        set
    }

    fn add_municipality(&mut self, landing: &'a fiskeridir_rs::Landing) {
        for m in NewMunicipality::municipalities_from_landing(landing) {
            self.municipalities.entry(m.id).or_insert_with(|| m);
        }
    }

    fn add_county(&mut self, landing: &'a fiskeridir_rs::Landing) {
        for c in NewCounty::counties_from_landing(landing) {
            self.counties.entry(c.id).or_insert_with(|| c);
        }
    }

    fn add_delivery_point(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(id) = &landing.delivery_point.id {
            self.delivery_points.insert(id.into());
        }
        if let Some(id) = &landing.partial_landing_next_delivery_point_id {
            self.delivery_points.insert(id.into());
        }
        if let Some(id) = &landing.partial_landing_previous_delivery_point_id {
            self.delivery_points.insert(id.into());
        }
    }

    fn add_vessel(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(vessel_id) = landing.vessel.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| (&landing.vessel).into());
        }
    }

    fn add_fishing_region(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(region) = NewAreaGrouping::from_landing(landing) {
            self.area_groupings.entry(region.id).or_insert(region);
        }
    }

    fn add_catch_area(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(catch_area) = NewCatchArea::from_landing(landing) {
            self.catch_areas
                .entry(catch_area.id as u32)
                .or_insert(catch_area);
        }
    }

    fn add_main_catch_area_fao(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(area) = NewCatchMainAreaFao::from_landing(landing) {
            self.catch_main_area_fao.entry(area.id).or_insert(area);
        }
    }

    fn add_main_catch_area(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(catch_area) = NewCatchMainArea::from_landing(landing) {
            self.catch_main_areas
                .entry(catch_area.id as u32)
                .or_insert(catch_area);
        }
    }

    fn add_landing_impl(&mut self, landing: &'a fiskeridir_rs::Landing) {
        self.landings
            .entry(&landing.id)
            .or_insert_with(|| NewLanding::from_fiskeridir_landing(landing, self.data_year));
    }

    fn add_landing_entry(&mut self, landing: &'a fiskeridir_rs::Landing) {
        self.landing_entries.push(NewLandingEntry::from(landing));
    }

    fn add_species(&mut self, landing: &'a fiskeridir_rs::Landing) {
        let species = NewSpecies::from(&landing.product.species);
        self.species.entry(species.id).or_insert(species);
    }

    fn add_species_fao(&mut self, landing: &'a fiskeridir_rs::Landing) {
        if let Some(species_fao) = NewSpeciesFao::from_landing_species(&landing.product.species) {
            self.species_fao
                .entry(species_fao.id)
                .or_insert(species_fao);
        }
    }

    fn add_species_fiskeridir(&mut self, landing: &'a fiskeridir_rs::Landing) {
        let species_fiskeridir = NewSpeciesFiskeridir::from(&landing.product.species);
        self.species_fiskeridir
            .entry(species_fiskeridir.id)
            .or_insert(species_fiskeridir);
    }
}
