use std::path::PathBuf;
use tempfile::{tempdir, TempDir};
use wiremock::{
    matchers::{self, method},
    Mock, MockServer, ResponseTemplate,
};

pub struct TestHelper {
    pub mock_server: MockServer,
    pub temp_dir: TempDir,
}

impl TestHelper {
    pub async fn new() -> TestHelper {
        let mock_server = setup_mock_server().await;
        let temp_dir = tempdir().unwrap();
        TestHelper {
            mock_server,
            temp_dir,
        }
    }
}

async fn setup_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;

    let mut path = PathBuf::new();
    path.push(env!("CARGO_MANIFEST_DIR"));
    path.push("test_data");
    path.push("landings_response.csv.zip");

    let landings_response = std::fs::read(&path).unwrap();

    path.set_file_name("ers_response.zip");
    let ers_response = std::fs::read(&path).unwrap();

    path.set_file_name("vms_response.zip");
    let vms_response = std::fs::read(&path).unwrap();

    path.set_file_name("register_vessels.json");
    let register_vessels = std::fs::read(&path).unwrap();

    path.set_file_name("aqua_culture_register.csv");
    let aqua_culture_register = std::fs::read(&path).unwrap();

    let template1 = ResponseTemplate::new(200).set_body_bytes(landings_response);
    let template2 = ResponseTemplate::new(200).set_body_bytes(ers_response);
    let template3 = ResponseTemplate::new(200).set_body_bytes(vms_response);
    let template4 = ResponseTemplate::new(200).set_body_bytes(register_vessels);
    let template5 = ResponseTemplate::new(200).set_body_bytes(aqua_culture_register);

    Mock::given(method("GET"))
        .and(matchers::path("/landings"))
        .respond_with(template1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(matchers::path("/ers"))
        .respond_with(template2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(matchers::path("/vms"))
        .respond_with(template3)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(matchers::path("/register_vessels"))
        .respond_with(template4)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(matchers::path("/aqua_culture_register"))
        .respond_with(template5)
        .mount(&mock_server)
        .await;

    mock_server
}
