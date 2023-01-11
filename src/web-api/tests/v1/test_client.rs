use reqwest::{Client, Response};
use web_api::routes::v1::ais::AisTrackParameters;

#[derive(Debug, Clone)]
pub struct ApiClient {
    address: String,
}

impl ApiClient {
    pub fn new(address: String) -> ApiClient {
        ApiClient { address }
    }

    async fn get(&self, path: &str, parameters: &[(String, String)]) -> Response {
        let url = format!("{}/{}", self.address, path);

        let client = Client::new();
        let request = client.get(url).query(parameters).build().unwrap();

        client.execute(request).await.unwrap()
    }

    pub async fn get_ais_track(&self, params: AisTrackParameters) -> Response {
        let url_params = vec![
            ("start".to_owned(), params.start.to_string()),
            ("end".to_owned(), params.end.to_string()),
            ("mmsi".to_owned(), params.mmsi.to_string()),
        ];

        self.get("ais_track", url_params.as_slice()).await
    }
}
