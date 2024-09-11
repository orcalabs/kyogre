use std::collections::{hash_map::Entry, HashMap};

use kyogre_core::FiskeridirVesselId;

use crate::{
    error::{MissingValueSnafu, Result},
    models::*,
};

#[derive(Default)]
pub struct ErsTraSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    vessels: HashMap<FiskeridirVesselId, fiskeridir_rs::Vessel>,
    municipalities: HashMap<i32, NewMunicipality>,
    counties: HashMap<i32, NewCounty>,
    catches: Vec<NewErsTraCatch>,
    ers_tra: HashMap<i64, NewErsTra>,
}

pub struct PreparedErsTraSet {
    pub ers_message_types: Vec<NewErsMessageType>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub vessels: Vec<fiskeridir_rs::Vessel>,
    pub municipalities: Vec<NewMunicipality>,
    pub counties: Vec<NewCounty>,
    pub catches: Vec<NewErsTraCatch>,
    pub ers_tra: Vec<NewErsTra>,
}

impl ErsTraSet {
    pub(crate) fn prepare(self) -> PreparedErsTraSet {
        let ers_message_types = self.ers_message_types.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let vessels = self.vessels.into_values().collect();
        let municipalities = self.municipalities.into_values().collect();
        let counties = self.counties.into_values().collect();
        let ers_tra = self.ers_tra.into_values().collect();

        PreparedErsTraSet {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            vessels,
            municipalities,
            counties,
            catches: self.catches,
            ers_tra,
        }
    }

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::ErsTra>>(
        ers_tra: T,
    ) -> Result<ErsTraSet> {
        let mut set = ErsTraSet::default();

        for e in ers_tra.into_iter() {
            set.add_ers_message_type(&e);
            set.add_vessel(&e);
            set.add_catch(&e)?;
            set.add_municipality(&e);
            set.add_county(&e)?;
            set.add_ers_tra(e);
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        if let Some(code) = ers_tra.vessel_info.municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(
                    code as i32,
                    ers_tra
                        .vessel_info
                        .municipality
                        .clone()
                        .map(|v| v.into_inner()),
                )
            });
        }
    }

    fn add_county(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<()> {
        if let Some(code) = ers_tra.vessel_info.county_code {
            if let Entry::Vacant(e) = self.counties.entry(code as i32) {
                let county = ers_tra
                    .vessel_info
                    .county
                    .clone()
                    .ok_or_else(|| MissingValueSnafu.build())?;
                e.insert(NewCounty::new(code as i32, county.into_inner()));
            }
        }
        Ok(())
    }

    fn add_ers_message_type(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        if !self
            .ers_message_types
            .contains_key(ers_tra.message_info.message_type_code.as_ref())
        {
            let id = ers_tra.message_info.message_type_code.clone().into_inner();
            self.ers_message_types.insert(
                id.clone(),
                NewErsMessageType::new(id, ers_tra.message_info.message_type.clone().into_inner()),
            );
        }
    }

    fn add_vessel(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        if let Some(vessel_id) = ers_tra.vessel_info.id {
            self.vessels
                .entry(vessel_id)
                .or_insert_with(|| ers_tra.vessel_info.clone().into());
        }
    }

    fn add_catch(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<()> {
        if let Some(catch) = NewErsTraCatch::from_ers_tra(ers_tra) {
            let species_fao_code = ers_tra
                .catch
                .species
                .species_fao_code
                .clone()
                .ok_or_else(|| MissingValueSnafu.build())?;
            self.add_species_fao(
                species_fao_code.as_ref(),
                ers_tra.catch.species.species_fao.as_deref(),
            );
            self.add_species_fiskeridir(
                ers_tra.catch.species.species_fdir_code,
                ers_tra.catch.species.species_fdir.as_deref(),
            );
            self.catches.push(catch);
        }
        Ok(())
    }

    fn add_species_fao(&mut self, code: &str, name: Option<&str>) {
        if !self.species_fao.contains_key(code) {
            self.species_fao.insert(
                code.into(),
                SpeciesFao::new(code.into(), name.map(From::from)),
            );
        }
    }

    fn add_species_fiskeridir(&mut self, code: Option<u32>, name: Option<&str>) {
        if let Some(code) = code {
            self.species_fiskeridir
                .entry(code as i32)
                .or_insert_with(|| SpeciesFiskeridir::new(code as i32, name.map(From::from)));
        }
    }

    fn add_ers_tra(&mut self, ers_tra: fiskeridir_rs::ErsTra) {
        self.ers_tra
            .entry(ers_tra.message_info.message_id as i64)
            .or_insert_with(|| ers_tra.into());
    }
}
