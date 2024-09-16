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

impl<'a> LandingSet<'a> {
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

    pub(crate) fn assert_is_empty(&self) {
        let Self {
            species,
            species_fao,
            species_fiskeridir,
            landings,
            landing_entries,
            vessels,
            delivery_points,
            catch_areas,
            catch_main_areas,
            catch_main_area_fao,
            area_groupings,
            counties,
            municipalities,
            data_year: _,
        } = self;

        assert!(species.is_empty());
        assert!(species_fao.is_empty());
        assert!(species_fiskeridir.is_empty());
        assert!(landings.is_empty());
        assert!(landing_entries.is_empty());
        assert!(vessels.is_empty());
        assert!(delivery_points.is_empty());
        assert!(catch_areas.is_empty());
        assert!(catch_main_areas.is_empty());
        assert!(catch_main_area_fao.is_empty());
        assert!(area_groupings.is_empty());
        assert!(counties.is_empty());
        assert!(municipalities.is_empty());
    }

    pub(crate) fn add_all(&mut self, landings: impl Iterator<Item = &'a fiskeridir_rs::Landing>) {
        for l in landings {
            self.add_vessel(l);
            self.add_species(l);
            self.add_species_fao(l);
            self.add_species_fiskeridir(l);
            self.add_delivery_point(l);
            self.add_catch_area(l);
            self.add_main_catch_area(l);
            self.add_main_catch_area_fao(l);
            self.add_fishing_region(l);
            self.add_municipality(l);
            self.add_county(l);
            self.add_landing_impl(l);
            self.add_landing_entry(l);
        }
    }

    pub(crate) fn species(&mut self) -> impl Iterator<Item = NewSpecies<'_>> {
        self.species.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fao(&mut self) -> impl Iterator<Item = NewSpeciesFao<'_>> {
        self.species_fao.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fiskeridir(&mut self) -> impl Iterator<Item = NewSpeciesFiskeridir<'_>> {
        self.species_fiskeridir.drain().map(|(_, v)| v)
    }
    pub(crate) fn vessels(&mut self) -> impl Iterator<Item = NewFiskeridirVessel<'_>> {
        self.vessels.drain().map(|(_, v)| v)
    }
    pub(crate) fn delivery_points(&mut self) -> impl Iterator<Item = NewDeliveryPointId<'_>> {
        self.delivery_points.drain()
    }
    pub(crate) fn catch_areas(&mut self) -> impl Iterator<Item = NewCatchArea> + '_ {
        self.catch_areas.drain().map(|(_, v)| v)
    }
    pub(crate) fn catch_main_areas(&mut self) -> impl Iterator<Item = NewCatchMainArea<'_>> {
        self.catch_main_areas.drain().map(|(_, v)| v)
    }
    pub(crate) fn catch_main_area_fao(&mut self) -> impl Iterator<Item = NewCatchMainAreaFao<'_>> {
        self.catch_main_area_fao.drain().map(|(_, v)| v)
    }
    pub(crate) fn area_groupings(&mut self) -> impl Iterator<Item = NewAreaGrouping<'_>> {
        self.area_groupings.drain().map(|(_, v)| v)
    }
    pub(crate) fn counties(&mut self) -> impl Iterator<Item = NewCounty<'_>> {
        self.counties.drain().map(|(_, v)| v)
    }
    pub(crate) fn municipalities(&mut self) -> impl Iterator<Item = NewMunicipality<'_>> {
        self.municipalities.drain().map(|(_, v)| v)
    }
    pub(crate) fn landing_entries(&mut self) -> impl Iterator<Item = NewLandingEntry<'_>> {
        self.landing_entries.drain(..)
    }
    pub(crate) fn landings(&mut self) -> Vec<NewLanding<'_>> {
        self.landings.drain().map(|(_, v)| v).collect()
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
