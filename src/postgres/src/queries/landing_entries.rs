use crate::{error::PostgresError, models::NewLandingEntry, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn add_landing_entries<'a>(
        &'a self,
        entries: Vec<NewLandingEntry>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = entries.len();

        let mut landing_id = Vec::with_capacity(len);
        let mut size_grouping_code = Vec::with_capacity(len);
        let mut withdrawn_catch_value = Vec::with_capacity(len);
        let mut catch_value = Vec::with_capacity(len);
        let mut sales_team_fee = Vec::with_capacity(len);
        let mut post_payment = Vec::with_capacity(len);
        let mut support_fee_for_fisher = Vec::with_capacity(len);
        let mut price_for_buyer = Vec::with_capacity(len);
        let mut price_for_fisher = Vec::with_capacity(len);
        let mut unit_price_for_buyer = Vec::with_capacity(len);
        let mut unit_price_for_fisher = Vec::with_capacity(len);
        let mut landing_method_id = Vec::with_capacity(len);
        let mut conservation_method_id = Vec::with_capacity(len);
        let mut product_condition_id = Vec::with_capacity(len);
        let mut product_purpose_id = Vec::with_capacity(len);
        let mut product_purpose_group_id = Vec::with_capacity(len);
        let mut line_number = Vec::with_capacity(len);
        let mut num_fish = Vec::with_capacity(len);
        let mut product_weight = Vec::with_capacity(len);
        let mut product_weight_over_quota = Vec::with_capacity(len);
        let mut gross_weight = Vec::with_capacity(len);
        let mut living_weight = Vec::with_capacity(len);
        let mut living_weight_over_quota = Vec::with_capacity(len);
        let mut species_id = Vec::with_capacity(len);
        let mut species_fao_id = Vec::with_capacity(len);
        let mut species_group_id = Vec::with_capacity(len);
        let mut species_fiskedir_id = Vec::with_capacity(len);
        let mut species_main_group_id = Vec::with_capacity(len);

        for l in entries {
            landing_id.push(l.landing_id);
            size_grouping_code.push(l.size_grouping_code);
            withdrawn_catch_value.push(l.withdrawn_catch_value);
            catch_value.push(l.catch_value);
            sales_team_fee.push(l.sales_team_fee);
            post_payment.push(l.post_payment);
            support_fee_for_fisher.push(l.support_fee_for_fisher);
            price_for_buyer.push(l.price_for_buyer);
            price_for_fisher.push(l.price_for_fisher);
            unit_price_for_buyer.push(l.unit_price_for_buyer);
            unit_price_for_fisher.push(l.unit_price_for_fisher);
            landing_method_id.push(l.landing_method_id);
            conservation_method_id.push(l.conservation_method_id);
            product_condition_id.push(l.product_condition_id);
            product_purpose_id.push(l.product_purpose_id);
            product_purpose_group_id.push(l.product_purpose_group_id);
            line_number.push(l.line_number);
            num_fish.push(l.num_fish);
            product_weight.push(l.product_weight);
            product_weight_over_quota.push(l.product_weight_over_quota);
            gross_weight.push(l.gross_weight);
            living_weight.push(l.living_weight);
            living_weight_over_quota.push(l.living_weight_over_quota);
            species_id.push(l.species_id);
            species_fao_id.push(l.species_fao_id);
            species_group_id.push(l.species_group_id);
            species_fiskedir_id.push(l.species_fiskedir_id);
            species_main_group_id.push(l.species_main_group_id);
        }

        sqlx::query!(
            r#"
INSERT INTO
    landing_entries (
        landing_id,
        size_grouping_code,
        withdrawn_catch_value,
        catch_value,
        sales_team_fee,
        post_payment,
        support_fee_for_fisher,
        price_for_buyer,
        price_for_fisher,
        unit_price_for_buyer,
        unit_price_for_fisher,
        landing_method_id,
        conservation_method_id,
        product_condition_id,
        product_purpose_id,
        product_purpose_group_id,
        line_number,
        num_fish,
        product_weight,
        product_weight_over_quota,
        gross_weight,
        living_weight,
        living_weight_over_quota,
        species_id,
        species_fao_id,
        species_group_id,
        species_fiskedir_id,
        species_main_group_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::VARCHAR[],
        $2::VARCHAR[],
        $3::DECIMAL[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::DECIMAL[],
        $7::DECIMAL[],
        $8::DECIMAL[],
        $9::DECIMAL[],
        $10::DECIMAL[],
        $11::DECIMAL[],
        $12::INT[],
        $13::INT[],
        $14::INT[],
        $15::INT[],
        $16::INT[],
        $17::INT[],
        $18::INT[],
        $19::DECIMAL[],
        $20::DECIMAL[],
        $21::DECIMAL[],
        $22::DECIMAL[],
        $23::DECIMAL[],
        $24::INT[],
        $25::VARCHAR[],
        $26::INT[],
        $27::INT[],
        $28::INT[]
    )
ON CONFLICT (landing_id, line_number) DO NOTHING
                "#,
            landing_id.as_slice(),
            size_grouping_code.as_slice(),
            withdrawn_catch_value.as_slice() as _,
            catch_value.as_slice() as _,
            sales_team_fee.as_slice() as _,
            post_payment.as_slice() as _,
            support_fee_for_fisher.as_slice() as _,
            price_for_buyer.as_slice() as _,
            price_for_fisher.as_slice() as _,
            unit_price_for_buyer.as_slice() as _,
            unit_price_for_fisher.as_slice() as _,
            landing_method_id.as_slice() as _,
            conservation_method_id.as_slice(),
            product_condition_id.as_slice(),
            product_purpose_id.as_slice() as _,
            product_purpose_group_id.as_slice() as _,
            line_number.as_slice(),
            num_fish.as_slice() as _,
            product_weight.as_slice(),
            product_weight_over_quota.as_slice() as _,
            gross_weight.as_slice() as _,
            living_weight.as_slice() as _,
            living_weight_over_quota.as_slice() as _,
            species_id.as_slice(),
            species_fao_id.as_slice() as _,
            species_group_id.as_slice(),
            species_fiskedir_id.as_slice(),
            species_main_group_id.as_slice()
        )
        .execute(&mut *tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
