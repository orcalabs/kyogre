use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use http_client::{AUTHORIZATION, Error, HttpClient, StatusCode};
use kyogre_core::{
    AisMigratorSource, BearerToken, CoreResult, Mmsi, NavigationStatus, OauthConfig,
    core_error::UnexpectedSnafu, distance_to_shore,
};
use postgres::PostgresAdapter;
use serde::Deserialize;
use stack_error::OpaqueError;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
pub struct BarentswatchSettings {
    pub ais_positions_url: String,
    pub oauth: OauthConfig,
}

#[derive(Clone)]
pub struct BarentswatchAdapter {
    inner: Arc<Inner>,
}

pub struct Inner {
    destination: PostgresAdapter,
    client: HttpClient,
    settings: BarentswatchSettings,
    token: RwLock<BearerToken>,
}

// API Documentation: `https://historic.ais.barentswatch.no/index.html`
impl BarentswatchAdapter {
    pub async fn new(destination: PostgresAdapter, settings: BarentswatchSettings) -> Self {
        let token = BearerToken::acquire(&settings.oauth).await.unwrap();

        Self {
            inner: Arc::new(Inner {
                destination,
                client: Default::default(),
                settings,
                token: RwLock::new(token),
            }),
        }
    }

    async fn get_ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AisPosition>, http_client::Error> {
        let f = |login| async move {
            let token = {
                if login {
                    let token = BearerToken::acquire(&self.inner.settings.oauth)
                        .await
                        .unwrap();
                    let mut guard = self.inner.token.write().await;
                    *guard = token;
                    format!("bearer {}", guard.as_ref())
                } else {
                    let token = self.inner.token.read().await;
                    format!("bearer {}", token.as_ref())
                }
            };

            self.inner
                .client
                .get(format!(
                    "{}/{}/{}/{}",
                    self.inner.settings.ais_positions_url,
                    mmsi,
                    start.to_rfc3339(),
                    end.to_rfc3339(),
                ))
                .header(AUTHORIZATION, token)
                .send()
                .await?
                .json()
                .await
        };

        match f(false).await {
            Ok(v) => Ok(v),
            Err(Error::FailedRequest {
                status: StatusCode::UNAUTHORIZED,
                ..
            }) => f(true).await,
            Err(e) => Err(e),
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
        let mut positions = self
            .get_ais_positions(mmsi, start, end)
            .await
            .map_err(|e| {
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
        self.inner.destination.existing_mmsis().await
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
