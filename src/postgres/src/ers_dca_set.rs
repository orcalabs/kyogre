use std::collections::{HashMap, hash_map::Entry};

use kyogre_core::FiskeridirVesselId;

use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};

#[derive(Default, Debug, Clone)]
pub struct ErsDcaSet<'a> {
    ers_message_types: HashMap<&'a str, NewErsMessageType<'a>>,
    area_groupings: HashMap<&'a str, NewAreaGrouping<'a>>,
    herring_populations: HashMap<&'a str, NewHerringPopulation<'a>>,
    main_areas: HashMap<i32, NewCatchMainArea<'a>>,
    catch_areas: HashMap<i32, NewCatchArea>,
    gear_fao: HashMap<&'a str, NewGearFao<'a>>,
    gear_problems: HashMap<i32, NewGearProblem<'a>>,
    vessels: HashMap<FiskeridirVesselId, NewFiskeridirVessel<'a>>,
    ports: HashMap<&'a str, NewPort<'a>>,
    species_fao: HashMap<&'a str, NewSpeciesFao<'a>>,
    species_fiskeridir: HashMap<i32, NewSpeciesFiskeridir<'a>>,
    municipalities: HashMap<i32, NewMunicipality<'a>>,
    economic_zones: HashMap<&'a str, NewEconomicZone<'a>>,
    counties: HashMap<i32, NewCounty<'a>>,
    ers_dca_bodies: Vec<NewErsDcaBody<'a>>,
    ers_dca: HashMap<i64, NewErsDca<'a>>,
}

impl<'a> ErsDcaSet<'a> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            ers_message_types: HashMap::with_capacity(capacity),
            area_groupings: HashMap::with_capacity(capacity),
            herring_populations: HashMap::with_capacity(capacity),
            main_areas: HashMap::with_capacity(capacity),
            catch_areas: HashMap::with_capacity(capacity),
            gear_fao: HashMap::with_capacity(capacity),
            gear_problems: HashMap::with_capacity(capacity),
            vessels: HashMap::with_capacity(capacity),
            ports: HashMap::with_capacity(capacity),
            species_fao: HashMap::with_capacity(capacity),
            species_fiskeridir: HashMap::with_capacity(capacity),
            municipalities: HashMap::with_capacity(capacity),
            economic_zones: HashMap::with_capacity(capacity),
            counties: HashMap::with_capacity(capacity),
            ers_dca_bodies: Vec::with_capacity(capacity),
            ers_dca: HashMap::with_capacity(capacity),
        }
    }

    pub(crate) fn assert_is_empty(&self) {
        let Self {
            ers_message_types,
            area_groupings,
            herring_populations,
            main_areas,
            catch_areas,
            gear_fao,
            gear_problems,
            vessels,
            ports,
            species_fao,
            species_fiskeridir,
            municipalities,
            economic_zones,
            counties,
            ers_dca_bodies,
            ers_dca,
        } = self;

        assert!(ers_message_types.is_empty());
        assert!(area_groupings.is_empty());
        assert!(herring_populations.is_empty());
        assert!(main_areas.is_empty());
        assert!(catch_areas.is_empty());
        assert!(gear_fao.is_empty());
        assert!(gear_problems.is_empty());
        assert!(vessels.is_empty());
        assert!(ports.is_empty());
        assert!(species_fao.is_empty());
        assert!(species_fiskeridir.is_empty());
        assert!(municipalities.is_empty());
        assert!(economic_zones.is_empty());
        assert!(counties.is_empty());
        assert!(ers_dca_bodies.is_empty());
        assert!(ers_dca.is_empty());
    }

    pub(crate) fn add_all(
        &mut self,
        ers_dca: impl Iterator<Item = &'a fiskeridir_rs::ErsDca>,
    ) -> Result<()> {
        for e in ers_dca {
            self.add_ers_message_type(e);
            self.add_area_grouping(e);
            self.add_herring_population(e)?;
            self.add_main_area(e);
            self.add_catch_area(e);
            self.add_gear_fao(e);
            self.add_gear_problem(e);
            self.add_vessel(e);
            self.add_port(e)?;
            self.add_municipality(e);
            self.add_economic_zone(e);
            self.add_county(e)?;
            self.add_species_fao(e);
            self.add_species_fiskeridir(e);
            self.add_ers_dca_body(e);
            self.add_ers_dca(e);
        }
        Ok(())
    }

    pub(crate) fn ers_message_types(&mut self) -> impl Iterator<Item = NewErsMessageType<'_>> {
        self.ers_message_types.drain().map(|(_, v)| v)
    }
    pub(crate) fn area_groupings(&mut self) -> impl Iterator<Item = NewAreaGrouping<'_>> {
        self.area_groupings.drain().map(|(_, v)| v)
    }
    pub(crate) fn herring_populations(&mut self) -> impl Iterator<Item = NewHerringPopulation<'_>> {
        self.herring_populations.drain().map(|(_, v)| v)
    }
    pub(crate) fn main_areas(&mut self) -> impl Iterator<Item = NewCatchMainArea<'_>> {
        self.main_areas.drain().map(|(_, v)| v)
    }
    pub(crate) fn catch_areas(&mut self) -> impl Iterator<Item = NewCatchArea> + '_ {
        self.catch_areas.drain().map(|(_, v)| v)
    }
    pub(crate) fn gear_fao(&mut self) -> impl Iterator<Item = NewGearFao<'_>> {
        self.gear_fao.drain().map(|(_, v)| v)
    }
    pub(crate) fn gear_problems(&mut self) -> impl Iterator<Item = NewGearProblem<'_>> {
        self.gear_problems.drain().map(|(_, v)| v)
    }
    pub(crate) fn municipalities(&mut self) -> impl Iterator<Item = NewMunicipality<'_>> {
        self.municipalities.drain().map(|(_, v)| v)
    }
    pub(crate) fn economic_zones(&mut self) -> impl Iterator<Item = NewEconomicZone<'_>> {
        self.economic_zones.drain().map(|(_, v)| v)
    }
    pub(crate) fn counties(&mut self) -> impl Iterator<Item = NewCounty<'_>> {
        self.counties.drain().map(|(_, v)| v)
    }
    pub(crate) fn vessels(&mut self) -> impl Iterator<Item = NewFiskeridirVessel<'_>> {
        self.vessels.drain().map(|(_, v)| v)
    }
    pub(crate) fn ports(&mut self) -> impl Iterator<Item = NewPort<'_>> {
        self.ports.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fao(&mut self) -> impl Iterator<Item = NewSpeciesFao<'_>> {
        self.species_fao.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fiskeridir(&mut self) -> impl Iterator<Item = NewSpeciesFiskeridir<'_>> {
        self.species_fiskeridir.drain().map(|(_, v)| v)
    }
    pub(crate) fn ers_dca_bodies(&mut self) -> impl Iterator<Item = NewErsDcaBody<'_>> {
        self.ers_dca_bodies.drain(..)
    }
    pub(crate) fn ers_dca(&mut self) -> Vec<NewErsDca<'_>> {
        self.ers_dca.drain().map(|(_, v)| v).collect()
    }

    fn add_municipality(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.vessel_info.municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_dca.vessel_info.municipality.as_deref())
            });
        }
    }

    fn add_economic_zone(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.economic_zone_code.as_deref() {
            self.economic_zones.entry(code).or_insert_with(|| {
                NewEconomicZone::new(code, ers_dca.economic_zone_code.as_deref())
            });
        }
    }

    fn add_county(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(code) = ers_dca.vessel_info.county_code {
            if let Entry::Vacant(e) = self.counties.entry(code as i32) {
                let county = ers_dca
                    .vessel_info
                    .county
                    .as_deref()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewCounty::new(code as i32, county));
            }
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        let id = ers_dca.message_info.message_type_code.as_ref();
        self.ers_message_types
            .entry(id)
            .or_insert_with(|| NewErsMessageType::new(id, &ers_dca.message_info.message_type));
    }

    fn add_herring_population(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(code) = ers_dca.herring_population_code.as_deref() {
            if let Entry::Vacant(e) = self.herring_populations.entry(code) {
                let herring_population = ers_dca
                    .herring_population
                    .as_deref()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewHerringPopulation::new(code, herring_population));
            }
        }
        Ok(())
    }

    fn add_gear_fao(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.gear.gear_fao_code.as_deref() {
            self.gear_fao
                .entry(code)
                .or_insert_with(|| NewGearFao::new(code, ers_dca.gear.gear_fao.as_deref()));
        }
    }

    fn add_gear_problem(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.gear.gear_problem_code {
            self.gear_problems.entry(code as i32).or_insert_with(|| {
                NewGearProblem::new(code as i32, ers_dca.gear.gear_fdir.as_deref())
            });
        }
    }

    fn add_vessel(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(vessel_id) = ers_dca.vessel_info.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| (&ers_dca.vessel_info).into());
        }
    }

    fn add_port(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(code) = ers_dca.port.code.as_deref() {
            if let Entry::Vacant(e) = self.ports.entry(code) {
                let port = NewPort::new(code, ers_dca.port.name.as_deref())?;
                e.insert(port);
            }
        }
        Ok(())
    }

    fn add_species_fao_impl(&mut self, code: Option<&'a str>, name: Option<&'a str>) {
        if let Some(code) = code {
            self.species_fao
                .entry(code)
                .or_insert_with(|| NewSpeciesFao::new(code, name));
        }
    }

    fn add_species_fao(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        self.add_species_fao_impl(
            ers_dca.catch.species.species_fao_code.as_deref(),
            ers_dca.catch.species.species_fao.as_deref(),
        );
        self.add_species_fao_impl(
            ers_dca.catch.majority_species_fao_code.as_deref(),
            ers_dca.catch.majority_species_fao.as_deref(),
        );
    }

    fn add_species_fiskeridir_impl(&mut self, code: Option<u32>, name: Option<&'a str>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| NewSpeciesFiskeridir::new(code as i32, name));
        }
    }

    fn add_species_fiskeridir(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        self.add_species_fiskeridir_impl(
            ers_dca.catch.species.species_fdir_code,
            ers_dca.catch.species.species_fdir.as_deref(),
        );
        self.add_species_fiskeridir_impl(ers_dca.catch.majority_species_fdir_code, None);
    }

    fn add_area_grouping_impl(&mut self, code: &'a str, name: Option<&'a str>) {
        self.area_groupings
            .entry(code)
            .or_insert_with(|| NewAreaGrouping::new(code, name));
    }

    fn add_area_grouping(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        if let Some(ref code) = ers_dca.area_grouping_end_code {
            self.add_area_grouping_impl(code.as_ref(), ers_dca.area_grouping_end.as_deref())
        }

        if let Some(ref code) = ers_dca.area_grouping_start_code {
            self.add_area_grouping_impl(code.as_ref(), ers_dca.area_grouping_start.as_deref())
        }
    }

    fn add_main_area_impl(&mut self, code: Option<u32>, name: Option<&'a str>) {
        if let Some(code) = code {
            self.main_areas
                .entry(code as i32)
                .or_insert_with(|| NewCatchMainArea {
                    id: code as i32,
                    name,
                    longitude: None,
                    latitude: None,
                });
        }
    }

    fn add_catch_area_impl(&mut self, code: Option<u32>) {
        if let Some(code) = code {
            self.catch_areas
                .entry(code as i32)
                .or_insert_with(|| NewCatchArea {
                    id: code as i32,
                    longitude: None,
                    latitude: None,
                });
        }
    }

    fn add_main_area(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        self.add_main_area_impl(ers_dca.main_area_end_code, ers_dca.main_area_end.as_deref());
        self.add_main_area_impl(
            ers_dca.main_area_start_code,
            ers_dca.main_area_start.as_deref(),
        );
    }

    fn add_catch_area(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        self.add_catch_area_impl(ers_dca.location_start_code);
        self.add_catch_area_impl(ers_dca.location_end_code);
    }

    fn add_ers_dca_body(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        self.ers_dca_bodies.push(ers_dca.into());
    }

    fn add_ers_dca(&mut self, ers_dca: &'a fiskeridir_rs::ErsDca) {
        let new = NewErsDca::from(ers_dca);
        match self.ers_dca.entry(new.message_id) {
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                if new.message_version > v.message_version {
                    *v = new;
                }
            }
            Entry::Vacant(e) => {
                e.insert(new);
            }
        }
    }
}
