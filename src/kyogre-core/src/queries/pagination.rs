#[derive(Debug, Clone)]
pub struct Trips;
#[derive(Debug, Clone)]
pub struct FishingFacilities;

const MAX_TRIPS_LIMIT: u64 = 100;
const DEFAULT_TRIPS_LIMIT: u64 = 20;

const MAX_FISHING_FACILITIES_LIMIT: u64 = 100;
const DEFAULT_FISHING_FACILITIES_LIMIT: u64 = 20;

#[derive(Debug, Clone)]
pub struct Pagination<T> {
    limit: u64,
    offset: u64,
    phantom: std::marker::PhantomData<T>,
}

macro_rules! impl_pagination {
    ($type: ty, $max: ident, $default: ident) => {
        impl Pagination<$type> {
            pub fn new(limit: Option<u64>, offset: Option<u64>) -> Pagination<$type> {
                Pagination::inner_new(limit, offset, $max, $default)
            }
        }
        impl Default for Pagination<$type> {
            fn default() -> Self {
                Pagination::<$type>::new(Some($default), None)
            }
        }
    };
}

impl_pagination!(Trips, MAX_TRIPS_LIMIT, DEFAULT_TRIPS_LIMIT);
impl_pagination!(
    FishingFacilities,
    MAX_FISHING_FACILITIES_LIMIT,
    DEFAULT_FISHING_FACILITIES_LIMIT
);

impl<T> Pagination<T> {
    pub fn limit(&self) -> u64 {
        self.limit
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    fn inner_new(limit: Option<u64>, offset: Option<u64>, max: u64, default: u64) -> Pagination<T> {
        let val = limit.unwrap_or(default);
        let limit = match val {
            i if i > max || i == 0 => default,
            _ => val,
        };
        Pagination {
            limit,
            offset: offset.unwrap_or_default(),
            phantom: std::marker::PhantomData,
        }
    }
}
