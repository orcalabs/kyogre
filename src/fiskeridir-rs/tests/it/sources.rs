use crate::helper::TestHelper;
use fiskeridir_rs::{
    ApiDownloader, ApiSource, AquaCultureEntry, Error, ErsDca, ErsDep, ErsPor, ErsTra,
    FileDownloader, FileSource, Landing, LandingRaw, RegisterVessel, Vms,
};

static ERS_YEAR: u32 = 2020;

#[tokio::test]
async fn download_and_read_ers_dca() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::ErsDca {
        year: ERS_YEAR,
        url: format!("{mock_server_uri}/ers"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<ErsDca>().unwrap();

    let mut result = Vec::new();

    for data in iter {
        result.push(data.unwrap());
    }

    assert_eq!(result.len(), 99);
}

#[tokio::test]
async fn download_and_read_ers_dep() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::ErsDep {
        year: ERS_YEAR,
        url: format!("{mock_server_uri}/ers"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<ErsDep>().unwrap();

    let mut result = Vec::new();

    for data in iter {
        result.push(data.unwrap());
    }

    assert_eq!(result.len(), 99);
}

#[tokio::test]
async fn download_and_read_ers_por() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::ErsPor {
        year: ERS_YEAR,
        url: format!("{mock_server_uri}/ers"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<ErsPor>().unwrap();

    let mut result = Vec::new();

    for data in iter {
        result.push(data.unwrap());
    }

    assert_eq!(result.len(), 99);
}

#[tokio::test]
async fn download_and_read_ers_tra() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::ErsTra {
        year: ERS_YEAR,
        url: format!("{mock_server_uri}/ers"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<ErsTra>().unwrap();

    let mut result = Vec::new();

    for data in iter {
        result.push(data.unwrap());
    }

    assert_eq!(result.len(), 99);
}

#[tokio::test]
async fn download_and_read_vms() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Vms {
        year: 2023,
        url: format!("{mock_server_uri}/vms"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<Vms>().unwrap();

    let mut result = Vec::new();

    for data in iter {
        result.push(data.unwrap());
    }

    test_helper.temp_dir.close().unwrap();

    assert_eq!(result.len(), 500);
}

#[tokio::test]
async fn download_and_read_landings() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Landings {
        year: 2021,
        url: Some(format!("{mock_server_uri}/landings")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile.into_deserialize::<LandingRaw>().unwrap();

    let mut result = Vec::new();

    let mut incomplete_data = 0;
    for data in iter {
        match data {
            Ok(data) => {
                let converted = Landing::try_from_raw(data, source.year());
                result.push(converted);
            }
            Err(e) => {
                if matches!(e.current_context(), Error::IncompleteData) {
                    incomplete_data += 1;
                } else {
                    panic!("{:?}", e);
                }
            }
        }
    }

    test_helper.temp_dir.close().unwrap();

    assert_eq!(incomplete_data, 1);
    assert_eq!(result.len(), 499);
}

#[tokio::test]
async fn download_and_read_register_vessels() {
    let test_helper = TestHelper::new().await;
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = ApiDownloader::new().unwrap();
    let source = ApiSource::RegisterVessels {
        url: format!("{mock_server_uri}/register_vessels"),
    };

    let data: Vec<RegisterVessel> = downloader.download(&source, None::<&()>).await.unwrap();

    test_helper.temp_dir.close().unwrap();

    assert_eq!(data.len(), 50);
}

#[tokio::test]
async fn download_and_read_aqua_culture_register() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = FileDownloader::new(path.into()).unwrap();
    let source = FileSource::AquaCultureRegister {
        url: format!("{mock_server_uri}/aqua_culture_register"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let result: Vec<AquaCultureEntry> = datafile
        .into_deserialize()
        .unwrap()
        .map(|d| d.unwrap())
        .collect();

    test_helper.temp_dir.close().unwrap();

    assert_eq!(result.len(), 500);
}
