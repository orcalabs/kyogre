use crate::helper::TestHelper;
use fiskeridir_rs::{
    ApiDownloader, ApiSource, AquaCultureEntry, DataDownloader, DataFile, Error, ErsDca, ErsDep,
    ErsPor, ErsTra, FileSource, Landing, LandingRaw, RegisterVessel, Vms,
};

static ERS_YEAR: u32 = 2020;

#[tokio::test]
async fn download_and_read_ers_dca() {
    let test_helper = TestHelper::new().await;
    let path = test_helper.temp_dir.path();
    let mock_server_uri = test_helper.mock_server.uri();

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Ers {
        year: ERS_YEAR,
        url: Some(format!("{mock_server_uri}/ers")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<ErsDca>(&DataFile::ErsDca {
            year: source.year(),
        })
        .unwrap();

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

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Ers {
        year: ERS_YEAR,
        url: Some(format!("{mock_server_uri}/ers")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<ErsDep>(&DataFile::ErsDep {
            year: source.year(),
        })
        .unwrap();

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

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Ers {
        year: ERS_YEAR,
        url: Some(format!("{mock_server_uri}/ers")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<ErsPor>(&DataFile::ErsPor {
            year: source.year(),
        })
        .unwrap();

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

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Ers {
        year: ERS_YEAR,
        url: Some(format!("{mock_server_uri}/ers")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<ErsTra>(&DataFile::ErsTra {
            year: source.year(),
        })
        .unwrap();

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

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Vms {
        year: 2023,
        url: format!("{mock_server_uri}/vms"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<Vms>(&source.files()[0])
        .unwrap();

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

    let downloader = DataDownloader::new(path.to_path_buf()).unwrap();
    let source = FileSource::Landings {
        year: 2021,
        url: Some(format!("{mock_server_uri}/landings")),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let iter = datafile
        .into_deserialize::<LandingRaw>(&source.files()[0])
        .unwrap();

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

    let downloader = DataDownloader::new(path.into()).unwrap();
    let source = FileSource::AquaCultureRegister {
        url: format!("{mock_server_uri}/aqua_culture_register"),
    };

    let datafile = downloader.download(&source).await.unwrap();

    let result: Vec<AquaCultureEntry> = datafile
        .into_deserialize(&source.files()[0])
        .unwrap()
        .map(|d| d.unwrap())
        .collect();

    test_helper.temp_dir.close().unwrap();

    assert_eq!(result.len(), 500);
}
