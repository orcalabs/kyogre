use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{FishingFacilitiesQuery, FishingFacilityToolType};
use sqlx::postgres::types::PgRange;

use crate::{error::PostgresError, models::FishingFacility, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn add_fishing_facilities_impl(
        &self,
        facilities: Vec<kyogre_core::FishingFacility>,
    ) -> Result<(), PostgresError> {
        let len = facilities.len();

        let mut tool_id = Vec::with_capacity(len);
        let mut barentswatch_vessel_id = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut call_sign = Vec::with_capacity(len);
        let mut mmsi = Vec::with_capacity(len);
        let mut imo = Vec::with_capacity(len);
        let mut reg_num = Vec::with_capacity(len);
        let mut sbr_reg_num = Vec::with_capacity(len);
        let mut contact_phone = Vec::with_capacity(len);
        let mut contact_email = Vec::with_capacity(len);
        let mut tool_type = Vec::with_capacity(len);
        let mut tool_type_name = Vec::with_capacity(len);
        let mut tool_color = Vec::with_capacity(len);
        let mut tool_count = Vec::with_capacity(len);
        let mut setup_timestamp = Vec::with_capacity(len);
        let mut setup_processed_timestamp = Vec::with_capacity(len);
        let mut removed_timestamp = Vec::with_capacity(len);
        let mut removed_processed_timestamp = Vec::with_capacity(len);
        let mut last_changed = Vec::with_capacity(len);
        let mut source = Vec::with_capacity(len);
        let mut comment = Vec::with_capacity(len);
        let mut geometry_wkt = Vec::with_capacity(len);

        for f in facilities {
            tool_id.push(f.tool_id);
            barentswatch_vessel_id.push(f.barentswatch_vessel_id);
            vessel_name.push(f.vessel_name);
            call_sign.push(f.call_sign.map(|c| c.into_inner()));
            mmsi.push(f.mmsi.map(|m| m.0));
            imo.push(f.imo);
            reg_num.push(f.reg_num);
            sbr_reg_num.push(f.sbr_reg_num);
            contact_phone.push(f.contact_phone);
            contact_email.push(f.contact_email);
            tool_type.push(f.tool_type as i32);
            tool_type_name.push(f.tool_type_name);
            tool_color.push(f.tool_color);
            tool_count.push(f.tool_count);
            setup_timestamp.push(f.setup_timestamp);
            setup_processed_timestamp.push(f.setup_processed_timestamp);
            removed_timestamp.push(f.removed_timestamp);
            removed_processed_timestamp.push(f.removed_processed_timestamp);
            last_changed.push(f.last_changed);
            source.push(f.source);
            comment.push(f.comment);

            let geometry: Geometry<f64> =
                f.geometry_wkt
                    .try_into()
                    .map_err(|e: wkt::geo_types_from_wkt::Error| {
                        report!(PostgresError::DataConversion).attach_printable(e.to_string())
                    })?;
            geometry_wkt.push(wkb::Encode(geometry));
        }

        sqlx::query!(
            r#"
INSERT INTO
    fishing_facilities AS f (
        tool_id,
        barentswatch_vessel_id,
        vessel_name,
        call_sign,
        mmsi,
        imo,
        reg_num,
        sbr_reg_num,
        contact_phone,
        contact_email,
        tool_type,
        tool_type_name,
        tool_color,
        tool_count,
        setup_timestamp,
        setup_processed_timestamp,
        removed_timestamp,
        removed_processed_timestamp,
        last_changed,
        source,
        "comment",
        geometry_wkt
    )
SELECT
    *
FROM
    UNNEST(
        $1::UUID[],
        $2::UUID[],
        $3::TEXT[],
        $4::TEXT[],
        $5::INT[],
        $6::BIGINT[],
        $7::TEXT[],
        $8::TEXT[],
        $9::TEXT[],
        $10::TEXT[],
        $11::INT[],
        $12::TEXT[],
        $13::TEXT[],
        $14::INT[],
        $15::TIMESTAMPTZ[],
        $16::TIMESTAMPTZ[],
        $17::TIMESTAMPTZ[],
        $18::TIMESTAMPTZ[],
        $19::TIMESTAMPTZ[],
        $20::TEXT[],
        $21::TEXT[],
        $22::GEOMETRY[]
    )
ON CONFLICT (tool_id) DO
UPDATE
SET
    barentswatch_vessel_id = COALESCE(
        f.barentswatch_vessel_id,
        EXCLUDED.barentswatch_vessel_id
    ),
    vessel_name = COALESCE(f.vessel_name, EXCLUDED.vessel_name),
    call_sign = COALESCE(f.call_sign, EXCLUDED.call_sign),
    mmsi = COALESCE(f.mmsi, EXCLUDED.mmsi),
    imo = COALESCE(f.imo, EXCLUDED.imo),
    reg_num = COALESCE(f.reg_num, EXCLUDED.reg_num),
    sbr_reg_num = COALESCE(f.sbr_reg_num, EXCLUDED.sbr_reg_num),
    contact_phone = COALESCE(f.contact_phone, EXCLUDED.contact_phone),
    contact_email = COALESCE(f.contact_email, EXCLUDED.contact_email),
    tool_type = COALESCE(f.tool_type, EXCLUDED.tool_type),
    tool_type_name = COALESCE(f.tool_type_name, EXCLUDED.tool_type_name),
    tool_color = COALESCE(f.tool_color, EXCLUDED.tool_color),
    tool_count = COALESCE(f.tool_count, EXCLUDED.tool_count),
    setup_timestamp = COALESCE(f.setup_timestamp, EXCLUDED.setup_timestamp),
    setup_processed_timestamp = COALESCE(
        f.setup_processed_timestamp,
        EXCLUDED.setup_processed_timestamp
    ),
    removed_timestamp = COALESCE(f.removed_timestamp, EXCLUDED.removed_timestamp),
    removed_processed_timestamp = COALESCE(
        f.removed_processed_timestamp,
        EXCLUDED.removed_processed_timestamp
    ),
    last_changed = COALESCE(f.last_changed, EXCLUDED.last_changed),
    source = COALESCE(f.source, EXCLUDED.source),
    "comment" = COALESCE(f."comment", EXCLUDED."comment"),
    geometry_wkt = COALESCE(f.geometry_wkt, EXCLUDED.geometry_wkt)
            "#,
            tool_id.as_slice(),
            barentswatch_vessel_id.as_slice() as _,
            vessel_name.as_slice() as _,
            call_sign.as_slice() as _,
            mmsi.as_slice() as _,
            imo.as_slice() as _,
            reg_num.as_slice() as _,
            sbr_reg_num.as_slice() as _,
            contact_phone.as_slice() as _,
            contact_email.as_slice() as _,
            tool_type.as_slice(),
            tool_type_name.as_slice() as _,
            tool_color.as_slice() as _,
            tool_count.as_slice() as _,
            setup_timestamp.as_slice(),
            setup_processed_timestamp.as_slice() as _,
            removed_timestamp.as_slice() as _,
            removed_processed_timestamp.as_slice() as _,
            last_changed.as_slice(),
            source.as_slice() as _,
            comment.as_slice() as _,
            geometry_wkt.as_slice() as _,
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) fn fishing_facilities_impl(
        &self,
        query: FishingFacilitiesQuery,
    ) -> impl Stream<Item = Result<FishingFacility, PostgresError>> + '_ {
        let args: FishingFacilitiesArgs = query.into();

        sqlx::query_as!(
            FishingFacility,
            r#"
SELECT
    tool_id,
    barentswatch_vessel_id,
    vessel_name,
    call_sign,
    mmsi,
    imo,
    reg_num,
    sbr_reg_num,
    contact_phone,
    contact_email,
    tool_type AS "tool_type!: FishingFacilityToolType",
    tool_type_name,
    tool_color,
    tool_count,
    setup_timestamp AS "setup_timestamp!",
    setup_processed_timestamp,
    removed_timestamp,
    removed_processed_timestamp,
    last_changed AS "last_changed!",
    source,
    "comment",
    geometry_wkt AS "geometry_wkt: _"
FROM
    fishing_facilities
WHERE
    (
        $1::INT[] IS NULL
        OR mmsi = ANY ($1)
    )
    AND (
        $2::TEXT[] IS NULL
        OR call_sign = ANY ($2)
    )
    AND (
        $3::INT[] IS NULL
        OR tool_type = ANY ($3)
    )
    AND (
        $4::BOOLEAN IS NULL
        OR CASE
            WHEN $4 THEN removed_timestamp IS NULL
            WHEN NOT $4 THEN removed_timestamp IS NOT NULL
        END
    )
    AND (
        $5::TSTZRANGE[] IS NULL
        OR setup_timestamp <@ ANY ($5)
    )
    AND (
        $6::TSTZRANGE[] IS NULL
        OR removed_timestamp <@ ANY ($6)
    )
            "#,
            args.mmsis as _,
            args.call_signs as _,
            args.tool_types as _,
            args.active,
            args.setup_ranges,
            args.removed_ranges,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}

#[derive(Debug, Clone)]
pub struct FishingFacilitiesArgs {
    pub mmsis: Option<Vec<i32>>,
    pub call_signs: Option<Vec<String>>,
    pub tool_types: Option<Vec<i32>>,
    pub active: Option<bool>,
    pub setup_ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub removed_ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
}

impl From<FishingFacilitiesQuery> for FishingFacilitiesArgs {
    fn from(v: FishingFacilitiesQuery) -> Self {
        Self {
            mmsis: v.mmsis.map(|ms| ms.into_iter().map(|m| m.0).collect()),
            call_signs: v
                .call_signs
                .map(|cs| cs.into_iter().map(|c| c.into_inner()).collect()),
            tool_types: v
                .tool_types
                .map(|ts| ts.into_iter().map(|t| t as i32).collect()),
            active: v.active,
            setup_ranges: v.setup_ranges.map(|ss| {
                ss.into_iter()
                    .map(|s| PgRange {
                        start: s.start,
                        end: s.end,
                    })
                    .collect()
            }),
            removed_ranges: v.removed_ranges.map(|rs| {
                rs.into_iter()
                    .map(|r| PgRange {
                        start: r.start,
                        end: r.end,
                    })
                    .collect()
            }),
        }
    }
}
