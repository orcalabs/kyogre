use snafu::{Location, Snafu};
use stack_error::StackError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
pub enum Error {
    #[snafu(display("Source error"))]
    #[stack_error(skip_from_impls)]
    Source {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: kyogre_core::Error,
    },
    #[snafu(display("Destination error"))]
    #[stack_error(skip_from_impls)]
    Destination {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: kyogre_core::Error,
    },
}
