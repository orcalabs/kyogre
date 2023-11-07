use chrono::{DateTime, Datelike, Duration, Utc};

mod fishing_spot_predictor;
mod fishing_weight_predictor;

pub use fishing_spot_predictor::*;
pub use fishing_weight_predictor::*;

fn is_last_week_of_year(current_time: DateTime<Utc>) -> bool {
    let current_week = current_time.iso_week().week();
    let current_year = current_time.year();

    (current_week == 52 || current_week == 53)
        && (current_time + Duration::weeks(1)).year() != current_year
}

pub enum PredictionRange {
    CurrentYear,
    CurrentWeekAndNextWeek,
    WeeksFromStartOfYear(u32),
}

pub struct PredictionTarget {
    pub week: u32,
    pub year: u32,
}

impl PredictionRange {
    fn prediction_targets(&self) -> Vec<PredictionTarget> {
        let now = Utc::now();
        let current_week = now.iso_week().week();
        let current_year = now.year() as u32;
        let is_end_of_year = is_last_week_of_year(now);

        match self {
            PredictionRange::CurrentYear => {
                let mut targets = Vec::with_capacity(current_week as usize);
                for i in 1..=current_week {
                    targets.push(PredictionTarget {
                        week: i,
                        year: current_year,
                    });
                }
                targets
            }
            PredictionRange::CurrentWeekAndNextWeek => {
                if is_end_of_year {
                    vec![
                        PredictionTarget {
                            week: current_week,
                            year: current_year,
                        },
                        PredictionTarget {
                            week: 1,
                            year: current_year + 1,
                        },
                    ]
                } else {
                    vec![
                        PredictionTarget {
                            week: current_week,
                            year: current_year,
                        },
                        PredictionTarget {
                            week: current_week + 1,
                            year: current_year,
                        },
                    ]
                }
            }
            PredictionRange::WeeksFromStartOfYear(max_week) => {
                let mut targets = Vec::with_capacity(*max_week as usize);
                for i in 1..=*max_week {
                    targets.push(PredictionTarget {
                        week: i,
                        year: current_year,
                    });
                }
                targets
            }
        }
    }
}
