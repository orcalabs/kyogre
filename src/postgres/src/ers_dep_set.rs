use std::collections::{HashMap, hash_map::Entry};

use kyogre_core::FiskeridirVesselId;

use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};

#[derive(Default)]
pub struct ErsDepSet<'a> {
    ers_message_types: HashMap<&'a str, NewErsMessageType<'a>>,
    species_fao: HashMap<&'a str, NewSpeciesFao<'a>>,
    species_fiskeridir: HashMap<i32, NewSpeciesFiskeridir<'a>>,
    vessels: HashMap<FiskeridirVesselId, NewFiskeridirVessel<'a>>,
    ports: HashMap<&'a str, NewPort<'a>>,
    municipalities: HashMap<i32, NewMunicipality<'a>>,
    counties: HashMap<i32, NewCounty<'a>>,
    catches: Vec<NewErsDepCatch<'a>>,
    ers_dep: HashMap<i64, NewErsDep<'a>>,
}

impl<'a> ErsDepSet<'a> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            ers_message_types: HashMap::with_capacity(capacity),
            species_fao: HashMap::with_capacity(capacity),
            species_fiskeridir: HashMap::with_capacity(capacity),
            vessels: HashMap::with_capacity(capacity),
            ports: HashMap::with_capacity(capacity),
            municipalities: HashMap::with_capacity(capacity),
            counties: HashMap::with_capacity(capacity),
            catches: Vec::with_capacity(capacity),
            ers_dep: HashMap::with_capacity(capacity),
        }
    }

    pub(crate) fn assert_is_empty(&self) {
        let Self {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            vessels,
            ports,
            municipalities,
            counties,
            catches,
            ers_dep,
        } = self;

        assert!(ers_message_types.is_empty());
        assert!(species_fao.is_empty());
        assert!(species_fiskeridir.is_empty());
        assert!(vessels.is_empty());
        assert!(ports.is_empty());
        assert!(municipalities.is_empty());
        assert!(counties.is_empty());
        assert!(catches.is_empty());
        assert!(ers_dep.is_empty());
    }

    pub(crate) fn add_all(
        &mut self,
        ers_dep: impl Iterator<Item = &'a fiskeridir_rs::ErsDep>,
    ) -> Result<()> {
        for e in ers_dep {
            self.add_ers_message_type(e);
            self.add_target_species_fao(e);
            self.add_target_species_fiskeridir(e);
            self.add_vessel(e);
            self.add_port(e)?;
            self.add_municipality(e);
            self.add_county(e)?;
            self.add_catch(e)?;
            self.add_ers_dep(e);
        }
        Ok(())
    }

    pub(crate) fn ers_message_types(&mut self) -> impl Iterator<Item = NewErsMessageType<'_>> {
        self.ers_message_types.drain().map(|(_, v)| v)
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
    pub(crate) fn municipalities(&mut self) -> impl Iterator<Item = NewMunicipality<'_>> {
        self.municipalities.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fao(&mut self) -> impl Iterator<Item = NewSpeciesFao<'_>> {
        self.species_fao.drain().map(|(_, v)| v)
    }
    pub(crate) fn species_fiskeridir(&mut self) -> impl Iterator<Item = NewSpeciesFiskeridir<'_>> {
        self.species_fiskeridir.drain().map(|(_, v)| v)
    }
    pub(crate) fn catches(&mut self) -> impl Iterator<Item = NewErsDepCatch<'_>> {
        self.catches.drain(..)
    }
    pub(crate) fn ers_dep(&mut self) -> Vec<NewErsDep<'_>> {
        self.ers_dep.drain().map(|(_, v)| v).collect()
    }

    fn add_municipality(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        if let Some(code) = ers_dep.vessel_info.municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_dep.vessel_info.municipality.as_deref())
            });
        }
    }

    fn add_county(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(code) = ers_dep.vessel_info.county_code {
            if let Entry::Vacant(e) = self.counties.entry(code as i32) {
                let county = ers_dep
                    .vessel_info
                    .county
                    .as_deref()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewCounty::new(code as i32, county));
            }
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        let id = ers_dep.message_info.message_type_code.as_ref();
        self.ers_message_types
            .entry(id)
            .or_insert_with(|| NewErsMessageType::new(id, &ers_dep.message_info.message_type));
    }

    fn add_vessel(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        if let Some(vessel_id) = ers_dep.vessel_info.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| (&ers_dep.vessel_info).into());
        }
    }

    fn add_port(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(code) = ers_dep.port.code.as_deref() {
            if let Entry::Vacant(e) = self.ports.entry(code) {
                let port = NewPort::new(code, ers_dep.port.name.as_deref())?;
                e.insert(port);
            }
        }
        Ok(())
    }

    fn add_catch(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) -> Result<()> {
        if let Some(catch) = NewErsDepCatch::from_ers_dep(ers_dep) {
            let species_fao_code = ers_dep
                .catch
                .species
                .species_fao_code
                .as_deref()
                .ok_or_else(|| MissingValueSnafu.build())?;
            self.add_species_fao_impl(
                species_fao_code,
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

    fn add_species_fao_impl(&mut self, code: &'a str, name: Option<&'a str>) {
        self.species_fao
            .entry(code)
            .or_insert_with(|| NewSpeciesFao::new(code, name));
    }

    fn add_target_species_fao(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        self.add_species_fao_impl(
            ers_dep.target_species_fao_code.as_ref(),
            ers_dep.target_species_fao.as_deref(),
        );
    }

    fn add_species_fiskeridir_impl(&mut self, code: Option<u32>, name: Option<&'a str>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| NewSpeciesFiskeridir::new(code as i32, name));
        }
    }

    fn add_target_species_fiskeridir(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        self.add_species_fiskeridir_impl(ers_dep.target_species_fdir_code, None);
    }

    fn add_ers_dep(&mut self, ers_dep: &'a fiskeridir_rs::ErsDep) {
        self.ers_dep
            .entry(ers_dep.message_info.message_id as i64)
            .or_insert_with(|| NewErsDep::from(ers_dep));
    }
}
