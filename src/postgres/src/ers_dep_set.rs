use crate::{error::PostgresError, models::*};
use error_stack::{report, Result, ResultExt};
use std::collections::HashMap;

#[derive(Default)]
pub struct ErsDepSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    vessels: HashMap<i64, fiskeridir_rs::Vessel>,
    ports: HashMap<String, NewPort>,
    municipalities: HashMap<i32, NewMunicipality>,
    counties: HashMap<i32, NewCounty>,
    species_groups: HashMap<i32, SpeciesGroup>,
    species_main_groups: HashMap<i32, SpeciesMainGroup>,
    catches: Vec<NewErsDepCatch>,
    ers_dep: HashMap<i64, NewErsDep>,
}

pub struct PreparedErsDepSet {
    pub ers_message_types: Vec<NewErsMessageType>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub vessels: Vec<fiskeridir_rs::Vessel>,
    pub ports: Vec<NewPort>,
    pub municipalities: Vec<NewMunicipality>,
    pub counties: Vec<NewCounty>,
    pub species_groups: Vec<SpeciesGroup>,
    pub species_main_groups: Vec<SpeciesMainGroup>,
    pub catches: Vec<NewErsDepCatch>,
    pub ers_dep: Vec<NewErsDep>,
}

impl ErsDepSet {
    pub(crate) fn prepare(self) -> PreparedErsDepSet {
        let ers_message_types = self.ers_message_types.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let ports = self.ports.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let counties = self.counties.into_values().collect();
        let species_groups = self.species_groups.into_values().collect();
        let species_main_groups = self.species_main_groups.into_values().collect();
        let ers_dep = self.ers_dep.into_values().collect();

        PreparedErsDepSet {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            vessels,
            ports,
            municipalities,
            counties,
            species_groups,
            species_main_groups,
            catches: self.catches,
            ers_dep,
        }
    }

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::ErsDep>>(
        ers_dep: T,
    ) -> Result<ErsDepSet, PostgresError> {
        let mut set = ErsDepSet::default();

        for e in ers_dep.into_iter() {
            set.add_ers_message_type(&e);
            set.add_target_species_fao(&e);
            set.add_target_species_fiskeridir(&e);
            set.add_vessel(&e)?;
            set.add_port(&e)?;
            set.add_municipality(&e);
            set.add_county(&e)?;
            set.add_species_group(&e)?;
            set.add_species_main_group(&e)?;
            set.add_catch(&e)?;
            set.add_ers_dep(&e)?;
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        if let Some(code) = ers_dep.vessel_info.vessel_municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_dep.vessel_info.vessel_municipality.clone())
            });
        }
    }

    fn add_county(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if let Some(code) = ers_dep.vessel_info.vessel_county_code {
            let county = ers_dep.vessel_info.vessel_county.clone().ok_or_else(|| {
                report!(PostgresError::DataConversion)
                    .attach_printable("expected vessel_county to be Some")
            })?;
            self.counties
                .entry(code as i32)
                .or_insert_with(|| NewCounty::new(code as i32, county));
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        if !self
            .ers_message_types
            .contains_key(ers_dep.message_info.message_type_code.as_ref())
        {
            let id = ers_dep.message_info.message_type_code.clone().into_inner();
            self.ers_message_types.insert(
                id.clone(),
                NewErsMessageType::new(id, ers_dep.message_info.message_type.clone().into_inner()),
            );
        }
    }

    fn add_vessel(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if let Some(vessel_id) = ers_dep.vessel_info.vessel_id {
            if !self.vessels.contains_key(&(vessel_id as i64)) {
                let vessel = ers_dep
                    .vessel_info
                    .clone()
                    .try_into()
                    .change_context(PostgresError::DataConversion)?;
                self.vessels.entry(vessel_id as i64).or_insert(vessel);
            }
        }
        Ok(())
    }

    fn add_port(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if let Some(ref code) = ers_dep.port.code {
            if !self.ports.contains_key(code) {
                let port = NewPort::new(code.clone(), ers_dep.port.name.clone())?;
                self.ports.insert(code.clone(), port);
            }
        }
        Ok(())
    }

    fn add_species_group(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if let Some(code) = ers_dep.catch.species.species_group_code {
            let species_group = ers_dep.catch.species.species_group.clone().ok_or_else(|| {
                report!(PostgresError::DataConversion)
                    .attach_printable("expected species_group to be Some")
            })?;
            self.species_groups
                .entry(code as i32)
                .or_insert_with(|| SpeciesGroup::new(code as i32, species_group));
        }
        Ok(())
    }

    fn add_species_main_group(
        &mut self,
        ers_dep: &fiskeridir_rs::ErsDep,
    ) -> Result<(), PostgresError> {
        if let Some(code) = ers_dep.catch.species.species_main_group_code {
            let species_main_group = ers_dep
                .catch
                .species
                .species_main_group
                .clone()
                .ok_or_else(|| {
                    report!(PostgresError::DataConversion)
                        .attach_printable("expected species_main_group to be Some")
                })?;
            self.species_main_groups
                .entry(code as i32)
                .or_insert_with(|| SpeciesMainGroup::new(code as i32, species_main_group));
        }
        Ok(())
    }

    fn add_catch(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if let Some(catch) = NewErsDepCatch::from_ers_dep(ers_dep) {
            let species_fao_code =
                ers_dep
                    .catch
                    .species
                    .species_fao_code
                    .clone()
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected species_fao_code to be Some")
                    })?;
            self.add_species_fao_impl(&species_fao_code, &ers_dep.catch.species.species_fao);
            self.add_species_fiskeridir_impl(
                ers_dep.catch.species.species_fdir_code,
                ers_dep.catch.species.species_fdir.clone(),
            );
            self.catches.push(catch);
        }
        Ok(())
    }

    fn add_species_fao_impl(&mut self, code: &String, name: &Option<String>) {
        if !self.species_fao.contains_key(code) {
            self.species_fao
                .insert(code.clone(), SpeciesFao::new(code.clone(), name.clone()));
        }
    }

    fn add_target_species_fao(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        self.add_species_fao_impl(
            &ers_dep.target_species_fao_code.clone().into_inner(),
            &ers_dep.target_species_fao.clone(),
        );
    }

    fn add_species_fiskeridir_impl(&mut self, code: Option<u32>, name: Option<String>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| SpeciesFiskeridir::new(code as i32, name));
        }
    }

    fn add_target_species_fiskeridir(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        self.add_species_fiskeridir_impl(ers_dep.target_species_fdir_code, None);
    }

    fn add_ers_dep(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<(), PostgresError> {
        if !self
            .ers_dep
            .contains_key(&(ers_dep.message_info.message_id as i64))
        {
            let new_ers_dep = NewErsDep::try_from(ers_dep.clone())?;
            self.ers_dep.insert(new_ers_dep.message_id, new_ers_dep);
        }
        Ok(())
    }
}