use super::float_to_decimal;
use crate::{error::PostgresError, models::VmsPosition, PostgresAdapter};
use error_stack::{report, IntoReport, Result, ResultExt};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::DateRange;

impl PostgresAdapter {
    pub(crate) fn vms_positions_impl(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> impl Stream<Item = Result<VmsPosition, PostgresError>> + '_ {
        sqlx::query_as!(
            VmsPosition,
            r#"
SELECT
    call_sign,
    course,
    latitude,
    longitude,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type
FROM
    vms_positions
WHERE
    call_sign = $1
    AND "timestamp" BETWEEN $2 AND $3
ORDER BY
    "timestamp" ASC
            "#,
            call_sign.as_ref(),
            range.start(),
            range.end(),
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn add_vms_impl(
        &self,
        vms: Vec<fiskeridir_rs::Vms>,
    ) -> Result<(), PostgresError> {
        let len = vms.len();
        let mut call_sign = Vec::with_capacity(len);
        let mut course = Vec::with_capacity(len);
        let mut gross_tonnage = Vec::with_capacity(len);
        let mut latitude = Vec::with_capacity(len);
        let mut longitude = Vec::with_capacity(len);
        let mut message_id = Vec::with_capacity(len);
        let mut message_type = Vec::with_capacity(len);
        let mut message_type_code = Vec::with_capacity(len);
        let mut registration_id = Vec::with_capacity(len);
        let mut speed = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);
        let mut vessel_length = Vec::with_capacity(len);
        let mut vessel_name = Vec::with_capacity(len);
        let mut vessel_type = Vec::with_capacity(len);

        for v in vms.clone() {
            call_sign.push(v.call_sign.into_inner());
            course.push(v.course as i32);
            gross_tonnage.push(v.gross_tonnage as i32);
            latitude
                .push(float_to_decimal(v.latitude).change_context(PostgresError::DataConversion)?);
            longitude
                .push(float_to_decimal(v.longitude).change_context(PostgresError::DataConversion)?);
            message_id.push(v.message_id as i32);
            message_type.push(v.message_type);
            message_type_code.push(v.message_type_code);
            registration_id.push(v.registration_id);
            speed.push(float_to_decimal(v.speed).change_context(PostgresError::DataConversion)?);
            timestamp.push(v.timestamp);
            vessel_length.push(
                float_to_decimal(v.vessel_length).change_context(PostgresError::DataConversion)?,
            );
            vessel_name.push(v.vessel_name);
            vessel_type.push(v.vessel_type);
        }

        sqlx::query!(
            r#"
INSERT INTO
    vms_positions (
        call_sign,
        course,
        gross_tonnage,
        latitude,
        longitude,
        message_id,
        message_type,
        message_type_code,
        registration_id,
        speed,
        "timestamp",
        vessel_length,
        vessel_name,
        vessel_type
    )
SELECT
    *
FROM
    UNNEST(
        $1::VARCHAR[],
        $2::INT[],
        $3::INT[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::INT[],
        $7::VARCHAR[],
        $8::VARCHAR[],
        $9::VARCHAR[],
        $10::DECIMAL[],
        $11::timestamptz[],
        $12::DECIMAL[],
        $13::VARCHAR[],
        $14::VARCHAR[]
    )
            "#,
            call_sign.as_slice(),
            course.as_slice(),
            gross_tonnage.as_slice(),
            latitude.as_slice(),
            longitude.as_slice(),
            message_id.as_slice(),
            message_type.as_slice(),
            message_type_code.as_slice(),
            registration_id.as_slice() as _,
            speed.as_slice(),
            timestamp.as_slice(),
            vessel_length.as_slice(),
            vessel_name.as_slice(),
            vessel_type.as_slice()
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
