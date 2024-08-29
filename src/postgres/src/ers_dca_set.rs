use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};
use std::collections::{hash_map::Entry, HashMap};

#[derive(Default, Debug, Clone)]
pub struct ErsDcaSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    area_groupings: HashMap<String, NewAreaGrouping>,
    herring_populations: HashMap<String, NewHerringPopulation>,
    main_areas: HashMap<i32, NewCatchMainArea>,
    catch_areas: HashMap<i32, NewCatchArea>,
    gear_fao: HashMap<String, NewGearFao>,
    gear_problems: HashMap<i32, NewGearProblem>,
    vessels: HashMap<i64, fiskeridir_rs::Vessel>,
    ports: HashMap<String, NewPort>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    municipalities: HashMap<i32, NewMunicipality>,
    economic_zones: HashMap<String, NewEconomicZone>,
    counties: HashMap<i32, NewCounty>,
    ers_dca_bodies: Vec<NewErsDcaBody>,
    ers_dca: HashMap<i64, NewErsDca>,
}

pub struct PreparedErsDcaSet {
    pub ers_message_types: Vec<NewErsMessageType>,
    pub area_groupings: Vec<NewAreaGrouping>,
    pub herring_populations: Vec<NewHerringPopulation>,
    pub main_areas: Vec<NewCatchMainArea>,
    pub catch_areas: Vec<NewCatchArea>,
    pub gear_fao: Vec<NewGearFao>,
    pub gear_problems: Vec<NewGearProblem>,
    pub vessels: Vec<fiskeridir_rs::Vessel>,
    pub ports: Vec<NewPort>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub municipalities: Vec<NewMunicipality>,
    pub economic_zones: Vec<NewEconomicZone>,
    pub counties: Vec<NewCounty>,
    pub ers_dca_bodies: Vec<NewErsDcaBody>,
    pub ers_dca: Vec<NewErsDca>,
}

impl ErsDcaSet {
    pub(crate) fn prepare(self) -> PreparedErsDcaSet {
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

    pub(crate) fn new<T: Iterator<Item = fiskeridir_rs::ErsDca>>(ers_dca: T) -> Result<ErsDcaSet> {
        let mut set = ErsDcaSet::default();

        for e in ers_dca {
            set.add_ers_message_type(&e);
            set.add_area_grouping(&e);
            set.add_herring_population(&e)?;
            set.add_main_area(&e);
            set.add_catch_area(&e);
            set.add_gear_fao(&e);
            set.add_gear_problem(&e);
            set.add_vessel(&e)?;
            set.add_port(&e)?;
            set.add_municipality(&e);
            set.add_economic_zone(&e);
            set.add_county(&e)?;
            set.add_species_fao(&e);
            set.add_species_fiskeridir(&e);
            set.add_ers_dca_body(&e)?;
            set.add_ers_dca(&e)?;
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.vessel_info.vessel_municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_dca.vessel_info.vessel_municipality.clone())
            });
        }
    }

    fn add_economic_zone(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if let Some(ref code) = ers_dca.economic_zone_code {
            if !self.economic_zones.contains_key(code) {
                self.economic_zones.insert(
                    code.clone(),
                    NewEconomicZone::new(code.clone(), ers_dca.economic_zone_code.clone()),
                );
            }
        }
    }

    fn add_county(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(code) = ers_dca.vessel_info.vessel_county_code {
            let county = ers_dca
                .vessel_info
                .vessel_county
                .clone()
                .ok_or_else(|| MissingValueSnafu.build())?;
            self.counties
                .entry(code as i32)
                .or_insert_with(|| NewCounty::new(code as i32, county));
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if !self
            .ers_message_types
            .contains_key(ers_dca.message_info.message_type_code.as_ref())
        {
            let id = ers_dca.message_info.message_type_code.clone().into_inner();
            self.ers_message_types.insert(
                id.clone(),
                NewErsMessageType::new(id, ers_dca.message_info.message_type.clone().into_inner()),
            );
        }
    }

    fn add_herring_population(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(ref code) = ers_dca.herring_population_code {
            if !self.herring_populations.contains_key(code) {
                let herring_population = ers_dca
                    .herring_population
                    .clone()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                self.herring_populations.insert(
                    code.clone(),
                    NewHerringPopulation::new(code.clone(), herring_population),
                );
            }
        }
        Ok(())
    }

    fn add_gear_fao(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if let Some(ref code) = ers_dca.gear.gear_fao_code {
            if !self.gear_fao.contains_key(code) {
                self.gear_fao.insert(
                    code.clone(),
                    NewGearFao::new(code.clone(), ers_dca.gear.gear_fao.clone()),
                );
            }
        }
    }

    fn add_gear_problem(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if let Some(code) = ers_dca.gear.gear_problem_code {
            self.gear_problems.entry(code as i32).or_insert_with(|| {
                NewGearProblem::new(code as i32, ers_dca.gear.gear_fdir.clone())
            });
        }
    }

    fn add_vessel(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(vessel_id) = ers_dca.vessel_info.vessel_id {
            if let Entry::Vacant(e) = self.vessels.entry(vessel_id as i64) {
                let vessel = fiskeridir_rs::Vessel::try_from(ers_dca.vessel_info.clone())?;
                e.insert(vessel);
            }
        }
        Ok(())
    }

    fn add_port(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        if let Some(ref code) = ers_dca.port.code {
            if !self.ports.contains_key(code) {
                let port = NewPort::new(code.clone(), ers_dca.port.name.clone())?;
                self.ports.insert(code.clone(), port);
            }
        }
        Ok(())
    }

    fn add_species_fao_impl(&mut self, code: &Option<String>, name: &Option<String>) {
        if let Some(code) = code {
            if !self.species_fao.contains_key(code) {
                self.species_fao
                    .insert(code.clone(), SpeciesFao::new(code.clone(), name.clone()));
            }
        }
    }

    fn add_species_fao(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        self.add_species_fao_impl(
            &ers_dca.catch.species.species_fao_code,
            &ers_dca.catch.species.species_fao,
        );
        self.add_species_fao_impl(
            &ers_dca.catch.majority_species_fao_code,
            &ers_dca.catch.majority_species_fao,
        );
    }

    fn add_species_fiskeridir_impl(&mut self, code: Option<u32>, name: &Option<String>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| SpeciesFiskeridir::new(code as i32, name.clone()));
        }
    }

    fn add_species_fiskeridir(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        self.add_species_fiskeridir_impl(
            ers_dca.catch.species.species_fdir_code,
            &ers_dca.catch.species.species_fdir,
        );
        self.add_species_fiskeridir_impl(ers_dca.catch.majority_species_fdir_code, &None);
    }

    fn add_area_grouping_impl(&mut self, code: &String, name: &Option<String>) {
        if !self.area_groupings.contains_key(code) {
            self.area_groupings.insert(
                code.clone(),
                NewAreaGrouping::new(code.clone(), name.clone()),
            );
        }
    }

    fn add_area_grouping(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        if let Some(ref code) = ers_dca.area_grouping_end_code {
            self.add_area_grouping_impl(code, &ers_dca.area_grouping_end)
        }

        if let Some(ref code) = ers_dca.area_grouping_start_code {
            self.add_area_grouping_impl(code, &ers_dca.area_grouping_start)
        }
    }

    fn add_main_area_impl(&mut self, code: Option<u32>, name: Option<String>) {
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

    fn add_main_area(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        self.add_main_area_impl(ers_dca.main_area_end_code, ers_dca.main_area_end.clone());
        self.add_main_area_impl(
            ers_dca.main_area_start_code,
            ers_dca.main_area_start.clone(),
        );
    }

    fn add_catch_area(&mut self, ers_dca: &fiskeridir_rs::ErsDca) {
        self.add_catch_area_impl(ers_dca.location_start_code);
        self.add_catch_area_impl(ers_dca.location_end_code);
    }

    fn add_ers_dca_body(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        self.ers_dca_bodies.push(ers_dca.try_into()?);
        Ok(())
    }

    fn add_ers_dca(&mut self, ers_dca: &fiskeridir_rs::ErsDca) -> Result<()> {
        let new = NewErsDca::try_from(ers_dca.clone())?;
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
        Ok(())
    }
}
