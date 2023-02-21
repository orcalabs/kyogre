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

impl From<fiskeridir_rs::CatchLocation> for NewCatchArea {
    fn from(value: fiskeridir_rs::CatchLocation) -> Self {
        NewCatchArea {
            id: value.location_code as i32,
            longitude: value.location_longitude,
            latitude: value.location_latitude,
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
