use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::FishingFacilityToolType;

use crate::{error::PostgresError, models::FishingFacilityHistoric, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn add_fishing_facility_historic_impl(
        &self,
        facilities: Vec<kyogre_core::FishingFacilityHistoric>,
    ) -> Result<(), PostgresError> {
        let len = facilities.len();

        let mut tool_id = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut call_sign = Vec::with_capacity(len);
        let mut mmsi = Vec::with_capacity(len);
        let mut imo = Vec::with_capacity(len);
        let mut reg_num = Vec::with_capacity(len);
        let mut sbr_reg_num = Vec::with_capacity(len);
        let mut tool_type = Vec::with_capacity(len);
        let mut tool_type_name = Vec::with_capacity(len);
        let mut tool_color = Vec::with_capacity(len);
        let mut setup_timestamp = Vec::with_capacity(len);
        let mut removed_timestamp = Vec::with_capacity(len);
        let mut source = Vec::with_capacity(len);
        let mut last_changed = Vec::with_capacity(len);
        let mut comment = Vec::with_capacity(len);
        let mut geometry_wkt = Vec::with_capacity(len);

        for f in facilities {
            tool_id.push(f.tool_id);
            vessel_name.push(f.vessel_name);
            call_sign.push(f.call_sign);
            mmsi.push(f.mmsi.0);
            imo.push(f.imo);
            reg_num.push(f.reg_num);
            sbr_reg_num.push(f.sbr_reg_num);
            tool_type.push(f.tool_type as i32);
            tool_type_name.push(f.tool_type_name);
            tool_color.push(f.tool_color);
            setup_timestamp.push(f.setup_timestamp);
            removed_timestamp.push(f.removed_timestamp);
            source.push(f.source);
            last_changed.push(f.last_changed);
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
    fishing_facilities (
        tool_id,
        vessel_name,
        call_sign,
        mmsi,
        imo,
        reg_num,
        sbr_reg_num,
        tool_type,
        tool_type_name,
        tool_color,
        setup_timestamp,
        removed_timestamp,
        source,
        last_changed,
        "comment",
        geometry_wkt
    )
SELECT
    *
FROM
    UNNEST(
        $1::UUID[],
        $2::TEXT[],
        $3::TEXT[],
        $4::INT[],
        $5::BIGINT[],
        $6::TEXT[],
        $7::TEXT[],
        $8::INT[],
        $9::TEXT[],
        $10::TEXT[],
        $11::TIMESTAMPTZ[],
        $12::TIMESTAMPTZ[],
        $13::TEXT[],
        $14::TIMESTAMPTZ[],
        $15::TEXT[],
        $16::GEOMETRY[]
    )
            "#,
            tool_id.as_slice(),
            vessel_name.as_slice(),
            call_sign.as_slice(),
            mmsi.as_slice(),
            imo.as_slice(),
            reg_num.as_slice() as _,
            sbr_reg_num.as_slice() as _,
            tool_type.as_slice(),
            tool_type_name.as_slice(),
            tool_color.as_slice(),
            setup_timestamp.as_slice(),
            removed_timestamp.as_slice() as _,
            source.as_slice() as _,
            last_changed.as_slice(),
            comment.as_slice() as _,
            geometry_wkt.as_slice() as _,
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) fn fishing_facility_historic_impl(
        &self,
    ) -> impl Stream<Item = Result<FishingFacilityHistoric, PostgresError>> + '_ {
        sqlx::query_as!(
            FishingFacilityHistoric,
            r#"
SELECT
    tool_id,
    vessel_name,
    call_sign,
    mmsi,
    imo,
    reg_num,
    sbr_reg_num,
    tool_type AS "tool_type!: FishingFacilityToolType",
    tool_type_name,
    tool_color,
    setup_timestamp AS "setup_timestamp!",
    removed_timestamp,
    source,
    last_changed AS "last_changed!",
    "comment",
    geometry_wkt AS "geometry_wkt: _"
FROM
    fishing_facilities
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
