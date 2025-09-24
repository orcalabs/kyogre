use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
    jwk::{AlgorithmParameters, CommonParameters, Jwk, JwkSet, KeyAlgorithm, RSAKeyParameters},
};
use kyogre_core::{BarentswatchUserId, TEST_SIGNED_IN_VESSEL_CALLSIGN};
use rsa::{
    RsaPrivateKey, RsaPublicKey, pkcs1::EncodeRsaPrivateKey, pkcs8::LineEnding,
    traits::PublicKeyParts,
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use web_api::extractors::{AcceptedIssuer, BwPolicy, BwProfile, BwRole, BwUser, BwVesselInfo};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

pub struct BarentswatchHelper {
    mock_server: MockServer,
    private_key: RsaPrivateKey,
    pub audience: String,
}

impl BarentswatchHelper {
    pub async fn new() -> Self {
        let (private_key, public_key) = {
            let s = include_str!("../test_private_key.json");
            let private_key = serde_json::from_str(s).unwrap();
            let public_key = RsaPublicKey::from(&private_key);
            (private_key, public_key)
        };

        let jwk = Jwk {
            common: CommonParameters {
                key_id: Some("TEST_KEY_ID".into()),
                key_algorithm: Some(KeyAlgorithm::RS256),
                ..Default::default()
            },
            algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                n: URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
                e: URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
                ..Default::default()
            }),
        };

        let decoding_key = DecodingKey::from_jwk(&jwk).unwrap();
        let audience = "test".to_string();

        let jwks = JwkSet { keys: vec![jwk] };

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(jwks))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/profiles"))
            .respond_with({
                let audience = audience.clone();
                move |req: &wiremock::Request| {
                    let auth = &req.headers.get("Authorization").unwrap();

                    let mut parts = auth.to_str().unwrap().split(' ');
                    parts.next();
                    let token = parts.next().unwrap();

                    let mut validation = Validation::new(Algorithm::RS256);
                    validation.set_audience(&[&audience]);

                    let decoded = decode::<Claims>(token, &decoding_key, &validation).unwrap();

                    let profile = BwProfile {
                        user: BwUser {
                            id: decoded.claims.id,
                            email: Some("test_email@orcalabs.no".to_string()),
                        },
                        fisk_info_profile: Some(BwVesselInfo {
                            ircs: Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap()),
                        }),
                        policies: decoded.claims.policies,
                        roles: decoded.claims.roles,
                    };

                    ResponseTemplate::new(200).set_body_json(profile)
                }
            })
            .mount(&mock_server)
            .await;

        Self {
            mock_server,
            private_key,
            audience,
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
            id: BarentswatchUserId::test_new(),
            exp: i64::MAX,
            aud: self.audience.clone(),
            policies: BwPolicy::iter().collect(),
            roles: BwRole::iter().collect(),
            iss: AcceptedIssuer::Barentswatch.to_string(),
        };
        self.get_bw_token_impl(claims)
    }

    pub fn get_bw_token_with_full_ais_permission(&self) -> String {
        let claims = Claims {
            iss: AcceptedIssuer::Barentswatch.to_string(),
            id: BarentswatchUserId::test_new(),
            exp: i64::MAX,
            aud: self.audience.clone(),
            policies: vec![BwPolicy::BwAisFiskinfo],
            roles: vec![
                BwRole::BwDownloadFishingfacility,
                BwRole::BwEksternFiskInfoUtvikler,
                BwRole::BwFiskerikyndig,
                BwRole::BwFiskinfoAdmin,
                BwRole::BwUtdanningsBruker,
                BwRole::BwViewAis,
                BwRole::BwYrkesfisker,
            ],
        };
        self.get_bw_token_impl(claims)
    }

    pub fn get_bw_token_with_policies_and_roles(
        &self,
        policies: Vec<BwPolicy>,
        roles: Vec<BwRole>,
    ) -> String {
        let claims = Claims {
            iss: AcceptedIssuer::Barentswatch.to_string(),
            id: BarentswatchUserId::test_new(),
            exp: i64::MAX,
            aud: self.audience.clone(),
            policies,
            roles,
        };
        self.get_bw_token_impl(claims)
    }

    pub fn get_bw_token_with_policies(&self, policies: Vec<BwPolicy>) -> String {
        let claims = Claims {
            iss: AcceptedIssuer::Barentswatch.to_string(),
            id: BarentswatchUserId::test_new(),
            exp: i64::MAX,
            aud: self.audience.clone(),
            policies,
            roles: vec![],
        };
        self.get_bw_token_impl(claims)
    }

    pub fn address(&self) -> String {
        format!("http://{}", self.mock_server.address())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    iss: String,
    id: BarentswatchUserId,
    exp: i64,
    aud: String,
    policies: Vec<BwPolicy>,
    roles: Vec<BwRole>,
}
