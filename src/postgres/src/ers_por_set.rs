use std::collections::{hash_map::Entry, HashMap};

use kyogre_core::FiskeridirVesselId;

use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};

#[derive(Default)]
pub struct ErsPorSet<'a> {
    ers_message_types: HashMap<&'a str, NewErsMessageType<'a>>,
    species_fao: HashMap<&'a str, NewSpeciesFao<'a>>,
    species_fiskeridir: HashMap<i32, NewSpeciesFiskeridir<'a>>,
    vessels: HashMap<FiskeridirVesselId, NewFiskeridirVessel<'a>>,
    ports: HashMap<&'a str, NewPort<'a>>,
    municipalities: HashMap<i32, NewMunicipality<'a>>,
    counties: HashMap<i32, NewCounty<'a>>,
    catches: Vec<NewErsPorCatch<'a>>,
    ers_por: HashMap<i64, NewErsPor<'a>>,
}

pub struct PreparedErsPorSet<'a> {
    pub ers_message_types: Vec<NewErsMessageType<'a>>,
    pub species_fao: Vec<NewSpeciesFao<'a>>,
    pub species_fiskeridir: Vec<NewSpeciesFiskeridir<'a>>,
    pub vessels: Vec<NewFiskeridirVessel<'a>>,
    pub ports: Vec<NewPort<'a>>,
    pub municipalities: Vec<NewMunicipality<'a>>,
    pub counties: Vec<NewCounty<'a>>,
    pub catches: Vec<NewErsPorCatch<'a>>,
    pub ers_por: Vec<NewErsPor<'a>>,
}

impl<'a> ErsPorSet<'a> {
    pub(crate) fn prepare(self) -> PreparedErsPorSet<'a> {
        let ers_message_types = self.ers_message_types.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let ports = self.ports.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let counties = self.counties.into_values().collect();
        let ers_por = self.ers_por.into_values().collect();

        PreparedErsPorSet {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            vessels,
            ports,
            municipalities,
            counties,
            catches: self.catches,
            ers_por,
        }
    }

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
            ers_por: HashMap::with_capacity(capacity),
        }
    }

    pub(crate) fn new(ers_por: impl Iterator<Item = &'a fiskeridir_rs::ErsPor>) -> Result<Self> {
        let (min, max) = ers_por.size_hint();
        let mut set = Self::with_capacity(max.unwrap_or(min));

        for e in ers_por {
            set.add_ers_message_type(e);
            set.add_vessel(e);
            set.add_port(e)?;
            set.add_catch(e)?;
            set.add_municipality(e);
            set.add_county(e)?;
            set.add_ers_por(e);
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) {
        if let Some(code) = ers_por.vessel_info.municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_por.vessel_info.municipality.as_deref())
            });
        }
    }

    fn add_county(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) -> Result<()> {
        if let Some(code) = ers_por.vessel_info.county_code {
            if let Entry::Vacant(e) = self.counties.entry(code as i32) {
                let county = ers_por
                    .vessel_info
                    .county
                    .as_deref()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewCounty::new(code as i32, county));
            }
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) {
        let id = ers_por.message_info.message_type_code.as_ref();
        self.ers_message_types
            .entry(id)
            .or_insert_with(|| NewErsMessageType::new(id, &ers_por.message_info.message_type));
    }

    fn add_vessel(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) {
        if let Some(vessel_id) = ers_por.vessel_info.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| (&ers_por.vessel_info).into());
        }
    }

    fn add_port(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) -> Result<()> {
        if let Some(code) = ers_por.port.code.as_deref() {
            if let Entry::Vacant(e) = self.ports.entry(code) {
                let port = NewPort::new(code, ers_por.port.name.as_deref())?;
                e.insert(port);
            }
        }
        Ok(())
    }

    fn add_catch(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) -> Result<()> {
        if let Some(catch) = NewErsPorCatch::from_ers_por(ers_por) {
            let species_fao_code = ers_por
                .catch
                .species
                .species_fao_code
                .as_deref()
                .ok_or_else(|| MissingValueSnafu {}.build())?;
            self.add_species_fao(
                species_fao_code,
                ers_por.catch.species.species_fao.as_deref(),
            );
            self.add_species_fiskeridir(
                ers_por.catch.species.species_fdir_code,
                ers_por.catch.species.species_fdir.as_deref(),
            );
            self.catches.push(catch);
        }
        Ok(())
    }

    fn add_species_fao(&mut self, code: &'a str, name: Option<&'a str>) {
        self.species_fao
            .entry(code)
            .or_insert_with(|| NewSpeciesFao::new(code, name));
    }

    fn add_species_fiskeridir(&mut self, code: Option<u32>, name: Option<&'a str>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| NewSpeciesFiskeridir::new(code as i32, name));
        }
    }

    fn add_ers_por(&mut self, ers_por: &'a fiskeridir_rs::ErsPor) {
        self.ers_por
            .entry(ers_por.message_info.message_id as i64)
            .or_insert_with(|| ers_por.into());
    }
}
