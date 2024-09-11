use std::collections::{hash_map::Entry, HashMap};

use kyogre_core::FiskeridirVesselId;

use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};

#[derive(Default)]
pub struct ErsDepSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    vessels: HashMap<FiskeridirVesselId, fiskeridir_rs::Vessel>,
    ports: HashMap<String, NewPort>,
    municipalities: HashMap<i32, NewMunicipality>,
    counties: HashMap<i32, NewCounty>,
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
        let ers_dep = self.ers_dep.into_values().collect();

        PreparedErsDepSet {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            vessels,
            ports,
            municipalities,
            counties,
            catches: self.catches,
            ers_dep,
        }
    }

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::ErsDep>>(
        ers_dep: T,
    ) -> Result<ErsDepSet> {
        let mut set = ErsDepSet::default();

        for e in ers_dep.into_iter() {
            set.add_ers_message_type(&e);
            set.add_target_species_fao(&e);
            set.add_target_species_fiskeridir(&e);
            set.add_vessel(&e);
            set.add_port(&e)?;
            set.add_municipality(&e);
            set.add_county(&e)?;
            set.add_catch(&e)?;
            set.add_ers_dep(e);
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        if let Some(code) = ers_dep.vessel_info.municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(
                    code as i32,
                    ers_dep
                        .vessel_info
                        .municipality
                        .clone()
                        .map(|v| v.into_inner()),
                )
            });
        }
    }

    fn add_county(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(code) = ers_dep.vessel_info.county_code {
            if let Entry::Vacant(e) = self.counties.entry(code as i32) {
                let county = ers_dep
                    .vessel_info
                    .county
                    .clone()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewCounty::new(code as i32, county.into_inner()));
            }
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

    fn add_vessel(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        if let Some(vessel_id) = ers_dep.vessel_info.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| ers_dep.vessel_info.clone().into());
        }
    }

    fn add_port(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(ref code) = ers_dep.port.code {
            let code = code.as_ref();
            if !self.ports.contains_key(code) {
                let port = NewPort::new(
                    code.into(),
                    ers_dep.port.name.clone().map(|v| v.into_inner()),
                )?;
                self.ports.insert(code.into(), port);
            }
        }
        Ok(())
    }

    fn add_catch(&mut self, ers_dep: &fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(catch) = NewErsDepCatch::from_ers_dep(ers_dep) {
            let species_fao_code = ers_dep
                .catch
                .species
                .species_fao_code
                .clone()
                .ok_or_else(|| MissingValueSnafu.build())?;
            self.add_species_fao_impl(
                species_fao_code.as_ref(),
                ers_dep.catch.species.species_fao.as_deref(),
            );
            self.add_species_fiskeridir_impl(
                ers_dep.catch.species.species_fdir_code,
                ers_dep.catch.species.species_fdir.as_deref(),
            );
            self.catches.push(catch);
        }
        Ok(())
    }

    fn add_species_fao_impl(&mut self, code: &str, name: Option<&str>) {
        if !self.species_fao.contains_key(code) {
            self.species_fao.insert(
                code.into(),
                SpeciesFao::new(code.into(), name.map(From::from)),
            );
        }
    }

    fn add_target_species_fao(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        self.add_species_fao_impl(
            ers_dep.target_species_fao_code.as_ref(),
            ers_dep.target_species_fao.as_deref(),
        );
    }

    fn add_species_fiskeridir_impl(&mut self, code: Option<u32>, name: Option<&str>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| SpeciesFiskeridir::new(code as i32, name.map(From::from)));
        }
    }

    fn add_target_species_fiskeridir(&mut self, ers_dep: &fiskeridir_rs::ErsDep) {
        self.add_species_fiskeridir_impl(ers_dep.target_species_fdir_code, None);
    }

    fn add_ers_dep(&mut self, ers_dep: fiskeridir_rs::ErsDep) {
        self.ers_dep
            .entry(ers_dep.message_info.message_id as i64)
            .or_insert_with(|| NewErsDep::from(ers_dep));
    }
}
