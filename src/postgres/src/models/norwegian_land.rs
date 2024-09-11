use unnest_insert::UnnestInsert;

#[derive(Default, Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "norwegian_municipalities",
    conflict = "norwegian_municipality_id",
    update_coalesce_all
)]
pub struct NewMunicipality {
    #[unnest_insert(field_name = "norwegian_municipality_id")]
    pub id: i32,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "norwegian_counties", conflict = "norwegian_county_id")]
pub struct NewCounty {
    #[unnest_insert(field_name = "norwegian_county_id")]
    pub id: i32,
    pub name: String,
}

impl NewMunicipality {
    pub fn new(id: i32, name: Option<String>) -> Self {
        Self { id, name }
    }

    pub fn municipalities_from_landing(landing: &fiskeridir_rs::Landing) -> Vec<NewMunicipality> {
        let mut municipalities = Vec::with_capacity(5);
        if let Some(id) = landing.landing_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.landing_municipality.clone().map(|v| v.into_inner()),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing
                    .fisher_tax_municipality
                    .clone()
                    .map(|v| v.into_inner()),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing
                    .fisher_tax_municipality
                    .clone()
                    .map(|v| v.into_inner()),
            });
        }

        if let Some(id) = landing.production_facility_municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing.production_facility.clone().map(|v| v.into_inner()),
            });
        }

        if let Some(id) = landing.vessel.municipality_code {
            municipalities.push(NewMunicipality {
                id: id as i32,
                name: landing
                    .vessel
                    .municipality_name
                    .clone()
                    .map(|v| v.into_inner()),
            });
        }
        municipalities
    }
}

impl NewCounty {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }

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
