use anyhow::Result;
use chrono::{DateTime, Utc};
use fiskeridir_rs::Gear;
use futures::{StreamExt, TryStreamExt, future};
use geoutils::Location;
use kyogre_core::{
    AisPermission, AisVmsParams, AisVmsPosition, DateRange, FiskeridirVesselId, Vessel,
    WebApiOutboundPort,
};
use orca_core::{PsqlLogStatements, PsqlSettings};
use plotters::{
    backend::BitMapBackend,
    chart::{ChartBuilder, LabelAreaPosition},
    drawing::IntoDrawingArea,
    element::Rectangle,
    series::LineSeries,
    style::{AsRelative, BLACK, Color, Palette, Palette99, WHITE},
};
use postgres::PostgresAdapter;
use processors::{
    FuelImpl, FuelImplDiscriminants, METER_PER_SECONDS_TO_KNOTS, VesselFuelInfo, estimate_fuel,
};

use crate::print_mean_and_sd;

pub async fn run_hera() -> Result<()> {
    let vessel_id = FiskeridirVesselId::new(2001015304);

    let adapter = PostgresAdapter::new(&PsqlSettings {
        ip: "localhost".into(),
        port: 5532,
        db_name: Some("postgres".into()),
        username: "postgres".into(),
        password: Some("test123".into()),
        max_connections: 10,
        root_cert: None,
        log_statements: PsqlLogStatements::Disable,
        application_name: None,
    })
    .await?;

    let vessel = adapter
        .vessels()
        .filter(|v| future::ready(v.as_ref().unwrap().id() == vessel_id))
        .boxed()
        .next()
        .await
        .unwrap()
        .unwrap();

    run_hera_hauls(&adapter, &vessel).await?;

    println!("\n\n");

    run_hera_steaming(&adapter, &vessel).await?;

    Ok(())
}

pub async fn run_hera_hauls(adapter: &PostgresAdapter, vessel: &Vessel) -> Result<()> {
    let mmsi = vessel.mmsi().unwrap();
    let call_sign = vessel.fiskeridir_call_sign().unwrap();

    // let user_hauls = adapter.user_hauls(call_sign).await?;

    let user_hauls = sqlx::query!(
        r#"
SELECT
    start_ts AS "start_ts!",
    end_ts AS "end_ts!",
    start_fuel_liter,
    end_fuel_liter AS "end_fuel_liter!"
FROM
    (
        SELECT
            GREATEST(u.start_ts, h.start_timestamp) - LEAST(u.start_ts, h.start_timestamp) AS start_diff,
            GREATEST(u.end_ts, h.stop_timestamp) - LEAST(u.end_ts, h.stop_timestamp) AS end_diff,
            u.user_haul_id,
            u.haul_id,
            u.start_ts,
            u.end_ts,
            u.start_fuel_liter,
            u.end_fuel_liter,
            h.start_timestamp,
            h.stop_timestamp
        FROM
            user_hauls u
            LEFT JOIN hauls h ON h.haul_id = u.haul_id
        WHERE
            u.call_sign = $1
            AND u.end_ts IS NOT NULL
            AND h.haul_id IS NOT NULL
    )
WHERE
    start_diff + end_diff < interval '10 minutes'
    AND end_fuel_liter - start_fuel_liter > 500
    AND end_fuel_liter - start_fuel_liter < 10000
        "#,
        call_sign,
    )
    .fetch_all(&adapter.pool)
    .await?;

    let mut diffs = Vec::with_capacity(user_hauls.len());
    let mut diffs2 = Vec::with_capacity(user_hauls.len());
    let mut res = Vec::with_capacity(user_hauls.len());

    let root = BitMapBackend::new("./plot.png", (3840, 2160)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Haul Speeds", ("sans-serif", (5).percent_height()))
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .margin((1).percent())
        .build_cartesian_2d(0f64..1f64, 0f64..10f64)?;

    chart
        .configure_mesh()
        .label_style(("sans-serif", (2).percent_height()))
        .y_desc("Speed (knots)")
        .x_desc("Time %")
        .draw()?;

    for (i, haul) in user_hauls.into_iter().enumerate() {
        let mut track = adapter
            .ais_vms_positions(
                AisVmsParams::Range {
                    mmsi: Some(mmsi),
                    call_sign: Some(call_sign.clone()),
                    range: DateRange::new(haul.start_ts, haul.end_ts)?,
                },
                AisPermission::All,
            )
            .map_ok(|v| AisVmsPosition {
                active_gear: Some(Gear::BottomTrawl),
                ..v
            })
            .try_collect::<Vec<_>>()
            .await?;

        let mut speeds = Vec::with_capacity(track.len());
        let mut speed_times = Vec::with_capacity(track.len());

        let mut iter = track.iter();
        let Some(mut prev) = iter.next() else {
            continue;
        };

        for next in iter {
            let secs = (next.timestamp - prev.timestamp).as_seconds_f64();
            if secs <= 0. {
                continue;
            }

            let prev_loc = Location::new(prev.latitude, prev.longitude);
            let next_loc = Location::new(next.latitude, next.longitude);

            let speed = match (prev.speed, next.speed) {
                (Some(a), Some(b)) => (a + b) / 2.,
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => match prev_loc.distance_to(&next_loc) {
                    Ok(v) => (v.meters() / secs) * METER_PER_SECONDS_TO_KNOTS,
                    Err(_) => continue,
                },
            };

            speeds.push(speed);
            speed_times.push((prev.timestamp, speed));
            prev = next;
        }

        // println!("\nDiff Percent (knots)");
        // print_mean_and_sd(&speeds);

        let vessel = VesselFuelInfo::from_core(vessel, None, FuelImplDiscriminants::Maru);
        let mut fuel_impl = FuelImpl::new(&vessel);
        let fuel_estimate = estimate_fuel(&mut fuel_impl, &mut track, &vessel);

        let id = i + 1;
        let duration = (haul.end_ts - haul.start_ts).as_seconds_f64() / 3600.;
        let fuel = (haul.end_fuel_liter - haul.start_fuel_liter) as f64;
        let estimate = fuel_estimate.fuel_liter;
        let diff = estimate - fuel;
        let diff_percent = diff * 100. / fuel;
        let ais = fuel_estimate.num_ais_positions;
        let vms = fuel_estimate.num_vms_positions;

        let haul_usage_per_sec = 12_000. / (24. * 60. * 60.);
        let haul_usage = haul_usage_per_sec * (haul.end_ts - haul.start_ts).as_seconds_f64();
        let haul_usage_diff = haul_usage - fuel;
        let haul_usage_diff_percent = haul_usage_diff * 100. / fuel;

        diffs.push(diff_percent.abs());
        diffs2.push(haul_usage_diff_percent.abs());

        let color = Palette99::pick(i);
        chart
            .draw_series(LineSeries::new(
                speed_times.iter().map(|(time, speed)| {
                    (
                        (*time - haul.start_ts).as_seconds_f64() / 3600. / duration,
                        *speed,
                    )
                }),
                color.stroke_width(3),
            ))?
            .label(format!("Haul {id}"))
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));

        res.push((
            diff_percent.abs(),
            format!(
                "Haul {id}, Duration: {duration:.1}H, Fuel: {fuel:.0}L, Estimate: {estimate:.0}L, Diff: {diff:.0}L ({diff_percent:.0}%), AIS: {ais}, VMS: {vms} -- {haul_usage:.1} {haul_usage_diff:.0}L ({haul_usage_diff_percent:.0}%)"
            ),
        ));
    }

    res.sort_by(|(a, _), (b, _)| b.total_cmp(a));

    for (_, v) in res {
        println!("{v}");
    }

    println!("\nDiff Percent (estimate)");
    print_mean_and_sd(&diffs);

    println!("\nDiff Percent (liters/day)");
    print_mean_and_sd(&diffs2);

    chart
        .configure_series_labels()
        .border_style(BLACK)
        .label_font(("sans-serif", (2).percent_height()))
        .legend_area_size((5).percent_height())
        .draw()?;

    root.present()?;

    Ok(())
}

pub async fn run_hera_steaming(adapter: &PostgresAdapter, vessel: &Vessel) -> Result<()> {
    let mmsi = vessel.mmsi().unwrap();
    let call_sign = vessel.fiskeridir_call_sign().unwrap();

    let speed = 10_f64;
    let start = "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>()?;

    let mut ranges = sqlx::query!(
        r#"
SELECT
    q.timestamp - INTERVAL '1 hour' AS "start!",
    q.timestamp + INTERVAL '1 hour' AS "stop!",
    q.min_speed
FROM
    (
        SELECT
            *,
            (
                SELECT
                    MIN(p2.speed_over_ground)
                FROM
                    ais_positions p2
                WHERE
                    mmsi = $1
                    AND p2.timestamp BETWEEN p.timestamp - INTERVAL '1 hour' AND p.timestamp  + INTERVAL '1 hour'
            ) AS min_speed
        FROM
            ais_positions p
        WHERE
            mmsi = $1
            AND speed_over_ground > $2
            AND timestamp > $3
        ORDER BY
            timestamp DESC
    ) q
WHERE
    q.min_speed > $2
        "#,
        mmsi.into_inner(),
        speed,
        start,
    )
    .fetch_all(adapter.no_plan_cache_pool())
    .await?
    .into_iter();

    let mut id = 1;
    let mut prev = None;

    let mut diffs = Vec::new();

    while let Some(range) = ranges.find(|v| prev.is_none_or(|p| v.stop < p)) {
        let mut track = adapter
            .ais_vms_positions(
                AisVmsParams::Range {
                    mmsi: Some(mmsi),
                    call_sign: Some(call_sign.clone()),
                    range: DateRange::new(range.start, range.stop)?,
                },
                AisPermission::All,
            )
            .try_collect::<Vec<_>>()
            .await?;

        let vessel = VesselFuelInfo::from_core(vessel, None, FuelImplDiscriminants::Maru);
        let mut fuel_impl = FuelImpl::new(&vessel);
        let fuel_estimate = estimate_fuel(&mut fuel_impl, &mut track, &vessel);

        let duration = (range.stop - range.start).as_seconds_f64() / 3600.;
        let estimate = fuel_estimate.fuel_liter;
        let ais = fuel_estimate.num_ais_positions;
        let vms = fuel_estimate.num_vms_positions;

        let steaming_usage_per_sec = 13_000. / (24. * 60. * 60.);
        let steaming_usage = steaming_usage_per_sec * (range.stop - range.start).as_seconds_f64();

        let diff = estimate - steaming_usage;
        let diff_percent = diff * 100. / steaming_usage;

        diffs.push(diff_percent.abs());

        println!(
            "Steaming {id}, Duration: {duration:.1}H, Estimate: {estimate:.0}L, Ballpark: {steaming_usage:.1}L, Diff: {diff:.0}L ({diff_percent:.0}%), AIS: {ais}, VMS: {vms}"
        );

        id += 1;
        prev = Some(range.start);
    }

    println!("\nDiff Percent");
    print_mean_and_sd(&diffs);

    Ok(())
}
