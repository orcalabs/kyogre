use geoutils::Location;

#[derive(Debug)]
pub struct LocationDistanceToError {
    pub from: Location,
    pub to: Location,
}

impl std::error::Error for LocationDistanceToError {}

impl std::fmt::Display for LocationDistanceToError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "an error occured when calculating distance from {:?} to {:?}",
            self.from, self.to
        ))
    }
}
