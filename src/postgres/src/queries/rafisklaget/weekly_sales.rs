use chrono::{NaiveDate, Weekday};
use fiskeridir_rs::{Condition, GearGroup, Quality, VesselLengthGroup};
use kyogre_core::WeeklySale;

use crate::{
    PostgresAdapter,
    error::{InvalidIsoWeekSnafu, Result},
};

impl PostgresAdapter {
    pub(crate) async fn add_weekly_sales_impl(&self, weekly_sales: Vec<WeeklySale>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let len = weekly_sales.len();
        let mut year = Vec::with_capacity(len);
        let mut week = Vec::with_capacity(len);
        let mut length_group = Vec::with_capacity(len);
        let mut gear_group = Vec::with_capacity(len);
        let mut species = Vec::with_capacity(len);
        let mut condition = Vec::with_capacity(len);
        let mut quality = Vec::with_capacity(len);
        let mut sum_net_quantity_kg = Vec::with_capacity(len);
        let mut sum_calculated_living_weight = Vec::with_capacity(len);
        let mut sum_price = Vec::with_capacity(len);

        for v in weekly_sales {
            year.push(v.id.iso_week.year());
            week.push(v.id.iso_week.week() as i32);
            length_group.push(v.id.vessel_length_group);
            gear_group.push(v.id.gear_group);
            species.push(v.id.species as i32);
            condition.push(v.id.condition);
            quality.push(v.id.quality);
            sum_net_quantity_kg.push(v.sum_net_quantity_kg);
            sum_calculated_living_weight.push(v.sum_calculated_living_weight);
            sum_price.push(v.sum_price);
        }

        sqlx::query!(
            r#"
WITH
    _ AS (
        INSERT INTO
            species_fiskeridir (species_fiskeridir_id)
        SELECT
            u.species
        FROM
            UNNEST($1::INT[]) u (species)
        ON CONFLICT DO NOTHING
    ),
    inserted AS (
        INSERT INTO
            rafisklaget_weekly_sales (
                "year",
                week,
                vessel_length_group,
                gear_group,
                species,
                condition,
                quality,
                sum_net_quantity_kg,
                sum_calculated_living_weight,
                sum_price
            )
        SELECT
            "year",
            week,
            vessel_length_group,
            gear_group,
            species,
            condition,
            quality,
            sum_net_quantity_kg,
            sum_calculated_living_weight,
            sum_price
        FROM
            UNNEST(
                $1::INT[],
                $2::INT[],
                $3::INT[],
                $4::INT[],
                $5::INT[],
                $6::INT[],
                $7::INT[],
                $8::DOUBLE PRECISION[],
                $9::DOUBLE PRECISION[],
                $10::DOUBLE PRECISION[]
            ) u (
                species,
                "year",
                week,
                vessel_length_group,
                gear_group,
                condition,
                quality,
                sum_net_quantity_kg,
                sum_calculated_living_weight,
                sum_price
            )
        ON CONFLICT (
            "year",
            week,
            vessel_length_group,
            gear_group,
            species,
            condition,
            quality
        ) DO UPDATE
        SET
            sum_net_quantity_kg = EXCLUDED.sum_net_quantity_kg,
            sum_calculated_living_weight = EXCLUDED.sum_calculated_living_weight,
            sum_price = EXCLUDED.sum_price
        RETURNING
            *
    ),
    updated AS (
        UPDATE landing_entries e
        SET
            estimated_unit_price_for_fisher = i.sum_price / i.sum_calculated_living_weight
        FROM
            inserted i
            INNER JOIN landings l ON DATE_PART('year', l.landing_timestamp) = i.year
            AND DATE_PART('week', l.landing_timestamp) = i.week
            AND l.vessel_length_group_id = i.vessel_length_group
            AND l.gear_group_id = i.gear_group
        WHERE
            e.landing_id = l.landing_id
            AND e.species_fiskeridir_id = i.species
            AND e.product_condition_id = i.condition
            AND e.product_quality_id = i.quality
            AND i.sum_calculated_living_weight != 0
        RETURNING
            l.fiskeridir_vessel_id,
            l.landing_timestamp
    )
UPDATE trips_refresh_boundary t
SET
    refresh_boundary = LEAST(t.refresh_boundary, q.landing_timestamp)
FROM
    (
        SELECT
            u.fiskeridir_vessel_id,
            MIN(u.landing_timestamp) AS landing_timestamp
        FROM
            updated u
        GROUP BY
            u.fiskeridir_vessel_id
    ) q
WHERE
    t.fiskeridir_vessel_id = q.fiskeridir_vessel_id
            "#,
            &species,
            &year,
            &week,
            &length_group as &Vec<VesselLengthGroup>,
            &gear_group as &Vec<GearGroup>,
            &condition as &Vec<Condition>,
            &quality as &Vec<Quality>,
            &sum_net_quantity_kg,
            &sum_calculated_living_weight,
            &sum_price,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn latest_weekly_sale_impl(&self) -> Result<Option<NaiveDate>> {
        let record = sqlx::query!(
            r#"
SELECT
    "year",
    week
FROM
    rafisklaget_weekly_sales
ORDER BY
    "year" DESC,
    week DESC
LIMIT
    1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(v) => NaiveDate::from_isoywd_opt(v.year, v.week as _, Weekday::Mon)
                .ok_or_else(|| {
                    InvalidIsoWeekSnafu {
                        year: v.year,
                        week: v.week,
                    }
                    .build()
                })
                .map(Some),
            None => Ok(None),
        }
    }
}
