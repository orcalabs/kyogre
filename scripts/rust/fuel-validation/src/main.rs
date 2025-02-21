#![allow(dead_code)]

use std::{
    fmt::Display,
    io::{stdout, Cursor, Write},
};

use anyhow::Result;
use calamine::{open_workbook_from_rs, Data, RangeDeserializerBuilder, Reader, Xlsx};
use serde::de::DeserializeOwned;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    types::chrono::NaiveDate,
    PgPool,
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
}

impl Trip {
    pub fn fuel_total(&self) -> f64 {
        self.entries.iter().map(|v| v.fuel).sum()
    }
}

async fn temp(pool: &PgPool, vessel_id: i64, start: NaiveDate, end: NaiveDate) -> Result<Trip> {
    let entries = sqlx::query_as!(
        TripEntry,
        r#"
SELECT
    e.date::DATE AS "date!",
    e.estimate_liter AS fuel
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

#[tokio::main]
async fn main() -> Result<()> {
    run_nergard().await
}

async fn run_nergard() -> Result<()> {
    let bytes = include_bytes!("../fuel-nergard.xlsx");
    let trips = decode_nergard(bytes)?;

    let pool = connect().await;
    let vessel_id = 2021119797;

    let mut diffs = Vec::with_capacity(trips.len());
    let mut stdout = stdout().lock();

    write_header(&mut stdout)?;

    for trip in trips {
        if trip.entries.is_empty() {
            continue;
        }

        let start = trip.entries[0].date;
        let end = trip.entries.last().unwrap().date;

        let estimated = temp(&pool, vessel_id, start, end).await?;

        let total = trip.fuel_total();
        let estimated_total = estimated.fuel_total();

        let diff_percent = write_stats(&mut stdout, trip.name, estimated_total, total)?;

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

pub fn write_header(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "{0: <15} | {1: <10} | {2: <10} | {3: <10} | {4: <10}",
        "Date/Trip", "Estimate", "Fuel", "Diff", "Diff Percent",
    )?;
    writeln!(
        writer,
        "---------------------------------------------------------------------",
    )?;
    Ok(())
}

pub fn write_stats(
    writer: &mut impl Write,
    ident: impl Display,
    estimate: f64,
    total: f64,
) -> Result<f64> {
    let diff = estimate - total;
    let diff_percent = (100. * diff) / total;

    writeln!(
        writer,
        "{0: <15} | {1:<10.0} | {2:<10.0} | {3:<10.0} | {4:<10.0}",
        ident, estimate, total, diff, diff_percent,
    )?;

    Ok(diff_percent)
}

pub fn decode_nergard(input: impl AsRef<[u8]>) -> Result<Vec<Trip>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let mut vec = Vec::new();

    for (name, range) in doc.worksheets() {
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
