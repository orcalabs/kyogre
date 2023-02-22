#[derive(Debug, Clone, PartialEq)]
pub struct NewCatchArea {
    pub id: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewCatchMainArea {
    pub id: i32,
    pub name: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAreaGrouping {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewCatchMainAreaFao {
    pub id: i32,
    pub name: Option<String>,
}

impl From<fiskeridir_rs::CatchLocation> for NewCatchMainArea {
    fn from(value: fiskeridir_rs::CatchLocation) -> Self {
        NewCatchMainArea {
            id: value.main_area_code as i32,
            name: value.main_area,
            longitude: value.main_area_longitude,
            latitude: value.main_area_latitude,
        }
    }
}

impl NewAreaGrouping {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<NewAreaGrouping> {
        landing
            .catch_location
            .area_grouping_code
            .as_ref()
            .map(|id| NewAreaGrouping {
                id: id.clone(),
                name: landing.catch_location.area_grouping_code.clone(),
            })
    }
}

impl NewCatchArea {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<NewCatchArea> {
        landing.catch_location.location_code.map(|id| NewCatchArea {
            id: id as i32,
            latitude: landing.catch_location.location_latitude,
            longitude: landing.catch_location.location_longitude,
        })
    }
}

impl NewCatchMainAreaFao {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<NewCatchMainAreaFao> {
        landing
            .catch_location
            .main_area_fao_code
            .map(|id| NewCatchMainAreaFao {
                id: id as i32,
                name: landing.catch_location.main_area_fao.clone(),
            })
    }
}
