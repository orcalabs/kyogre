use crate::error::Result;
use crate::protobuf::{matrix_cache_client::MatrixCacheClient, *};
use async_trait::async_trait;
use kyogre_core::retry;
use kyogre_core::{CoreResult, HaulsMatrixQuery, LandingMatrixQuery, MatrixCacheOutbound};
use std::time::Duration;
use tonic::codegen::CompressionEncoding;
use tracing::instrument;

#[derive(Clone)]
pub struct Client {
    inner: MatrixCacheClient<tonic::transport::Channel>,
}

impl Client {
    pub async fn new(ip: impl AsRef<str>, port: u16) -> Result<Client> {
        let addr = tonic::transport::Uri::try_from(format!("http://{}:{port}", ip.as_ref()))?;

        let channel = tonic::transport::Channel::builder(addr)
            .timeout(Duration::from_secs(5))
            .http2_keep_alive_interval(Duration::from_secs(5))
            .keep_alive_while_idle(true)
            .connect_lazy();

        Ok(Client {
            inner: MatrixCacheClient::new(channel).accept_compressed(CompressionEncoding::Gzip),
        })
    }

    // Only used for test purposes
    pub async fn refresh(&self) -> Result<()> {
        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        Ok(client.refresh(EmptyMessage {}).await.map(|_| ())?)
    }

    async fn landing_matrix_impl(
        &self,
        query: LandingMatrixQuery,
    ) -> Result<kyogre_core::LandingMatrix> {
        let active_filter = query.active_filter;
        let parameters = LandingFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client.get_landing_matrix(parameters).await?.into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(kyogre_core::LandingMatrix::empty(active_filter))
        } else {
            Ok(kyogre_core::LandingMatrix::from(matrix))
        }
    }

    async fn hauls_matrix_impl(&self, query: HaulsMatrixQuery) -> Result<kyogre_core::HaulsMatrix> {
        let active_filter = query.active_filter;
        let parameters = HaulFeatures::from(query);

        // Cloning a channel is cheap see
        // https://docs.rs/tonic/latest/tonic/transport/struct.Channel.html for more
        // explanation.
        let mut client = self.inner.clone();

        let matrix = client.get_haul_matrix(parameters).await?.into_inner();

        if matrix.dates.is_empty()
            || matrix.gear_group.is_empty()
            || matrix.length_group.is_empty()
            || matrix.species_group.is_empty()
        {
            Ok(kyogre_core::HaulsMatrix::empty(active_filter))
        } else {
            Ok(kyogre_core::HaulsMatrix::from(matrix))
        }
    }
}

#[async_trait]
impl MatrixCacheOutbound for Client {
    #[instrument(name = "cache_landing_matrix", skip(self))]
    async fn landing_matrix(
        &self,
        query: &LandingMatrixQuery,
    ) -> CoreResult<kyogre_core::LandingMatrix> {
        Ok(retry(|| self.landing_matrix_impl(query.clone())).await?)
    }
    #[instrument(name = "cache_hauls_matrix", skip(self))]
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<kyogre_core::HaulsMatrix> {
        Ok(retry(|| self.hauls_matrix_impl(query.clone())).await?)
    }
}
