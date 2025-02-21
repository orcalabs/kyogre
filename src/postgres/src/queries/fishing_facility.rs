use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{
    EmptyVecToNone, FishingFacilitiesQuery, FishingFacility, FishingFacilityApiSource,
    FishingFacilityToolType, FiskeridirVesselId, Mmsi, Range,
};

use crate::{
    PostgresAdapter,
    error::{ConvertSnafu, Result},
};

impl PostgresAdapter {
    pub(crate) async fn add_fishing_facilities_impl(
        &self,
        facilities: Vec<kyogre_core::FishingFacility>,
    ) -> Result<()> {
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
            mmsi.push(f.mmsi);
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

            let geometry = f
                .geometry_wkt
                .map(|v| Geometry::<f64>::try_from(v.0).map(wkb::Encode))
                .transpose()
                .map_err(|e| {
                    ConvertSnafu {
                        stringified_error: e.to_string(),
                    }
                    .build()
                })?;
            geometry_wkt.push(geometry);
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
        $1::UUID [],
        $2::UUID [],
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
        .await?;

        Ok(())
    }

    pub(crate) fn fishing_facilities_impl(
        &self,
        query: FishingFacilitiesQuery,
    ) -> impl Stream<Item = Result<FishingFacility>> + '_ {
        sqlx::query_as!(
            FishingFacility,
            r#"
SELECT
    tool_id,
    barentswatch_vessel_id,
    fiskeridir_vessel_id AS "fiskeridir_vessel_id?: FiskeridirVesselId",
    vessel_name,
    call_sign AS "call_sign: CallSign",
    mmsi AS "mmsi?: Mmsi",
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
            query.mmsis.empty_to_none() as Option<Vec<Mmsi>>,
            query.fiskeridir_vessel_ids.empty_to_none() as Option<Vec<FiskeridirVesselId>>,
            query.tool_types.empty_to_none() as Option<Vec<FishingFacilityToolType>>,
            query.active,
            query.setup_ranges.empty_to_none() as Option<Vec<Range<DateTime<Utc>>>>,
            query.removed_ranges.empty_to_none() as Option<Vec<Range<DateTime<Utc>>>>,
            query.ordering.unwrap_or_default() as i32,
            query.sorting.unwrap_or_default() as i32,
            query.pagination.offset() as i64,
            query.pagination.limit() as i64,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn latest_fishing_facility_update_impl(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> Result<Option<DateTime<Utc>>> {
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
        .await?
        .map(|r| r.last_changed))
    }
}
