use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{
    FishingFacilitiesQuery, FishingFacilitiesSorting, FishingFacilityApiSource,
    FishingFacilityToolType, Ordering,
};
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
        let mut api_source = Vec::with_capacity(len);

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
            api_source.push(f.api_source as i32);
        }

        sqlx::query!(
            r#"
INSERT INTO
    fishing_facilities AS f (
        tool_id,
        barentswatch_vessel_id,
        fiskeridir_vessel_id,
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
        geometry_wkt,
        api_source
    )
SELECT
    u.tool_id,
    u.barentswatch_vessel_id,
    v.fiskeridir_vessel_id,
    u.vessel_name,
    u.call_sign,
    u.mmsi,
    u.imo,
    u.reg_num,
    u.sbr_reg_num,
    u.contact_phone,
    u.contact_email,
    u.tool_type,
    u.tool_type_name,
    u.tool_color,
    u.tool_count,
    u.setup_timestamp,
    u.setup_processed_timestamp,
    u.removed_timestamp,
    u.removed_processed_timestamp,
    u.last_changed,
    u.source,
    u.comment,
    u.geometry_wkt,
    u.api_source
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
        $22::GEOMETRY[],
        $23::INT[]
    ) u (
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
        geometry_wkt,
        api_source
    )
    LEFT JOIN (
        SELECT
            call_sign,
            MIN(fiskeridir_vessel_id) AS fiskeridir_vessel_id
        FROM
            fiskeridir_vessels
        GROUP BY
            call_sign
        HAVING
            COUNT(fiskeridir_vessel_id) = 1
    ) v ON v.call_sign = u.call_sign
ON CONFLICT (tool_id) DO
UPDATE
SET
    barentswatch_vessel_id = COALESCE(
        EXCLUDED.barentswatch_vessel_id,
        f.barentswatch_vessel_id
    ),
    fiskeridir_vessel_id = COALESCE(
        EXCLUDED.fiskeridir_vessel_id,
        f.fiskeridir_vessel_id
    ),
    vessel_name = COALESCE(EXCLUDED.vessel_name, f.vessel_name),
    call_sign = COALESCE(EXCLUDED.call_sign, f.call_sign),
    mmsi = COALESCE(EXCLUDED.mmsi, f.mmsi),
    imo = COALESCE(EXCLUDED.imo, f.imo),
    reg_num = COALESCE(EXCLUDED.reg_num, f.reg_num),
    sbr_reg_num = COALESCE(EXCLUDED.sbr_reg_num, f.sbr_reg_num),
    contact_phone = COALESCE(EXCLUDED.contact_phone, f.contact_phone),
    contact_email = COALESCE(EXCLUDED.contact_email, f.contact_email),
    tool_type = EXCLUDED.tool_type,
    tool_type_name = COALESCE(EXCLUDED.tool_type_name, f.tool_type_name),
    tool_color = COALESCE(EXCLUDED.tool_color, f.tool_color),
    tool_count = COALESCE(EXCLUDED.tool_count, f.tool_count),
    setup_timestamp = EXCLUDED.setup_timestamp,
    setup_processed_timestamp = COALESCE(
        EXCLUDED.setup_processed_timestamp,
        f.setup_processed_timestamp
    ),
    removed_timestamp = COALESCE(EXCLUDED.removed_timestamp, f.removed_timestamp),
    removed_processed_timestamp = COALESCE(
        EXCLUDED.removed_processed_timestamp,
        f.removed_processed_timestamp
    ),
    last_changed = EXCLUDED.last_changed,
    source = COALESCE(EXCLUDED.source, f.source),
    "comment" = COALESCE(EXCLUDED.comment, f.comment),
    geometry_wkt = EXCLUDED.geometry_wkt,
    api_source = EXCLUDED.api_source
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
            api_source.as_slice(),
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
    fiskeridir_vessel_id,
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
    geometry_wkt AS "geometry_wkt: _",
    api_source AS "api_source!: FishingFacilityApiSource"
FROM
    fishing_facilities
WHERE
    (
        $1::INT[] IS NULL
        OR mmsi = ANY ($1)
    )
    AND (
        $2::BIGINT[] IS NULL
        OR fiskeridir_vessel_id = ANY ($2)
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
ORDER BY
    CASE
        WHEN $7 = 1 THEN CASE
            WHEN $8 = 1 THEN setup_timestamp
            WHEN $8 = 2 THEN removed_timestamp
            WHEN $8 = 3 THEN last_changed
        END
    END ASC,
    CASE
        WHEN $7 = 2 THEN CASE
            WHEN $8 = 1 THEN setup_timestamp
            WHEN $8 = 2 THEN removed_timestamp
            WHEN $8 = 3 THEN last_changed
        END
    END DESC
OFFSET
    $9
LIMIT
    $10
            "#,
            args.mmsis as _,
            args.fiskeridir_vessel_ids as _,
            args.tool_types as _,
            args.active,
            args.setup_ranges,
            args.removed_ranges,
            args.ordering as i32,
            args.sorting as i32,
            args.offset as i64,
            args.limit as i64,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn latest_fishing_facility_update_impl(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    last_changed
FROM
    fishing_facilities
WHERE
    (
        $1::INT IS NULL
        OR api_source = $1
    )
ORDER BY
    last_changed DESC
LIMIT
    1
            "#,
            source.map(|s| s as i32),
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?
        .map(|r| r.last_changed))
    }
}

#[derive(Debug, Clone)]
pub struct FishingFacilitiesArgs {
    pub mmsis: Option<Vec<i32>>,
    pub fiskeridir_vessel_ids: Option<Vec<i64>>,
    pub tool_types: Option<Vec<i32>>,
    pub active: Option<bool>,
    pub setup_ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub removed_ranges: Option<Vec<PgRange<DateTime<Utc>>>>,
    pub limit: u64,
    pub offset: u64,
    pub ordering: Ordering,
    pub sorting: FishingFacilitiesSorting,
}

impl From<FishingFacilitiesQuery> for FishingFacilitiesArgs {
    fn from(v: FishingFacilitiesQuery) -> Self {
        Self {
            mmsis: v.mmsis.map(|ms| ms.into_iter().map(|m| m.0).collect()),
            fiskeridir_vessel_ids: v
                .fiskeridir_vessel_ids
                .map(|fs| fs.into_iter().map(|f| f.0).collect()),
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
            limit: v.pagination.limit(),
            offset: v.pagination.offset(),
            ordering: v.ordering.unwrap_or_default(),
            sorting: v.sorting.unwrap_or_default(),
        }
    }
}
