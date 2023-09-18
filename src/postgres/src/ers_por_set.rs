use crate::{error::PostgresError, models::*};
use error_stack::{report, Result, ResultExt};
use std::collections::HashMap;

#[derive(Default)]
pub struct ErsPorSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    vessels: HashMap<i64, fiskeridir_rs::Vessel>,
    ports: HashMap<String, NewPort>,
    municipalities: HashMap<i32, NewMunicipality>,
    counties: HashMap<i32, NewCounty>,
    catches: Vec<NewErsPorCatch>,
    ers_por: HashMap<i64, NewErsPor>,
}

pub struct PreparedErsPorSet {
    pub ers_message_types: Vec<NewErsMessageType>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub vessels: Vec<fiskeridir_rs::Vessel>,
    pub ports: Vec<NewPort>,
    pub municipalities: Vec<NewMunicipality>,
    pub counties: Vec<NewCounty>,
    pub catches: Vec<NewErsPorCatch>,
    pub ers_por: Vec<NewErsPor>,
}

impl ErsPorSet {
    pub(crate) fn prepare(self) -> PreparedErsPorSet {
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

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::ErsPor>>(
        ers_por: T,
    ) -> Result<ErsPorSet, PostgresError> {
        let mut set = ErsPorSet::default();

        for e in ers_por.into_iter() {
            set.add_ers_message_type(&e);
            set.add_vessel(&e)?;
            set.add_port(&e)?;
            set.add_catch(&e)?;
            set.add_municipality(&e);
            set.add_county(&e)?;
            set.add_ers_por(&e)?;
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_por: &fiskeridir_rs::ErsPor) {
        if let Some(code) = ers_por.vessel_info.vessel_municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_por.vessel_info.vessel_municipality.clone())
            });
        }
    }

    fn add_county(&mut self, ers_por: &fiskeridir_rs::ErsPor) -> Result<(), PostgresError> {
        if let Some(code) = ers_por.vessel_info.vessel_county_code {
            let county = ers_por.vessel_info.vessel_county.clone().ok_or_else(|| {
                report!(PostgresError::DataConversion)
                    .attach_printable("expected vessel_county to be Some")
            })?;
            self.counties
                .entry(code as i32)
                .or_insert_with(|| NewCounty::new(code as i32, county));
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_por: &fiskeridir_rs::ErsPor) {
        if !self
            .ers_message_types
            .contains_key(ers_por.message_info.message_type_code.as_ref())
        {
            let id = ers_por.message_info.message_type_code.clone().into_inner();
            self.ers_message_types.insert(
                id.clone(),
                NewErsMessageType::new(id, ers_por.message_info.message_type.clone().into_inner()),
            );
        }
    }

    fn add_vessel(&mut self, ers_por: &fiskeridir_rs::ErsPor) -> Result<(), PostgresError> {
        if let Some(vessel_id) = ers_por.vessel_info.vessel_id {
            if !self.vessels.contains_key(&(vessel_id as i64)) {
                let vessel = fiskeridir_rs::Vessel::try_from(ers_por.vessel_info.clone())
                    .change_context(PostgresError::DataConversion)?;
                self.vessels.entry(vessel_id as i64).or_insert(vessel);
            }
        }
        Ok(())
    }

    fn add_port(&mut self, ers_por: &fiskeridir_rs::ErsPor) -> Result<(), PostgresError> {
        if let Some(ref code) = ers_por.port.code {
            if !self.ports.contains_key(code) {
                let port = NewPort::new(code.clone(), ers_por.port.name.clone())?;
                self.ports.insert(code.clone(), port);
            }
        }
        Ok(())
    }

    fn add_catch(&mut self, ers_por: &fiskeridir_rs::ErsPor) -> Result<(), PostgresError> {
        if let Some(catch) = NewErsPorCatch::from_ers_por(ers_por) {
            let species_fao_code =
                ers_por
                    .catch
                    .species
                    .species_fao_code
                    .clone()
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected species_fao_code to be Some")
                    })?;
            self.add_species_fao(&species_fao_code, &ers_por.catch.species.species_fao);
            self.add_species_fiskeridir(
                ers_por.catch.species.species_fdir_code,
                ers_por.catch.species.species_fdir.clone(),
            );
            self.catches.push(catch);
        }
        Ok(())
    }

    fn add_species_fao(&mut self, code: &String, name: &Option<String>) {
        if !self.species_fao.contains_key(code) {
            self.species_fao
                .insert(code.clone(), SpeciesFao::new(code.clone(), name.clone()));
        }
    }

    fn add_species_fiskeridir(&mut self, code: Option<u32>, name: Option<String>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| SpeciesFiskeridir::new(code as i32, name));
        }
    }

    fn add_ers_por(&mut self, ers_por: &fiskeridir_rs::ErsPor) -> Result<(), PostgresError> {
        if !self
            .ers_por
            .contains_key(&(ers_por.message_info.message_id as i64))
        {
            let new_ers_por = NewErsPor::try_from(ers_por.clone())?;
            self.ers_por.insert(new_ers_por.message_id, new_ers_por);
        }
        Ok(())
    }
}
