use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{Cursor, Write, stdout},
};

use anyhow::Result;
use calamine::{Data, DataType, RangeDeserializerBuilder, Reader, Xlsx, open_workbook_from_rs};
use chrono::{Datelike, Duration, Months};
use clap::{Parser, ValueEnum};
use serde::de::DeserializeOwned;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    types::chrono::NaiveDate,
};

#[derive(Debug, Clone)]
pub struct Trip {
    pub name: String,
    pub entries: Vec<TripEntry>,
}

#[derive(Debug, Clone)]
pub struct TripEntry {
    pub date: NaiveDate,
    pub fuel: f64,
    pub num_ais_positions: i32,
    pub num_vms_positions: i32,
}

impl Trip {
    pub fn fuel_total(&self) -> f64 {
        self.entries.iter().map(|v| v.fuel).sum()
    }
    pub fn ais_total(&self) -> i32 {
        self.entries.iter().map(|v| v.num_ais_positions).sum()
    }
    pub fn vms_total(&self) -> i32 {
        self.entries.iter().map(|v| v.num_vms_positions).sum()
    }
}

async fn get_estimates(
    pool: &PgPool,
    vessel_id: i64,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Trip> {
    let entries = sqlx::query_as!(
        TripEntry,
        r#"
SELECT
    e.date::DATE AS "date!",
    e.estimate_liter AS fuel,
    num_ais_positions,
    num_vms_positions
FROM
    fuel_estimates e
WHERE
    e.fiskeridir_vessel_id = $1
    AND e.date::DATE BETWEEN $2 AND $3
ORDER BY
    e.date
        "#,
        vessel_id,
        start,
        end,
    )
    .fetch_all(pool)
    .await?;

    Ok(Trip {
        name: format!("Estimate from {start} to {end}"),
        entries,
    })
}

#[allow(dead_code)]
async fn get_trip_estimates(
    pool: &PgPool,
    vessel_id: i64,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Trip> {
    let entries = sqlx::query_as!(
        TripEntry,
        r#"
WITH
    trip AS (
        SELECT
            ARRAY_AGG(t.trip_id) AS trip_ids,
            SUM(t.benchmark_fuel_consumption_liter) AS benchmark_fuel_consumption_liter
        FROM
            trips_detailed t
        WHERE
            t.fiskeridir_vessel_id = $1::BIGINT
            AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (
                t.period,
                TSTZRANGE (
                    $2::DATE::TIMESTAMPTZ,
                    ($3::DATE + 1)::TIMESTAMPTZ,
                    '[)'
                )
            ) > 0.5
    )
SELECT
    p.timestamp::DATE AS "date!",
    COALESCE(
        MAX(p.trip_cumulative_fuel_consumption_liter) - MIN(p.trip_cumulative_fuel_consumption_liter),
        0
    ) AS "fuel!",
    COUNT(*) FILTER (
        WHERE
            p.position_type_id = 1
    )::INT AS "num_ais_positions!",
    COUNT(*) FILTER (
        WHERE
            p.position_type_id = 2
    )::INT AS "num_vms_positions!"
FROM
    trip_positions p
    INNER JOIN trip t ON p.trip_id = ANY (t.trip_ids)
GROUP BY
    p.timestamp::DATE
ORDER BY
    p.timestamp::DATE
        "#,
        vessel_id,
        start,
        end,
    )
    .fetch_all(pool)
    .await?;

    Ok(Trip {
        name: format!("Estimate from {start} to {end}"),
        entries,
    })
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Vessels {
    Ramoen,
    #[default]
    Nergaard,
    Heroyfjord,
    Eros,
    SilleMarie,
}

/// Run fuel validation on vessels
#[derive(Parser, Debug)]
struct Args {
    /// Name of the vessel to run validation on
    #[arg(value_enum, short, long, default_value_t = Vessels::default())]
    vessel: Vessels,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.vessel {
        Vessels::Ramoen => run_ramoen().await,
        Vessels::Nergaard => run_nergard().await,
        Vessels::Heroyfjord => {
            let (name, vessel_id, bytes) = (
                "HERØYFJORD",
                2021117460,
                include_bytes!("../Herøyfjord oljeforbruk 2022-24.xlsx"),
            );
            run_heroyfjord_eros(bytes, vessel_id, name).await
        }
        Vessels::Eros => {
            let (name, vessel_id, bytes) = (
                "TUROVERSIKT EROS",
                2013060592,
                include_bytes!("../EROS oljeforbruk 2022 - 2024.xlsx"),
            );
            run_heroyfjord_eros(bytes, vessel_id, name).await
        }
        Vessels::SilleMarie => run_sille_marie().await,
    }
}

async fn run_sille_marie() -> Result<()> {
    let bytes = [
        include_bytes!("../Produksjonsoversikt Sille Marie 2025.xlsx") as &[u8],
        include_bytes!("../Produksjonsoversikt Sille Marie 2024.xlsx") as &[u8],
        include_bytes!("../Produksjonsoversikt Sille Marie 2023.xlsx") as &[u8],
    ];
    let vessel_id = 2023124435;

    let mut trips = decode_sille_marie(&bytes)?;

    let pool = connect().await;

    let mut diffs = Vec::with_capacity(trips.len());
    let mut stdout = stdout().lock();

    write_header(&mut stdout)?;

    trips.sort_by_key(|t| t.entries[0].date);
    for trip in trips {
        if trip.entries.is_empty() {
            continue;
        }

        let start = trip.entries[0].date;
        let end = trip.entries.last().unwrap().date;

        let estimated = get_trip_estimates(&pool, vessel_id, start, end).await?;

        let total = trip.fuel_total();
        let estimated_total = estimated.fuel_total();
        let ais_total = estimated.ais_total();
        let vms_total = estimated.vms_total();

        let diff_percent = write_stats(
            &mut stdout,
            &trip.name,
            ais_total,
            vms_total,
            estimated_total,
            total,
        )?;

        diffs.push(diff_percent.abs());
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs
        .iter()
        .map(|v| ((v - mean).abs().powf(2.)))
        .sum::<f64>()
        / n)
        .sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}

async fn run_ramoen() -> Result<()> {
    let bytes = include_bytes!("../RAMOEN oljeforbruk 2022-24.xlsx");
    let vessel_id = 2016073913;
    let mut trips = decode_ramoen(bytes)?;

    let pool = connect().await;

    let mut diffs = Vec::with_capacity(trips.len());
    let mut stdout = stdout().lock();

    write_header(&mut stdout)?;

    trips.sort_by_key(|t| t.entries[0].date);
    for trip in trips {
        if trip.entries.is_empty() {
            continue;
        }

        let start = trip.entries[0].date;
        let end = trip.entries.last().unwrap().date;

        let estimated = get_estimates(&pool, vessel_id, start, end).await?;

        let total = trip.fuel_total();
        let estimated_total = estimated.fuel_total();
        let ais_total = estimated.ais_total();
        let vms_total = estimated.vms_total();

        let diff_percent = write_stats(
            &mut stdout,
            &trip.name,
            ais_total,
            vms_total,
            estimated_total,
            total,
        )?;

        diffs.push(diff_percent.abs());
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs
        .iter()
        .map(|v| ((v - mean).abs().powf(2.)))
        .sum::<f64>()
        / n)
        .sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}

async fn run_heroyfjord_eros(bytes: &[u8], vessel_id: i64, name: &str) -> Result<()> {
    let trips = decode_heroyfjord_eros(bytes, name)?;

    let pool = connect().await;

    let mut diffs = Vec::with_capacity(trips.len());
    let mut stdout = stdout().lock();

    write_header(&mut stdout)?;

    for trip in trips {
        if trip.entries.is_empty() {
            continue;
        }

        let start = trip.entries[0].date;
        let end = trip.entries.last().unwrap().date;

        let estimated = get_estimates(&pool, vessel_id, start, end).await?;

        let total = trip.fuel_total();
        let estimated_total = estimated.fuel_total();
        let ais_total = estimated.ais_total();
        let vms_total = estimated.vms_total();

        if ais_total + vms_total == 0 {
            continue;
        }

        let diff_percent = write_stats(
            &mut stdout,
            &trip.name,
            ais_total,
            vms_total,
            estimated_total,
            total,
        )?;

        diffs.push(diff_percent.abs());
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs
        .iter()
        .map(|v| ((v - mean).abs().powf(2.)))
        .sum::<f64>()
        / n)
        .sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}

async fn run_nergard() -> Result<()> {
    let bytes = include_bytes!("../fuel-nergard.xlsx");
    let trips = decode_nergard(bytes)?;

    let pool = connect().await;
    let vessel_id = 2021119797;

    let mut diffs = Vec::with_capacity(trips.len());
    let mut stdout = stdout().lock();

    write_header(&mut stdout)?;

    for mut trip in trips {
        if trip.entries.is_empty() {
            continue;
        }

        let start = trip.entries[0].date;
        let end = trip.entries.last().unwrap().date;

        let mut estimated = get_estimates(&pool, vessel_id, start, end).await?;

        let Some((start_idx, _)) = estimated
            .entries
            .iter()
            .enumerate()
            .find(|(_, v)| v.fuel > 0.)
        else {
            continue;
        };
        let Some((end_idx, _)) = estimated
            .entries
            .iter()
            .enumerate()
            .rev()
            .find(|(_, v)| v.fuel > 0.)
        else {
            continue;
        };

        let mut keep_iter = (0..trip.entries.len()).map(|v| v >= start_idx && v <= end_idx);
        trip.entries.retain(|_| keep_iter.next().unwrap());

        let mut keep_iter = (0..estimated.entries.len()).map(|v| v >= start_idx && v <= end_idx);
        estimated.entries.retain(|_| keep_iter.next().unwrap());

        assert_eq!(trip.entries.len(), estimated.entries.len());

        let total = trip.fuel_total();
        let estimated_total = estimated.fuel_total();
        let ais_total = estimated.ais_total();
        let vms_total = estimated.vms_total();

        let diff_percent = write_stats(
            &mut stdout,
            &trip.name,
            ais_total,
            vms_total,
            estimated_total,
            total,
        )?;

        diffs.push(diff_percent.abs());

        let mut file = File::create(format!("./{}.txt", trip.name.trim().replace(' ', "-")))?;

        write_header(&mut file)?;
        write_stats(
            &mut file,
            &trip.name,
            ais_total,
            vms_total,
            estimated_total,
            total,
        )?;
        write_line(&mut file)?;

        for (entry, estimate) in trip.entries.into_iter().zip(estimated.entries) {
            write_stats(
                &mut file,
                entry.date,
                estimate.num_ais_positions,
                estimate.num_vms_positions,
                estimate.fuel,
                entry.fuel,
            )?;
        }
        file.flush()?;
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs
        .iter()
        .map(|v| ((v - mean).abs().powf(2.)))
        .sum::<f64>()
        / n)
        .sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}

pub fn write_header(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "{0: <35} | {1: <10} | {2: <10} | {3: <10} | {4: <10} | {5: <10} | {6: <10}",
        "Date/Trip", "AIS", "VMS", "Estimate", "Fuel", "Diff", "Diff %",
    )?;
    write_line(writer)?;
    Ok(())
}

pub fn write_line(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "{0:-<35}---{0:-<10}---{0:-<10}---{0:-<10}---{0:-<10}---{0:-<10}---{0:-<10}",
        "",
    )?;
    Ok(())
}

pub fn write_stats(
    writer: &mut impl Write,
    ident: impl Display,
    ais_positions: i32,
    vms_positions: i32,
    estimate: f64,
    total: f64,
) -> Result<f64> {
    let diff = estimate - total;
    let diff_percent = (100. * diff) / total;

    writeln!(
        writer,
        "{0: <35} | {1:<10.0} | {2:<10.0} | {3:<10.0} | {4:<10.0} | {5:<10.0} | {6:<10.0}",
        ident.to_string(),
        ais_positions,
        vms_positions,
        estimate,
        total,
        diff,
        diff_percent,
    )?;

    Ok(diff_percent)
}

pub fn decode_sille_marie(inputs: &[impl AsRef<[u8]>]) -> Result<Vec<Trip>> {
    let mut vec = Vec::new();
    for input in inputs {
        vec.extend(decode_sille_marie_impl(input)?);
    }
    Ok(vec)
}

pub fn decode_sille_marie_impl(input: impl AsRef<[u8]>) -> Result<Vec<Trip>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let (_, range) = doc.worksheets().into_iter().nth(1).unwrap();

    let mut trips = Vec::new();

    for row in range.rows().skip(1) {
        let Some(Data::DateTime(start)) = row.get(4) else {
            continue;
        };
        let Some(Data::DateTime(end)) = row.get(6) else {
            continue;
        };
        let Some(Data::Float(fuel)) = row.get(14) else {
            continue;
        };

        let start = start.as_datetime().unwrap().date();
        let end = end.as_datetime().unwrap().date();

        trips.push(Trip {
            name: format!("Trip from {start} to {end}"),
            entries: vec![
                TripEntry {
                    date: start,
                    fuel: 0.,
                    num_ais_positions: 0,
                    num_vms_positions: 0,
                },
                TripEntry {
                    date: end,
                    fuel: *fuel,
                    num_ais_positions: 0,
                    num_vms_positions: 0,
                },
            ],
        });
    }

    Ok(trips)
}

pub fn decode_ramoen(input: impl AsRef<[u8]>) -> Result<Vec<Trip>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let (_, range) = doc.worksheets().into_iter().next().unwrap();

    let mut rows = range.rows();

    // year, current_date
    let mut current_trip_year_date: HashMap<usize, NaiveDate> = HashMap::new();

    let mut row_idx = 0;

    struct YearIndex {
        year: usize,
        fuel: usize,
        date: usize,
    }

    let year_indexes = vec![
        YearIndex {
            year: 2024,
            fuel: 1,
            date: 3,
        },
        YearIndex {
            year: 2023,
            fuel: 6,
            date: 8,
        },
        YearIndex {
            year: 2022,
            fuel: 11,
            date: 13,
        },
    ];

    let mut trips = Vec::new();

    loop {
        let Some(row) = rows.next() else {
            break;
        };
        let is_data_row = !row.is_empty() && row[0].to_string().starts_with("Tur") && row_idx <= 14;

        if is_data_row {
            for y in &year_indexes {
                let mut start = current_trip_year_date
                    .get(&y.year)
                    .cloned()
                    .unwrap_or(NaiveDate::from_ymd_opt(y.year as i32, 1, 1).unwrap());
                if start.month() == 10 {
                    start = start.with_month0(start.month0() + 1).unwrap();
                }
                let fuel = row[y.fuel].get_float().unwrap();
                let duration_days = (row[y.date].get_float().unwrap()) as i64;

                let end = start + Duration::days(duration_days);

                trips.push(Trip {
                    name: format!("Trip from {start} to {end}"),
                    entries: vec![
                        TripEntry {
                            date: start,
                            fuel: 0.,
                            num_ais_positions: 0,
                            num_vms_positions: 0,
                        },
                        TripEntry {
                            date: end,
                            fuel,
                            num_ais_positions: 0,
                            num_vms_positions: 0,
                        },
                    ],
                });

                current_trip_year_date.insert(y.year, end.succ_opt().unwrap());
            }
        }
        row_idx += 1;
    }

    Ok(trips)
}

pub fn decode_heroyfjord_eros(input: impl AsRef<[u8]>, name: &str) -> Result<Vec<Trip>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let mut vec = Vec::new();

    let (_, range) = doc.worksheets().into_iter().next().unwrap();

    let mut rows = range.rows();
    let mut year = 0;
    let mut col = 0;

    loop {
        let Some(row) = rows.next() else {
            break;
        };

        if let Some((i, _)) = row
            .iter()
            .enumerate()
            .find(|(_, v)| matches!(v, Data::String(v) if v == "Oljeforbruk"))
        {
            col = i;
        }

        let first_col = match row.first() {
            Some(Data::String(v)) => v,
            Some(Data::Empty) | None => continue,
            Some(Data::DateTime(v)) => {
                let d = v.as_datetime().unwrap().date();
                &format!("{0} - {0}", d.format("%d.%m"))
            }
            v => {
                panic!("unexpected column: {v:?}, row: {row:?}");
            }
        };

        if let Some(y) = first_col.strip_prefix(name) {
            year = y.trim().parse().unwrap();
            continue;
        }

        let Some((start, end)) = first_col.split_once("-") else {
            continue;
        };

        let start = parse_date(start.trim(), year);
        let mut end = parse_date(end.trim(), year);

        if end < start {
            // Special case, currently only found for Eros
            end = end.checked_add_months(Months::new(1)).unwrap()
        }

        let fuel = match row[col] {
            Data::Float(v) => v,
            Data::Empty => continue,
            _ => panic!("unexpected value: {:?}, expected float", row[col]),
        };

        vec.push(Trip {
            name: format!("Trip from {start} to {end}"),
            entries: vec![
                TripEntry {
                    date: start,
                    fuel: 0.,
                    num_ais_positions: 0,
                    num_vms_positions: 0,
                },
                TripEntry {
                    date: end,
                    fuel: fuel * 1_000.,
                    num_ais_positions: 0,
                    num_vms_positions: 0,
                },
            ],
        });
    }

    Ok(vec)
}

fn parse_date(v: &str, year: i32) -> NaiveDate {
    let (day, month) = v.split_once('.').unwrap();
    let day = day.parse().unwrap();
    let month = month.parse().unwrap();
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

pub fn decode_nergard(input: impl AsRef<[u8]>) -> Result<Vec<Trip>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let mut vec = Vec::new();

    for (name, range) in doc.worksheets() {
        if name.contains("reke")
            || name.contains("Reke")
            || name.contains("3 2022")
            || name.contains("4 2022")
            || name.contains("8 2023")
            || name.contains("ØstGr")
            || name.contains("4 OG 5")
        {
            continue;
        }

        let mut rows = range.rows();

        let Some(first_row) = rows.next() else {
            continue;
        };

        if first_row
            .iter()
            .find(|v| !matches!(v, Data::Empty))
            .is_none_or(|v| !matches!(&v, Data::String(v) if v == "Navn"))
        {
            continue;
        }

        let Some(dates_row) = rows.find(|row| {
            row.iter()
                .find(|v| !matches!(v, Data::Empty))
                .is_some_and(|v| matches!(&v, Data::String(v) if v == "Aktivitet"))
        }) else {
            continue;
        };

        let Some(fuels_row) = rows.find(|row| {
            row.get(1)
                .is_some_and(|v| matches!(&v, Data::String(v) if v == "Bunkersforbruk"))
        }) else {
            continue;
        };

        let iter = dates_row
            .iter()
            .zip(fuels_row)
            .skip_while(|(v, _)| !matches!(v, Data::DateTime(..)));

        let mut entries = Vec::with_capacity(dates_row.len());

        for (date, fuel) in iter {
            let fuel = match fuel {
                Data::Float(v) => *v,
                Data::Empty => continue,
                v => panic!("unexpected value: {v}, expected float"),
            };

            if fuel <= 0. {
                continue;
            }

            let Data::DateTime(date) = date else {
                panic!();
            };

            entries.push(TripEntry {
                fuel,
                date: date.as_datetime().unwrap().date(),
                num_ais_positions: 0,
                num_vms_positions: 0,
            })
        }

        vec.push(Trip { name, entries });
    }

    Ok(vec)
}

async fn connect() -> PgPool {
    PgPoolOptions::new()
        .max_connections(64)
        .connect_with(
            PgConnectOptions::new()
                .application_name("fuel-stuff")
                .username("postgres")
                .password("test123")
                .host("localhost")
                .port(5532),
        )
        .await
        .unwrap()
}

pub fn decode_excel<T: DeserializeOwned>(input: impl AsRef<[u8]>) -> Result<Vec<T>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let mut vec = Vec::new();

    for (_, range) in doc.worksheets() {
        let iter = RangeDeserializerBuilder::new()
            .has_headers(true)
            .from_range(&range)?;

        vec.extend(iter.flatten());
    }

    Ok(vec)
}
