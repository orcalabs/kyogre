use crate::error::BearerTokenError;
use error_stack::{IntoReport, Result, ResultExt};
use oauth2::AccessToken;
use oauth2::TokenResponse;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, ClientId, ClientSecret, Scope,
    TokenUrl,
};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct OauthConfig {
    pub client_secret: String,
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub scope: String,
}

pub struct BearerToken(AccessToken);

impl BearerToken {
    pub async fn acquire(config: &OauthConfig) -> Result<BearerToken, BearerTokenError> {
        let auth_url = AuthUrl::new(config.auth_url.clone())
            .into_report()
            .change_context(BearerTokenError::Acquisition)?;

        let token_url = TokenUrl::new(config.token_url.clone())
            .into_report()
            .change_context(BearerTokenError::Acquisition)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            auth_url,
            Some(token_url),
        );

        let response = client
            .exchange_client_credentials()
            .add_scope(Scope::new(config.scope.clone()))
            .request_async(async_http_client)
            .await
            .into_report()
            .change_context(BearerTokenError::Acquisition)?;

        Ok(BearerToken(response.access_token().clone()))
    }
}

impl AsRef<str> for BearerToken {
    fn as_ref(&self) -> &str {
        self.0.secret()
    }
}
