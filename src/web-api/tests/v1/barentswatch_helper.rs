use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{
    decode, encode,
    jwk::{AlgorithmParameters, CommonParameters, Jwk, JwkSet, RSAKeyParameters},
    Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use rsa::{
    pkcs1::EncodeRsaPrivateKey, pkcs8::LineEnding, traits::PublicKeyParts, RsaPrivateKey,
    RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use web_api::extractors::{BwPolicy, BwProfile};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub struct BarentswatchHelper {
    mock_server: MockServer,
    private_key: RsaPrivateKey,
}

impl BarentswatchHelper {
    pub async fn new() -> Self {
        let (private_key, public_key) = {
            let mut rng = rand::thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
            let public_key = RsaPublicKey::from(&private_key);
            (private_key, public_key)
        };

        let jwk = Jwk {
            common: CommonParameters {
                key_id: Some("TEST_KEY_ID".into()),
                algorithm: Some(Algorithm::RS256),
                ..Default::default()
            },
            algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                n: URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
                e: URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
                ..Default::default()
            }),
        };

        let decoding_key = DecodingKey::from_jwk(&jwk).unwrap();

        let jwks = JwkSet { keys: vec![jwk] };

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(jwks))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/profiles"))
            .respond_with(move |req: &wiremock::Request| {
                let auth = &req
                    .headers
                    .get(&"Authorization".try_into().unwrap())
                    .unwrap()[0];

                let mut parts = auth.as_str().split(' ');
                parts.next();
                let token = parts.next().unwrap();

                let decoded =
                    decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::RS256))
                        .unwrap();

                let profile = BwProfile {
                    policies: decoded.claims.policies,
                };

                ResponseTemplate::new(200).set_body_json(profile)
            })
            .mount(&mock_server)
            .await;

        Self {
            mock_server,
            private_key,
        }
    }

    fn get_bw_token_impl(&self, claims: Claims) -> String {
        let pem = self.private_key.to_pkcs1_pem(LineEnding::LF).unwrap();

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("TEST_KEY_ID".into());

        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();

        encode(&header, &claims, &encoding_key).unwrap()
    }

    pub fn get_bw_token(&self) -> String {
        let claims = Claims {
            exp: i64::MAX,
            policies: BwPolicy::iter().collect(),
        };
        self.get_bw_token_impl(claims)
    }

    pub fn get_bw_token_with_policies(&self, policies: Vec<BwPolicy>) -> String {
        let claims = Claims {
            exp: i64::MAX,
            policies,
        };
        self.get_bw_token_impl(claims)
    }

    pub fn address(&self) -> String {
        format!("http://{}", self.mock_server.address())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    exp: i64,
    policies: Vec<BwPolicy>,
}