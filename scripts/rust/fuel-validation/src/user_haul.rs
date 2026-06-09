use anyhow::Result;
use chrono::{DateTime, Utc};
use fiskeridir_rs::Gear;
use futures::{StreamExt, TryStreamExt, future};
use kyogre_core::{
    AisPermission, AisVmsParams, AisVmsPosition, DateRange, FiskeridirVesselId, Vessel,
    WebApiOutboundPort,
};
use orca_core::{PsqlLogStatements, PsqlSettings};
use postgres::PostgresAdapter;
use processors::{FuelImpl, FuelImplDiscriminants, VesselFuelInfo, estimate_fuel};

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

    let user_hauls = adapter.user_hauls(call_sign).await?;

    let mut diffs = Vec::with_capacity(user_hauls.len());
    let mut diffs2 = Vec::with_capacity(user_hauls.len());

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

        let vessel = VesselFuelInfo::from_core(vessel, None, FuelImplDiscriminants::Maru);
        let mut fuel_impl = FuelImpl::new(&vessel);
        let fuel_estimate = estimate_fuel(&mut fuel_impl, &mut track, &vessel);

        let id = i + 1;
        let duration = (haul.end_ts - haul.start_ts).as_seconds_f64() / 3600.;
        let fuel = (haul.start_fuel_liter - haul.end_fuel_liter) as f64;
        let estimate = fuel_estimate.fuel_liter;
        let diff = estimate - fuel;
        let diff_percent = diff * 100. / fuel;
        let ais = fuel_estimate.num_ais_positions;
        let vms = fuel_estimate.num_vms_positions;

        let haul_usage_per_sec = 13_000. / (24. * 60. * 60.);
        let haul_usage = haul_usage_per_sec * (haul.end_ts - haul.start_ts).as_seconds_f64();
        let haul_usage_diff = haul_usage - fuel;
        let haul_usage_diff_percent = haul_usage_diff * 100. / fuel;

        diffs.push(diff_percent.abs());
        diffs2.push(haul_usage_diff_percent.abs());

        println!(
            "Haul {id}, Duration: {duration:.1}H, Fuel: {fuel:.0}L, Estimate: {estimate:.0}L, Diff: {diff:.0}L ({diff_percent:.0}%), AIS: {ais}, VMS: {vms} -- {haul_usage:.1} {haul_usage_diff:.0}L ({haul_usage_diff_percent:.0}%)"
        );
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs.iter().map(|v| (v - mean).abs().powf(2.)).sum::<f64>() / n).sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    let n = diffs2.len() as f64;
    let mean = diffs2.iter().sum::<f64>() / n;

    let sd = (diffs2
        .iter()
        .map(|v| (v - mean).abs().powf(2.))
        .sum::<f64>()
        / n)
        .sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}

pub async fn run_hera_steaming(adapter: &PostgresAdapter, vessel: &Vessel) -> Result<()> {
    let mmsi = vessel.mmsi().unwrap();
    let call_sign = vessel.fiskeridir_call_sign().unwrap();

    let speed = 10_f64;
    let start = "2026-05-01T00:00:00Z".parse::<DateTime<Utc>>()?;

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
    .into_iter()
    ;

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

        let steaming_usage_per_sec = 12_000. / (24. * 60. * 60.);
        let steaming_usage = steaming_usage_per_sec * (range.stop - range.start).as_seconds_f64();

        let diff = estimate - steaming_usage;
        let diff_percent = diff * 100. / steaming_usage;

        diffs.push(diff_percent.abs());

        println!(
            "Haul {id}, Duration: {duration:.1}H, Estimate: {estimate:.0}L, Ballpark: {steaming_usage:.1}L, Diff: {diff:.0}L ({diff_percent:.0}%), AIS: {ais}, VMS: {vms}"
        );

        id += 1;
        prev = Some(range.start);
    }

    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs.iter().map(|v| (v - mean).abs().powf(2.)).sum::<f64>() / n).sqrt();

    println!();
    println!("Mean diff percent: {mean:.0}");
    println!("SD:                {sd:.2}");

    Ok(())
}
