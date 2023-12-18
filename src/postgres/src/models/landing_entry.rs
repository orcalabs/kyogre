use crate::{
    error::PostgresErrorWrapper,
    queries::{float_to_decimal, opt_float_to_decimal},
};
use bigdecimal::BigDecimal;
use unnest_insert::UnnestInsert;

#[derive(UnnestInsert)]
#[unnest_insert(table_name = "landing_entries", conflict = "landing_id,line_number")]
pub struct NewLandingEntry {
    // Dokumentnummer-SalgslagId-Dokumenttype
    pub landing_id: String,
    // Størrelsesgruppering (kode)
    pub size_grouping_code: String,
    // Inndradd fangstverdi
    pub withdrawn_catch_value: Option<BigDecimal>,
    // Fangstverdi
    pub catch_value: Option<BigDecimal>,
    // Lagsavgift
    pub sales_team_fee: Option<BigDecimal>,
    // Etterbetaling
    pub post_payment: Option<BigDecimal>,
    // Støttebeløp
    pub support_fee_for_fisher: Option<BigDecimal>,
    // Beløp for kjøper
    pub price_for_buyer: Option<BigDecimal>,
    // Beløp for fisker
    pub price_for_fisher: Option<BigDecimal>,
    // Enhetspris for kjøper
    pub unit_price_for_buyer: Option<BigDecimal>,
    // Enhetspris for fisker
    pub unit_price_for_fisher: Option<BigDecimal>,
    // Landingsmåte (kode)
    pub landing_method_id: Option<i32>,
    // Konserveringsmåte (kode)
    pub conservation_method_id: i32,
    // Produkttilstand (kode)
    pub product_condition_id: i32,
    // Anvendelse (kode)
    pub product_purpose_id: Option<i32>,
    // Anvendelse hovedgruppe (kode)
    pub product_purpose_group_id: Option<i32>,
    // Linjenummer
    pub line_number: i32,
    // Antall stykk
    pub num_fish: Option<i32>,
    // Produktvekt
    pub product_weight: BigDecimal,
    // Produktvekt over kvote
    pub product_weight_over_quota: Option<BigDecimal>,
    // Bruttovekt
    pub gross_weight: Option<BigDecimal>,
    // Rundvekt
    pub living_weight: Option<BigDecimal>,
    // Rundvekt over kvote
    pub living_weight_over_quota: Option<BigDecimal>,
    // Art (kode)
    pub species_id: i32,
    //  Art FAO (kode)
    pub species_fao_id: Option<String>,
    // Art - gruppe (kode)
    pub species_group_id: i32,
    // Art - FDIR (kode)
    pub species_fiskeridir_id: i32,
    // Art - hovedgruppe (kode)
    pub species_main_group_id: i32,
}

impl TryFrom<&fiskeridir_rs::Landing> for NewLandingEntry {
    type Error = PostgresErrorWrapper;

    fn try_from(landing: &fiskeridir_rs::Landing) -> Result<Self, Self::Error> {
        Ok(NewLandingEntry {
            landing_id: landing.id.clone().into_inner(),
            size_grouping_code: landing.product.size_grouping_code.clone(),
            withdrawn_catch_value: opt_float_to_decimal(landing.finances.withdrawn_catch_value)?,
            catch_value: opt_float_to_decimal(landing.finances.catch_value)?,
            sales_team_fee: opt_float_to_decimal(landing.finances.sales_team_fee)?,
            post_payment: opt_float_to_decimal(landing.finances.post_payment)?,
            support_fee_for_fisher: opt_float_to_decimal(
                landing.finances.support_amount_for_fisher,
            )?,
            price_for_buyer: opt_float_to_decimal(landing.finances.price_for_buyer)?,
            price_for_fisher: opt_float_to_decimal(landing.finances.price_for_fisher)?,
            unit_price_for_buyer: opt_float_to_decimal(landing.finances.unit_price_for_buyer)?,
            unit_price_for_fisher: opt_float_to_decimal(landing.finances.unit_price_for_fisher)?,
            landing_method_id: landing.product.landing_method.map(|v| v as i32),
            conservation_method_id: landing.product.conservation_method as i32,
            product_condition_id: landing.product.condition as i32,
            product_purpose_id: landing.product.purpose.code.map(|v| v as i32),
            product_purpose_group_id: landing.product.purpose.group_code.map(|v| v as i32),
            line_number: landing.line_number,
            num_fish: landing.product.num_fish.map(|v| v as i32),
            product_weight: float_to_decimal(landing.product.product_weight)?,
            product_weight_over_quota: opt_float_to_decimal(
                landing.product.product_weight_over_quota,
            )?,
            gross_weight: opt_float_to_decimal(landing.product.gross_weight)?,
            living_weight: opt_float_to_decimal(landing.product.living_weight)?,
            living_weight_over_quota: opt_float_to_decimal(
                landing.product.living_weight_over_quota,
            )?,
            species_id: landing.product.species.code as i32,
            species_fao_id: landing.product.species.fao_code.clone(),
            species_group_id: landing.product.species.group_code as i32,
            species_fiskeridir_id: landing.product.species.fdir_code as i32,
            species_main_group_id: landing.product.species.main_group_code as i32,
        })
    }
}
