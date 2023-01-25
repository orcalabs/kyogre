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

    async fn get<T: AsRef<str>>(&self, path: T, parameters: &[(String, String)]) -> Response {
        let url = format!("{}/{}", self.address, path.as_ref());

        let client = Client::new();
        let request = client.get(url).query(parameters).build().unwrap();

        client.execute(request).await.unwrap()
    }

    pub async fn get_ais_track(&self, mmsi: i32, params: AisTrackParameters) -> Response {
        let mut url_params = Vec::new();

        if let Some(s) = params.start {
            url_params.push((("start".to_owned()), s.to_string()));
        }

        if let Some(s) = params.end {
            url_params.push((("end".to_owned()), s.to_string()));
        }

        self.get(format!("ais_track/{}", mmsi), url_params.as_slice())
            .await
    }
}
