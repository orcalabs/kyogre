use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use http_client::{AUTHORIZATION, HttpClient};
use kyogre_core::{
    AisMigratorSource, BearerToken, CoreResult, Mmsi, NavigationStatus, OauthConfig,
    core_error::UnexpectedSnafu, distance_to_shore,
};
use postgres::PostgresAdapter;
use serde::Deserialize;
use stack_error::OpaqueError;

#[derive(Debug, Deserialize)]
pub struct BarentswatchSettings {
    pub auth_url: String,
    pub token_url: String,
    pub ais_positions_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Clone)]
pub struct BarentswatchAdapter {
    destination: PostgresAdapter,
    client: HttpClient,
    ais_positions_url: String,
    token: BearerToken,
}

// API Documentation: `https://historic.ais.barentswatch.no/index.html`
impl BarentswatchAdapter {
    pub async fn new(destination: PostgresAdapter, settings: &BarentswatchSettings) -> Self {
        let token = BearerToken::acquire(&OauthConfig {
            client_secret: settings.client_secret.clone(),
            client_id: settings.client_id.clone(),
            auth_url: settings.auth_url.clone(),
            token_url: settings.token_url.clone(),
            scope: "ais".into(),
        })
        .await
        .unwrap();

        Self {
            destination,
            client: Default::default(),
            ais_positions_url: settings.ais_positions_url.clone(),
            token,
        }
    }
}

#[async_trait]
impl AisMigratorSource for BarentswatchAdapter {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CoreResult<Vec<kyogre_core::AisPosition>> {
        let response = self
            .client
            .get(format!(
                "{}/{}/{}/{}",
                self.ais_positions_url,
                mmsi,
                start.to_rfc3339(),
                end.to_rfc3339(),
            ))
            .header(AUTHORIZATION, format!("bearer {}", self.token.as_ref()))
            .send()
            .await
            .map_err(|e| {
                UnexpectedSnafu {
                    opaque: OpaqueError::Std(Box::new(e)),
                }
                .build()
            })?;

        let mut positions: Vec<AisPosition> = response.json().await.map_err(|e| {
            UnexpectedSnafu {
                opaque: OpaqueError::Std(Box::new(e)),
            }
            .build()
        })?;

        // The API documentation does not specify any ordering, so we manually sort the positions
        // to make sure the downsampling is works correctly.
        positions.sort_by_key(|v| v.msgtime);

        // Additionally, the Barentswatch API does not have any options for downsampling like the live API does,
        // so we have to manually downsample the data.
        let downsample_duration = Duration::minutes(1);

        Ok(positions.into_iter().fold(Vec::new(), |mut state, pos| {
            if state
                .last()
                .is_none_or(|v| pos.msgtime - v.msgtime > downsample_duration)
            {
                state.push(pos.into());
            }
            state
        }))
    }
    async fn existing_mmsis(&self) -> CoreResult<Vec<Mmsi>> {
        // The Barentswatch API does not provide any endpoints for getting all existing mmsis,
        // so we have to get them from the destination database.
        // This assumes that the destination contains AIS data and this data migration
        // is just run to patch a hole.
        self.destination.existing_mmsis().await
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub mmsi: Mmsi,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
}

impl From<AisPosition> for kyogre_core::AisPosition {
    fn from(value: AisPosition) -> Self {
        let AisPosition {
            latitude,
            longitude,
            mmsi,
            msgtime,
            course_over_ground,
            navigational_status,
            rate_of_turn,
            speed_over_ground,
            true_heading,
        } = value;

        Self {
            latitude,
            longitude,
            mmsi,
            msgtime,
            course_over_ground,
            navigational_status,
            rate_of_turn,
            speed_over_ground,
            true_heading,
            distance_to_shore: distance_to_shore(latitude, longitude),
        }
    }
}
