use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

use chrono::{DateTime, Utc};
use colorgrad::{Gradient, GradientBuilder, LinearGradient};
use error_stack::{report, Report, Result, ResultExt};
use flate2::{write::GzEncoder, Compression};
use futures::{Stream, TryStreamExt};
use image::{ImageFormat, Rgba, RgbaImage};
use kyogre_core::{HaulWeatherOutput, WeatherFeature, WeatherImages, WeatherQuery};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    models::{HaulWeather, NewWeather, Weather, WeatherLocation},
    PostgresAdapter,
};

use super::opt_float_to_decimal;

impl PostgresAdapter {
    pub(crate) fn weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<impl Stream<Item = Result<Weather, PostgresError>> + '_, PostgresError> {
        let args = WeatherArgs::try_from(query)?;

        let stream = sqlx::query_as!(
            Weather,
            r#"
SELECT
    TO_TIMESTAMP(
        AVG(
            EXTRACT(
                epoch
                FROM
                    "timestamp"
            )
        )
    ) AS "timestamp!",
    AVG(latitude) AS "latitude!",
    AVG(longitude) AS "longitude!",
    AVG(altitude) AS "altitude!",
    AVG(wind_speed_10m) AS "wind_speed_10m",
    AVG(wind_direction_10m) AS "wind_direction_10m",
    AVG(air_temperature_2m) AS "air_temperature_2m",
    AVG(relative_humidity_2m) AS "relative_humidity_2m",
    AVG(air_pressure_at_sea_level) AS "air_pressure_at_sea_level",
    AVG(precipitation_amount) AS "precipitation_amount",
    AVG(land_area_fraction) AS "land_area_fraction!",
    AVG(cloud_area_fraction) AS "cloud_area_fraction",
    weather_location_id
FROM
    weather
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR weather_location_id = ANY ($3)
    )
GROUP BY
    weather_location_id
            "#,
            args.start_date,
            args.end_date,
            args.weather_location_ids.as_deref(),
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn weather_image_impl(
        &self,
        timestamp: DateTime<Utc>,
        feature: WeatherFeature,
    ) -> Result<Option<Vec<u8>>, PostgresError> {
        Ok(sqlx::query!(
            r#"
SELECT
    "timestamp",
    CASE
        WHEN $1 = 1 THEN wind_speed_10m
        WHEN $1 = 2 THEN air_temperature_2m
        WHEN $1 = 3 THEN relative_humidity_2m
        WHEN $1 = 4 THEN air_pressure_at_sea_level
        WHEN $1 = 5 THEN precipitation_amount
    END AS "bytes!"
FROM
    weather_images
WHERE
    "timestamp" = $2::TIMESTAMPTZ
            "#,
            feature as i32,
            timestamp,
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)?
        .map(|r| r.bytes))
    }

    pub(crate) async fn weather_images_impl(
        &self,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<WeatherImages>, PostgresError> {
        sqlx::query_as!(
            WeatherImages,
            r#"
SELECT
    "timestamp",
    wind_speed_10m,
    air_temperature_2m,
    relative_humidity_2m,
    air_pressure_at_sea_level,
    precipitation_amount
FROM
    weather_images
WHERE
    "timestamp" = $1::TIMESTAMPTZ
            "#,
            timestamp,
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn haul_weather_impl(
        &self,
        query: WeatherQuery,
    ) -> Result<Option<HaulWeather>, PostgresError> {
        let args = WeatherArgs::try_from(query)?;

        sqlx::query_as!(
            HaulWeather,
            r#"
SELECT
    AVG(wind_speed_10m) AS "wind_speed_10m",
    AVG(wind_direction_10m) AS "wind_direction_10m",
    AVG(air_temperature_2m) AS "air_temperature_2m",
    AVG(relative_humidity_2m) AS "relative_humidity_2m",
    AVG(air_pressure_at_sea_level) AS "air_pressure_at_sea_level",
    AVG(precipitation_amount) AS "precipitation_amount",
    AVG(cloud_area_fraction) AS "cloud_area_fraction"
FROM
    weather
WHERE
    "timestamp" BETWEEN $1::TIMESTAMPTZ AND $2::TIMESTAMPTZ
    AND (
        $3::INT[] IS NULL
        OR weather_location_id = ANY ($3)
    )
            "#,
            args.start_date,
            args.end_date,
            args.weather_location_ids.as_deref(),
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_haul_weather_impl(
        &self,
        values: Vec<HaulWeatherOutput>,
    ) -> Result<(), PostgresError> {
        let len = values.len();
        let mut haul_id = Vec::with_capacity(len);
        let mut wind_speed_10m = Vec::with_capacity(len);
        let mut wind_direction_10m = Vec::with_capacity(len);
        let mut air_temperature_2m = Vec::with_capacity(len);
        let mut relative_humidity_2m = Vec::with_capacity(len);
        let mut air_pressure_at_sea_level = Vec::with_capacity(len);
        let mut precipitation_amount = Vec::with_capacity(len);
        let mut cloud_area_fraction = Vec::with_capacity(len);
        let mut water_speed = Vec::with_capacity(len);
        let mut water_direction = Vec::with_capacity(len);
        let mut salinity = Vec::with_capacity(len);
        let mut water_temperature = Vec::with_capacity(len);
        let mut ocean_climate_depth = Vec::with_capacity(len);
        let mut sea_floor_depth = Vec::with_capacity(len);
        let mut haul_weather_status_id = Vec::with_capacity(len);

        for v in values {
            haul_id.push(v.haul_id.0);
            if let Some(w) = v.weather {
                wind_speed_10m.push(
                    opt_float_to_decimal(w.wind_speed_10m)
                        .change_context(PostgresError::DataConversion)?,
                );
                wind_direction_10m.push(
                    opt_float_to_decimal(w.wind_direction_10m)
                        .change_context(PostgresError::DataConversion)?,
                );
                air_temperature_2m.push(
                    opt_float_to_decimal(w.air_temperature_2m)
                        .change_context(PostgresError::DataConversion)?,
                );
                relative_humidity_2m.push(
                    opt_float_to_decimal(w.relative_humidity_2m)
                        .change_context(PostgresError::DataConversion)?,
                );
                air_pressure_at_sea_level.push(
                    opt_float_to_decimal(w.air_pressure_at_sea_level)
                        .change_context(PostgresError::DataConversion)?,
                );
                precipitation_amount.push(
                    opt_float_to_decimal(w.precipitation_amount)
                        .change_context(PostgresError::DataConversion)?,
                );
                cloud_area_fraction.push(
                    opt_float_to_decimal(w.cloud_area_fraction)
                        .change_context(PostgresError::DataConversion)?,
                );
            } else {
                wind_speed_10m.push(None);
                wind_direction_10m.push(None);
                air_temperature_2m.push(None);
                relative_humidity_2m.push(None);
                air_pressure_at_sea_level.push(None);
                precipitation_amount.push(None);
                cloud_area_fraction.push(None);
            }
            if let Some(o) = v.ocean_climate {
                water_speed.push(
                    opt_float_to_decimal(o.water_speed)
                        .change_context(PostgresError::DataConversion)?,
                );
                water_direction.push(
                    opt_float_to_decimal(o.water_direction)
                        .change_context(PostgresError::DataConversion)?,
                );
                salinity.push(
                    opt_float_to_decimal(o.salinity)
                        .change_context(PostgresError::DataConversion)?,
                );
                water_temperature.push(
                    opt_float_to_decimal(o.water_temperature)
                        .change_context(PostgresError::DataConversion)?,
                );
                ocean_climate_depth.push(
                    opt_float_to_decimal(o.ocean_climate_depth)
                        .change_context(PostgresError::DataConversion)?,
                );
                sea_floor_depth.push(
                    opt_float_to_decimal(o.sea_floor_depth)
                        .change_context(PostgresError::DataConversion)?,
                );
            } else {
                water_speed.push(None);
                water_direction.push(None);
                salinity.push(None);
                water_temperature.push(None);
                ocean_climate_depth.push(None);
                sea_floor_depth.push(None);
            }
            haul_weather_status_id.push(v.status as i32);
        }

        sqlx::query!(
            r#"
UPDATE hauls h
SET
    wind_speed_10m = u.wind_speed_10m,
    wind_direction_10m = u.wind_direction_10m,
    air_temperature_2m = u.air_temperature_2m,
    relative_humidity_2m = u.relative_humidity_2m,
    air_pressure_at_sea_level = u.air_pressure_at_sea_level,
    precipitation_amount = u.precipitation_amount,
    cloud_area_fraction = u.cloud_area_fraction,
    water_speed = u.water_speed,
    water_direction = u.water_direction,
    salinity = u.salinity,
    water_temperature = u.water_temperature,
    ocean_climate_depth = u.ocean_climate_depth,
    sea_floor_depth = u.sea_floor_depth,
    haul_weather_status_id = u.haul_weather_status_id
FROM
    UNNEST(
        $1::BIGINT[],
        $2::DECIMAL[],
        $3::DECIMAL[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::DECIMAL[],
        $7::DECIMAL[],
        $8::DECIMAL[],
        $9::DECIMAL[],
        $10::DECIMAL[],
        $11::DECIMAL[],
        $12::DECIMAL[],
        $13::INT[],
        $14::DECIMAL[],
        $15::INT[]
    ) u (
        haul_id,
        wind_speed_10m,
        wind_direction_10m,
        air_temperature_2m,
        relative_humidity_2m,
        air_pressure_at_sea_level,
        precipitation_amount,
        cloud_area_fraction,
        water_speed,
        water_direction,
        salinity,
        water_temperature,
        ocean_climate_depth,
        sea_floor_depth,
        haul_weather_status_id
    )
WHERE
    h.haul_id = u.haul_id
            "#,
            &haul_id,
            &wind_speed_10m as _,
            &wind_direction_10m as _,
            &air_temperature_2m as _,
            &relative_humidity_2m as _,
            &air_pressure_at_sea_level as _,
            &precipitation_amount as _,
            &cloud_area_fraction as _,
            &water_speed as _,
            &water_direction as _,
            &salinity as _,
            &water_temperature as _,
            &ocean_climate_depth as _,
            &sea_floor_depth as _,
            &haul_weather_status_id,
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn add_weather_impl(
        &self,
        weather: Vec<kyogre_core::NewWeather>,
    ) -> Result<(), PostgresError> {
        let values = weather
            .iter()
            .map(NewWeather::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let mut tx = self.begin().await?;

        NewWeather::unnest_insert(values, &self.pool)
            .await
            .change_context(PostgresError::Query)?;

        self.add_weather_images_impl(weather, &mut tx).await?;

        tx.commit().await.change_context(PostgresError::Query)?;

        Ok(())
    }

    pub(crate) async fn add_weather_images_impl<'a>(
        &'a self,
        weather: Vec<kyogre_core::NewWeather>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        if weather.is_empty() {
            return Ok(());
        }

        let timestamp = weather[0].timestamp;

        // MAGIC NUMBERS!
        // Derived from the bounding box of all weather locations (not including ocean climate
        // locations), multiplied by 10 to get an int instead of float.
        let lat_min: i32 = 523;
        let lat_max: i32 = 738;
        let lon_min: i32 = -118;
        let lon_max: i32 = 417;

        // MAGIC NUMBERS!
        // Size of each weather location in pixels.
        // Derived from visual inspection + reasonable file size.
        // Y is twice as large as X because the locations are, on average, roughly twice as tall as
        // wide.
        let x_mult = 10;
        let y_mult = 20;

        let celcius_kelvin_diff = 273.15;

        // MAGIC NUMBERS!
        // Derived from the min/max values in the dataset as of 2023-9
        let wind_range = (0., 40.);
        let temp_range = (-40. + celcius_kelvin_diff, 40. + celcius_kelvin_diff);
        let hum_range = (0.1, 1.);
        let press_range = (93870., 105600.);
        let prec_range = (0., 1300.);

        let wind_grad = gradient("#bea86d00, #bea86dff, #a64c4c, #9f214a, #460e27");
        let temp_grad =
            gradient("#e5eeff, #99b0d6, #284d7e, #277492, #bea86d, #a64c4c, #9f214a, #460e27");
        let hum_grad = gradient(
            "#f00101, #c64111, #c08541, #76ccbe, #37aeaf, #3b9eaf, #1395a8, #3985ae, #394675",
        );
        // 1 atm = 101325 Pa
        let press_grad = gradient(
            "#012064, #148892, #3499c2 62.555%, #d6d6d7 63.555%, #e0a551 64.555%, #c08541, #973b35",
        );
        let prec_grad = gradient("#3985ae00, #3985aeaa 0.5%, #394675, #012064");

        let width = (lon_max - lon_min + 1) as u32 * x_mult;
        let height = (lat_max - lat_min + 1) as u32 * y_mult;

        let mut wind_img = RgbaImage::new(width, height);
        let mut temp_img = RgbaImage::new(width, height);
        let mut hum_img = RgbaImage::new(width, height);
        let mut press_img = RgbaImage::new(width, height);
        let mut prec_img = RgbaImage::new(width, height);

        let mut weather: HashMap<_, _> = weather
            .into_iter()
            .map(|v| {
                (
                    (
                        (v.latitude * 10.).floor() as i32,
                        (v.longitude * 10.).floor() as i32,
                    ),
                    v,
                )
            })
            .collect();

        let add_pixels = |value: Option<f64>,
                          range: (f64, f64),
                          img: &mut RgbaImage,
                          grad: &LinearGradient,
                          x: u32,
                          y: u32| {
            if let Some(v) = value {
                let at = (v - range.0) / (range.1 - range.0);
                let color = Rgba(grad.at(at as f32).to_rgba8());
                for i in 0..x_mult {
                    for j in 0..y_mult {
                        img.put_pixel(x + i, y + j, color)
                    }
                }
            }
        };

        for (x, lon) in (lon_min..=lon_max).enumerate() {
            for (y, lat) in (lat_min..=lat_max).enumerate() {
                let x = x as u32 * x_mult;
                let y = height - y_mult - (y as u32 * y_mult);

                if let Some(w) = weather.remove(&(lat, lon)) {
                    add_pixels(
                        w.wind_speed_10m,
                        wind_range,
                        &mut wind_img,
                        &wind_grad,
                        x,
                        y,
                    );
                    add_pixels(
                        w.air_temperature_2m,
                        temp_range,
                        &mut temp_img,
                        &temp_grad,
                        x,
                        y,
                    );
                    add_pixels(
                        w.relative_humidity_2m,
                        hum_range,
                        &mut hum_img,
                        &hum_grad,
                        x,
                        y,
                    );
                    add_pixels(
                        w.air_pressure_at_sea_level,
                        press_range,
                        &mut press_img,
                        &press_grad,
                        x,
                        y,
                    );
                    add_pixels(
                        w.precipitation_amount,
                        prec_range,
                        &mut prec_img,
                        &prec_grad,
                        x,
                        y,
                    );
                }
            }
        }

        let process_img = |img: RgbaImage| {
            let mut cursor = Cursor::new(Vec::with_capacity(600_000));
            img.write_to(&mut cursor, ImageFormat::Png).unwrap();

            let mut encoder = GzEncoder::new(Vec::with_capacity(100_000), Compression::new(9));
            encoder
                .write_all(&cursor.into_inner())
                .change_context(PostgresError::DataConversion)?;

            encoder
                .finish()
                .change_context(PostgresError::DataConversion)
        };

        let wind = process_img(wind_img)?;
        let temp = process_img(temp_img)?;
        let hum = process_img(hum_img)?;
        let press = process_img(press_img)?;
        let prec = process_img(prec_img)?;

        sqlx::query!(
            r#"
INSERT INTO
    weather_images (
        "timestamp",
        wind_speed_10m,
        air_temperature_2m,
        relative_humidity_2m,
        air_pressure_at_sea_level,
        precipitation_amount
    )
VALUES
    (
        $1::TIMESTAMPTZ,
        $2::BYTEA,
        $3::BYTEA,
        $4::BYTEA,
        $5::BYTEA,
        $6::BYTEA
    )
ON CONFLICT DO NOTHING
            "#,
            timestamp,
            &wind,
            &temp,
            &hum,
            &press,
            &prec,
        )
        .execute(&mut **tx)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn latest_weather_timestamp_impl(
        &self,
    ) -> Result<Option<DateTime<Utc>>, PostgresError> {
        sqlx::query!(
            r#"
SELECT
    MAX("timestamp") AS ts
FROM
    weather
            "#
        )
        .fetch_one(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|r| r.ts)
    }

    pub(crate) fn weather_locations_impl(
        &self,
    ) -> impl Stream<Item = Result<WeatherLocation, PostgresError>> + '_ {
        sqlx::query_as!(
            WeatherLocation,
            r#"
SELECT
    weather_location_id,
    "polygon" AS "polygon!: _"
FROM
    weather_locations
            "#,
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}

struct WeatherArgs {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub weather_location_ids: Option<Vec<i32>>,
}

impl TryFrom<WeatherQuery> for WeatherArgs {
    type Error = Report<PostgresError>;

    fn try_from(v: WeatherQuery) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            start_date: v.start_date,
            end_date: v.end_date,
            weather_location_ids: v
                .weather_location_ids
                .map(|ids| ids.into_iter().map(|id| id.0).collect()),
        })
    }
}

fn gradient(colors: &'static str) -> LinearGradient {
    GradientBuilder::new().css(colors).build().unwrap()
}
