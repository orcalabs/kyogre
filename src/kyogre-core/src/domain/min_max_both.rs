#[derive(Debug, Clone)]
pub enum MinMaxBoth<T> {
    Min(T),
    Max(T),
    Both { min: T, max: T },
}

impl<T> MinMaxBoth<T> {
    pub fn new(min: Option<T>, max: Option<T>) -> Option<Self> {
        match (min, max) {
            (Some(min), Some(max)) => Some(Self::Both { min, max }),
            (Some(min), None) => Some(Self::Min(min)),
            (None, Some(max)) => Some(Self::Max(max)),
            (None, None) => None,
        }
    }
}
