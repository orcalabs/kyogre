use unnest_insert::UnnestInsert;

#[derive(Default, Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "norwegian_municipalities",
    conflict = "norwegian_municipality_id",
    update_coalesce_all
)]
pub struct NewMunicipality<'a> {
    #[unnest_insert(field_name = "norwegian_municipality_id")]
    pub id: i32,
    pub name: Option<&'a str>,
}

#[derive(Default, Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "norwegian_counties", conflict = "norwegian_county_id")]
pub struct NewCounty<'a> {
    #[unnest_insert(field_name = "norwegian_county_id")]
    pub id: i32,
    pub name: &'a str,
}

impl<'a> NewMunicipality<'a> {
    pub fn new(id: i32, name: Option<&'a str>) -> Self {
        Self { id, name }
    }

    pub fn municipalities_from_landing(landing: &'a fiskeridir_rs::Landing) -> Vec<Self> {
        let mut municipalities = Vec::with_capacity(5);

        if let Some(id) = landing.landing_municipality_code {
            municipalities.push(Self {
                id: id as i32,
                name: landing.landing_municipality.as_deref(),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(Self {
                id: id as i32,
                name: landing.fisher_tax_municipality.as_deref(),
            });
        }

        if let Some(id) = landing.fisher_tax_municipality_code {
            municipalities.push(Self {
                id: id as i32,
                name: landing.fisher_tax_municipality.as_deref(),
            });
        }

        if let Some(id) = landing.production_facility_municipality_code {
            municipalities.push(Self {
                id: id as i32,
                name: landing.production_facility.as_deref(),
            });
        }

        if let Some(id) = landing.vessel.municipality_code {
            municipalities.push(Self {
                id: id as i32,
                name: landing.vessel.municipality_name.as_deref(),
            });
        }
        municipalities
    }
}

impl<'a> NewCounty<'a> {
    pub fn new(id: i32, name: &'a str) -> Self {
        Self { id, name }
    }

    pub fn counties_from_landing(landing: &'a fiskeridir_rs::Landing) -> Vec<Self> {
        let mut counties = Vec::with_capacity(2);
        if let (Some(id), Some(name)) = (landing.landing_county_code, &landing.landing_county) {
            counties.push(Self {
                id: id as i32,
                name,
            })
        }

        if let (Some(id), Some(name)) = (landing.vessel.county_code, &landing.vessel.county) {
            counties.push(Self {
                id: id as i32,
                name,
            })
        }
        counties
    }
}
