use std::{any::type_name, collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{Datelike, Duration, IsoWeek, NaiveDate, NaiveDateTime, Utc, Weekday};
use fiskeridir_rs::{Condition, GearGroup, Quality, SalesTeam, VesselLengthGroup};
use http_client::HttpClient;
use kyogre_core::{BearerToken, WeeklySale, WeeklySaleId};
use num_traits::FromPrimitive;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DisplayFromStr};
use tracing::info;

use crate::{ApiClientConfig, DataSource, Error, Processor, ScraperId};

pub struct WeeklySalesScraper {
    config: Option<ApiClientConfig>,
    client: Arc<HttpClient>,
}

impl WeeklySalesScraper {
    pub fn new(config: Option<ApiClientConfig>, client: Arc<HttpClient>) -> Self {
        Self { config, client }
    }
}

#[async_trait]
impl DataSource for WeeklySalesScraper {
    fn id(&self) -> ScraperId {
        ScraperId::FishingFacility
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), Error> {
        let Some(config) = &self.config else {
            return Ok(());
        };
        let Some(oauth) = &config.oauth else {
            return Ok(());
        };

        let token = BearerToken::acquire(oauth).await?;

        // The API does not support querying the current week,
        // so only scrape until last week.
        let end_date = Utc::now().date_naive() - Duration::weeks(1);

        let mut scrape_date = processor
            .latest_weekly_sale()
            .await?
            .unwrap_or_else(|| (end_date - Duration::days(365)));

        loop {
            let iso_week = scrape_date.iso_week();

            if iso_week > end_date.iso_week() {
                break;
            }

            let weekly_sales: WeeklySales = self
                .client
                .download(
                    &config.url,
                    Some(&WeeklySalesQuery {
                        iso_week,
                        group_by_vessel_length: Some(true),
                        group_by_coast12nm: None,
                        group_by_landing_zone: None,
                        include_byproducts: None,
                        include_foreign_vessels: None,
                    }),
                    Some(&token),
                )
                .await?;

            let mut map = HashMap::with_capacity(weekly_sales.salesdata.len());

            for s in weekly_sales.salesdata {
                map.entry(WeeklySaleId {
                    iso_week,
                    vessel_length_group: s.vessel_length_group,
                    gear_group: s.gear_category,
                    species: s.species,
                    condition: s.condition,
                    quality: s.quality,
                })
                .and_modify(|v: &mut WeeklySale| {
                    v.sum_net_quantity_kg += s.sum_net_quantity_kg;
                    v.sum_calculated_living_weight += s.sum_calculated_live_weight_kg;
                    v.sum_price += s.sum_amount;
                })
                .or_insert_with(|| WeeklySale {
                    id: WeeklySaleId {
                        iso_week,
                        vessel_length_group: s.vessel_length_group,
                        gear_group: s.gear_category,
                        species: s.species,
                        condition: s.condition,
                        quality: s.quality,
                    },
                    sum_net_quantity_kg: s.sum_net_quantity_kg,
                    sum_calculated_living_weight: s.sum_calculated_live_weight_kg,
                    sum_price: s.sum_amount,
                });
            }

            processor
                .add_weekly_sales(map.into_values().collect())
                .await?;

            scrape_date += Duration::weeks(1);
        }

        info!("successfully scraped weekly sales");

        Ok(())
    }
}

#[allow(dead_code)]
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklySales {
    pub description: String,
    /// SalesTeam
    #[serde(deserialize_with = "deserialize_primitive")]
    pub sales_organization_id: SalesTeam,
    #[serde(deserialize_with = "deserialize_iso_week")]
    pub sales_week: IsoWeek,
    /// Time when server generated the response. ISO 8601 format
    pub registration_time: NaiveDateTime,
    /// ISO 4217
    pub currency: String,
    /// As passed in query
    pub include_byproducts: bool,
    /// As passed in query
    pub include_foreign_vessels: bool,
    /// As passed in query
    pub group_by_vessel_length: bool,
    /// As passed in query
    pub group_by_coast12nm: bool,
    /// As passed in query
    pub group_by_landing_zone: bool,
    pub salesdata: Vec<SalesData>,
}

#[allow(dead_code)]
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesData {
    #[serde(deserialize_with = "deserialize_vessel_length_group")]
    pub vessel_length_group: VesselLengthGroup,
    /// True if vessel is Norwegian
    pub vessel_nation_nor: Option<bool>,
    #[serde(deserialize_with = "deserialize_gear_group")]
    pub gear_category: GearGroup,
    /// True if catch inside 12 nm
    pub coast12nm: Option<bool>,
    pub landing_zone: Option<String>,
    /// See code list
    #[serde_as(as = "DisplayFromStr")]
    pub species: u32,
    /// See code list
    pub size: String,
    /// | species                        | sizeGroup | Description NOR | Description ENG |
    /// |--------------------------------|------|-----------------|-----------------|
    /// |                                | 0    | Uspesifisert    | Unspecified     |
    /// | 1032                           | 1    | -1,2 kg         | -1,2 kg         |
    /// | 1032                           | 2    | 1,2-2,3 kg      | 1,2-2,3 kg      |
    /// | 1032                           | 3    | 2,3+ kg         | 2,3+ kg         |
    /// | 2311                           | 1    | -4,0 kg         | -4,0 kg         |
    /// | 2311                           | 2    | 4-20 kg         | 4-20 kg         |
    /// | 2311                           | 3    | 20-40 kg        | 20-40 kg        |
    /// | 2311                           | 3    | 40-60 kg        | 40-60 kg        |
    /// | 2311                           | 4    | 60+ kg          | 60+ kg          |
    /// | 2311                           | 5    | 100+ kg         | 100+ kg         |
    /// | 2313                           | 1    | -1,0 kg         | -1,0 kg         |
    /// | 2313                           | 2    | 1,0-2,0 kg      | 1,0-2,0 kg      |
    /// | 2313                           | 3    | 2,0+ kg         | 2,0+ kg         |
    /// | 2524                           | 1    | 291-999 stk     | 291-999 pcs     |
    /// | 2524                           | 2    | 271-290 stk     | 271-290 pcs     |
    /// | 2524                           | 3    | 251-270 stk     | 251-270 pcs     |
    /// | 2524                           | 4    | 231-250 stk     | 231-250 pcs     |
    /// | 2524                           | 5    | 201-230 stk     | 201-230 pcs     |
    /// | 2524                           | 6    | 181-200 stk     | 181-200 pcs     |
    /// | 2524                           | 7    | 161-180 stk     | 161-180 pcs     |
    /// | 2524                           | 8    | 121-160 stk     | 121-160 pcs     |
    /// | 2524                           | 9    | 050-120 stk     | 050-120 pcs     |
    /// | 2534                           | 1    | -0,8 kg         | -0,8 kg         |
    /// | 2534                           | 5    | 3,2+ kg         | 3,2+ kg         |
    /// | 102201, 102202, 102204, 102205 | 1    | -1,0 kg         | -1,0 kg         |
    /// | 102201, 102202, 102204, 102205 | 2    | 1,0-2,5 kg      | 1,0-2,5 kg      |
    /// | 102201, 102202, 102204, 102205 | 3    | 2,5-6,0 kg      | 2,5-6,0 kg      |
    /// | 102201, 102202, 102204, 102205 | 4    | 6,0+ kg         | 6,0+ kg         |
    /// | 102701                         | 1    | -0,57 kg        | -0,57 kg        |
    /// | 102701                         | 2    | 0,57-0,8 kg     | 0,57-0,8 kg     |
    /// | 102701                         | 3    | 0,8+ kg         | 0,8+ kg         |
    /// | 253410, 253420                 | 1    | -0,8 kg         | -0,8 kg         |
    /// | 253410, 253420                 | 2    | 0,8-1,6 kg      | 0,8-1,6 kg      |
    /// | 253410, 253420                 | 3    | 1,6-2,2 kg      | 1,6-2,2 kg      |
    /// | 253410, 253420                 | 4    | 2,2-3,2 kg      | 2,2-3,2 kg      |
    /// | 253410, 253420                 | 5    | 3,2+ kg         | 3,2+ kg         |
    pub size_group: Option<String>,
    /// See code list
    #[serde(deserialize_with = "deserialize_primitive")]
    pub condition: Condition,
    /// See code list
    pub preservation: String,
    /// See code list
    #[serde(deserialize_with = "deserialize_primitive")]
    pub quality: Quality,
    /// True if byproduct
    pub byproduct: Option<bool>,
    pub sum_net_quantity_kg: f64,
    /// <https://www.fiskeridir.no/Yrkesfiske/Tema/Omregningsfaktorer>
    pub sum_calculated_live_weight_kg: f64,
    pub sum_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklySalesQuery {
    #[serde(serialize_with = "serialize_iso_week")]
    pub iso_week: IsoWeek,
    pub group_by_coast12nm: Option<bool>,
    pub group_by_landing_zone: Option<bool>,
    pub group_by_vessel_length: Option<bool>,
    pub include_byproducts: Option<bool>,
    pub include_foreign_vessels: Option<bool>,
}

fn serialize_iso_week<S: Serializer>(value: &IsoWeek, serializer: S) -> Result<S::Ok, S::Error> {
    let s = format!("{}W{:0>2}", value.year(), value.week());
    serializer.serialize_str(&s)
}

fn deserialize_iso_week<'de, D: Deserializer<'de>>(deserializer: D) -> Result<IsoWeek, D::Error> {
    use serde::de::Error;

    let s = <&str>::deserialize(deserializer)?;

    let err = || D::Error::invalid_value(de::Unexpected::Str(s), &"a valid ISO week");

    let Some((year, week)) = s.split_once('W') else {
        return Err(err());
    };

    let year = year.parse().map_err(|_| err())?;
    let week = week.parse().map_err(|_| err())?;

    NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)
        .map(|v| v.iso_week())
        .ok_or_else(err)
}

fn deserialize_primitive<'de, D: Deserializer<'de>, T: FromPrimitive>(
    deserializer: D,
) -> Result<T, D::Error> {
    use serde::de::Error;

    let s = <&str>::deserialize(deserializer)?;

    let err = || {
        D::Error::invalid_value(
            de::Unexpected::Str(s),
            &format!("a valid {}", type_name::<T>()).as_str(),
        )
    };

    let num = s.parse().map_err(|_| err())?;
    T::from_i64(num).ok_or_else(err)
}

fn deserialize_vessel_length_group<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<VesselLengthGroup, D::Error> {
    use serde::de::Error;

    let s = <&str>::deserialize(deserializer)?;

    // | vesselLengthGroup | Description   |
    // |-----------|---------------|
    // | UNSPEC    | Unspecified   |
    // | [1,11)    | Under 11 m    |
    // | [11,15)   | 11 - 14,99 m  |
    // | [15,21)   | 15 - 20,99 m  |
    // | [21,28)   | 21 -27,99 m   |
    // | [28,1000) | 28 m and over |
    match s {
        "UNSPEC" => Ok(VesselLengthGroup::Unknown),
        "[1,11)" => Ok(VesselLengthGroup::UnderEleven),
        "[11,15)" => Ok(VesselLengthGroup::ElevenToFifteen),
        "[15,21)" => Ok(VesselLengthGroup::FifteenToTwentyOne),
        "[21,28)" => Ok(VesselLengthGroup::TwentyTwoToTwentyEight),
        "[28,1000)" => Ok(VesselLengthGroup::TwentyEightAndAbove),
        _ => Err(D::Error::invalid_value(
            de::Unexpected::Str(s),
            &"a valid VesselLengthGroup",
        )),
    }
}

fn deserialize_gear_group<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<GearGroup, D::Error> {
    use serde::de::Error;

    let s = <&str>::deserialize(deserializer)?;

    // | gearCategory | Description NOR | Description ENG |
    // |-------|-----------------|-----------------|
    // | NT    | Not             | Seine           |
    // | GA    | Garn            | Nets            |
    // | JU    | Juksa           | Jigs            |
    // | LI    | Line            | Longline        |
    // | AL    | Autoline        | Autoline        |
    // | RU    | Ruser           | Traps           |
    // | TE    | Teiner          | Pots            |
    // | TR    | Trål            | Trawl           |
    // | SV    | Snurrevad       | Danish seine    |
    // | HV    | Hvalkanon       | Cannon          |
    // | AN    | Annet           | Other           |
    match s {
        "NT" => Ok(GearGroup::Seine),
        "GA" => Ok(GearGroup::Net),
        "JU" | "LI" | "AL" => Ok(GearGroup::HookGear),
        "TE" | "RU" => Ok(GearGroup::LobsterTrapAndFykeNets),
        "TR" => Ok(GearGroup::Trawl),
        "SV" => Ok(GearGroup::DanishSeine),
        "HV" => Ok(GearGroup::HarpoonCannon),
        "AN" => Ok(GearGroup::OtherGear),
        _ => Err(D::Error::invalid_value(
            de::Unexpected::Str(s),
            &"a valid GearGroup",
        )),
    }
}
