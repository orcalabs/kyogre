use crate::oauth_error::ExchangeCredentialsSnafu;
use crate::OauthError;
use oauth2::reqwest;
use oauth2::AccessToken;
use oauth2::TokenResponse;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, Scope, TokenUrl};
use serde::Deserialize;
use snafu::ResultExt;

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
    pub async fn acquire(config: &OauthConfig) -> Result<BearerToken, OauthError> {
        let auth_url = AuthUrl::new(config.auth_url.clone())?;

        let token_url = TokenUrl::new(config.token_url.clone())?;

        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        let response = client
            .exchange_client_credentials()
            .add_scope(Scope::new(config.scope.clone()))
            .request_async(&reqwest::Client::new())
            .await
            .boxed()
            .context(ExchangeCredentialsSnafu)?;

        Ok(BearerToken(response.access_token().clone()))
    }
}

impl AsRef<str> for BearerToken {
    fn as_ref(&self) -> &str {
        self.0.secret()
    }
}
