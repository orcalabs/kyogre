pub struct NewMunicipality {
    pub id: i32,
    pub name: Option<String>,
}

pub struct NewCounty {
    pub id: i32,
    pub name: String,
}

impl NewMunicipality {
    pub fn municipalities_from_landing(landing: &fiskeridir_rs::Landing) -> Vec<NewMunicipality> {
        let mut municipalities = Vec::with_capacity(5);
        if let Some(id) = landing.landing_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.landing_municipality.clone(),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.fisher_tax_municipality.clone(),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.fisher_tax_municipality.clone(),
            });
        }

        if let Some(id) = landing.production_facility_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.production_facility.clone(),
            });
        }

        if let Some(id) = landing.vessel.municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.vessel.municipality_name.clone(),
            });
        }
        municipalities
    }
}

impl NewCounty {
    pub fn counties_from_landing(landing: &fiskeridir_rs::Landing) -> Vec<NewCounty> {
        let mut counties = Vec::with_capacity(2);
        if let (Some(id), Some(name)) = (landing.landing_county_code, &landing.landing_county) {
            counties.push(NewCounty {
                id: id as i32,
                name: name.to_string(),
            })
        }

        if let (Some(id), Some(name)) = (landing.vessel.county_code, &landing.vessel.county) {
            counties.push(NewCounty {
                id: id as i32,
                name: name.to_string(),
            })
        }
        counties
    }
}
