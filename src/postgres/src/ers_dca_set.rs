use std::collections::{hash_map::Entry, HashMap};

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

pub struct PreparedErsDcaSet<'a> {
    pub ers_message_types: Vec<NewErsMessageType<'a>>,
    pub area_groupings: Vec<NewAreaGrouping<'a>>,
    pub herring_populations: Vec<NewHerringPopulation<'a>>,
    pub main_areas: Vec<NewCatchMainArea<'a>>,
    pub catch_areas: Vec<NewCatchArea>,
    pub gear_fao: Vec<NewGearFao<'a>>,
    pub gear_problems: Vec<NewGearProblem<'a>>,
    pub vessels: Vec<NewFiskeridirVessel<'a>>,
    pub ports: Vec<NewPort<'a>>,
    pub species_fao: Vec<NewSpeciesFao<'a>>,
    pub species_fiskeridir: Vec<NewSpeciesFiskeridir<'a>>,
    pub municipalities: Vec<NewMunicipality<'a>>,
    pub economic_zones: Vec<NewEconomicZone<'a>>,
    pub counties: Vec<NewCounty<'a>>,
    pub ers_dca_bodies: Vec<NewErsDcaBody<'a>>,
    pub ers_dca: Vec<NewErsDca<'a>>,
}

impl<'a> ErsDcaSet<'a> {
    pub(crate) fn prepare(self) -> PreparedErsDcaSet<'a> {
        let ers_message_types = self.ers_message_types.into_values().collect();
        let area_groupings = self.area_groupings.into_values().collect();
        let herring_populations = self.herring_populations.into_values().collect();
        let main_areas = self.main_areas.into_values().collect();
        let catch_areas = self.catch_areas.into_values().collect();
        let gear_fao = self.gear_fao.into_values().collect();
        let gear_problems = self.gear_problems.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let economic_zones = self.economic_zones.into_values().collect();
        let counties = self.counties.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let ports = self.ports.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let ers_dca = self.ers_dca.into_values().collect();

        PreparedErsDcaSet {
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
            ers_dca_bodies: self.ers_dca_bodies,
            ers_dca,
        }
    }

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

    pub(crate) fn new(ers_dca: impl Iterator<Item = &'a fiskeridir_rs::ErsDca>) -> Result<Self> {
        let (min, max) = ers_dca.size_hint();
        let mut set = Self::with_capacity(max.unwrap_or(min));

        for e in ers_dca {
            set.add_ers_message_type(e);
            set.add_area_grouping(e);
            set.add_herring_population(e)?;
            set.add_main_area(e);
            set.add_catch_area(e);
            set.add_gear_fao(e);
            set.add_gear_problem(e);
            set.add_vessel(e);
            set.add_port(e)?;
            set.add_municipality(e);
            set.add_economic_zone(e);
            set.add_county(e)?;
            set.add_species_fao(e);
            set.add_species_fiskeridir(e);
            set.add_ers_dca_body(e);
            set.add_ers_dca(e);
        }

        Ok(set)
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
