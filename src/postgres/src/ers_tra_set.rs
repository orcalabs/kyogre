use crate::{error::PostgresError, models::*};
use error_stack::{report, Result, ResultExt};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct ErsTraSet {
    ers_message_types: HashMap<String, NewErsMessageType>,
    species_fao: HashMap<String, SpeciesFao>,
    species_fiskeridir: HashMap<i32, SpeciesFiskeridir>,
    fiskeridir_vessels: HashMap<i64, fiskeridir_rs::Vessel>,
    ers_vessels: HashMap<String, ErsVessel>,
    vessel_identifications: HashSet<NewVesselIdentification>,
    municipalities: HashMap<i32, NewMunicipality>,
    counties: HashMap<i32, NewCounty>,
    species_groups: HashMap<i32, SpeciesGroup>,
    species_main_groups: HashMap<i32, SpeciesMainGroup>,
    catches: Vec<NewErsTraCatch>,
    ers_tra: HashMap<i64, NewErsTra>,
}

pub struct PreparedErsTraSet {
    pub ers_message_types: Vec<NewErsMessageType>,
    pub species_fao: Vec<SpeciesFao>,
    pub species_fiskeridir: Vec<SpeciesFiskeridir>,
    pub fiskeridir_vessels: Vec<fiskeridir_rs::Vessel>,
    pub ers_vessels: Vec<ErsVessel>,
    pub vessel_identifications: Vec<NewVesselIdentification>,
    pub municipalities: Vec<NewMunicipality>,
    pub counties: Vec<NewCounty>,
    pub species_groups: Vec<SpeciesGroup>,
    pub species_main_groups: Vec<SpeciesMainGroup>,
    pub catches: Vec<NewErsTraCatch>,
    pub ers_tra: Vec<NewErsTra>,
}

impl ErsTraSet {
    pub(crate) fn prepare(self) -> PreparedErsTraSet {
        let ers_message_types = self.ers_message_types.into_values().collect();
        let species_fao = self.species_fao.into_values().collect();
        let species_fiskeridir = self.species_fiskeridir.into_values().collect();
        let fiskeridir_vessels = self.fiskeridir_vessels.into_values().collect();
        let ers_vessels = self.ers_vessels.into_values().collect();
        let vessel_identifications = self.vessel_identifications.into_iter().collect();
        let municipalities = self.municipalities.into_values().collect();
        let counties = self.counties.into_values().collect();
        let species_groups = self.species_groups.into_values().collect();
        let species_main_groups = self.species_main_groups.into_values().collect();
        let ers_tra = self.ers_tra.into_values().collect();

        PreparedErsTraSet {
            ers_message_types,
            species_fao,
            species_fiskeridir,
            fiskeridir_vessels,
            ers_vessels,
            vessel_identifications,
            municipalities,
            counties,
            species_groups,
            species_main_groups,
            catches: self.catches,
            ers_tra,
        }
    }

    pub(crate) fn new<T: IntoIterator<Item = fiskeridir_rs::ErsTra>>(
        ers_tra: T,
    ) -> Result<ErsTraSet, PostgresError> {
        let mut set = ErsTraSet::default();

        for e in ers_tra.into_iter() {
            set.add_ers_message_type(&e);
            set.add_fiskeridir_vessel(&e)?;
            set.add_ers_vessel(&e);
            set.add_vessel_identification(&e);
            set.add_catch(&e)?;
            set.add_municipality(&e);
            set.add_county(&e)?;
            set.add_species_group(&e)?;
            set.add_species_main_group(&e)?;
            set.add_ers_tra(&e)?;
        }

        Ok(set)
    }

    fn add_municipality(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        if let Some(code) = ers_tra.vessel_info.vessel_municipality_code {
            self.municipalities.entry(code as i32).or_insert_with(|| {
                NewMunicipality::new(code as i32, ers_tra.vessel_info.vessel_municipality.clone())
            });
        }
    }

    fn add_county(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<(), PostgresError> {
        if let Some(code) = ers_tra.vessel_info.vessel_county_code {
            let county = ers_tra.vessel_info.vessel_county.clone().ok_or_else(|| {
                report!(PostgresError::DataConversion)
                    .attach_printable("expected vessel_county to be Some")
            })?;
            self.counties
                .entry(code as i32)
                .or_insert_with(|| NewCounty::new(code as i32, county));
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

    fn add_fiskeridir_vessel(
        &mut self,
        ers_tra: &fiskeridir_rs::ErsTra,
    ) -> Result<(), PostgresError> {
        if let Some(vessel_id) = ers_tra.vessel_info.vessel_id {
            if !self.fiskeridir_vessels.contains_key(&(vessel_id as i64)) {
                let vessel = ers_tra
                    .vessel_info
                    .clone()
                    .try_into()
                    .change_context(PostgresError::DataConversion)?;
                self.fiskeridir_vessels
                    .entry(vessel_id as i64)
                    .or_insert(vessel);
            }
        }
        Ok(())
    }

    fn add_ers_vessel(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        if !self
            .ers_vessels
            .contains_key(ers_tra.vessel_info.call_sign_ers.as_ref())
        {
            let vessel: ErsVessel = (&ers_tra.vessel_info).into();
            self.ers_vessels.insert(vessel.call_sign.clone(), vessel);
        }
    }

    fn add_vessel_identification(&mut self, ers_tra: &fiskeridir_rs::ErsTra) {
        self.vessel_identifications
            .insert((&ers_tra.vessel_info).into());
    }

    fn add_species_group(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<(), PostgresError> {
        if let Some(code) = ers_tra.catch.species.species_group_code {
            let species_group = ers_tra.catch.species.species_group.clone().ok_or_else(|| {
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
        ers_tra: &fiskeridir_rs::ErsTra,
    ) -> Result<(), PostgresError> {
        if let Some(code) = ers_tra.catch.species.species_main_group_code {
            let species_main_group = ers_tra
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

    fn add_catch(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<(), PostgresError> {
        if let Some(catch) = NewErsTraCatch::from_ers_tra(ers_tra) {
            let species_fao_code =
                ers_tra
                    .catch
                    .species
                    .species_fao_code
                    .clone()
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected species_fao_code to be Some")
                    })?;
            self.add_species_fao(&species_fao_code, &ers_tra.catch.species.species_fao);
            self.add_species_fiskeridir(
                ers_tra.catch.species.species_fdir_code,
                ers_tra.catch.species.species_fdir.clone(),
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

    fn add_ers_tra(&mut self, ers_tra: &fiskeridir_rs::ErsTra) -> Result<(), PostgresError> {
        if !self
            .ers_tra
            .contains_key(&(ers_tra.message_info.message_id as i64))
        {
            let new_ers_tra = NewErsTra::try_from(ers_tra.clone())?;
            self.ers_tra.insert(new_ers_tra.message_id, new_ers_tra);
        }
        Ok(())
    }
}
