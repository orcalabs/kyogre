use crate::{utils::hash_file, Error};

use csv::DeserializeRecordsIntoIter;
use error_stack::{report, Result, ResultExt};
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use std::{fmt::Display, io::Write, path::PathBuf};

#[derive(Debug, Clone)]
pub struct DataDownloader {
    // Path to directory where file will be downloaded
    directory_path: PathBuf,
    // HTTP client instance
    http_client: reqwest::Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    dir_path: PathBuf,
}

pub struct FiskeridirRecordIter<R, D> {
    inner: DeserializeRecordsIntoIter<R, D>,
}

impl<R: std::io::Read, D: DeserializeOwned> Iterator for FiskeridirRecordIter<R, D> {
    type Item = Result<D, Error>;
    fn next(&mut self) -> Option<Result<D, Error>> {
        self.inner.next().map(|r| {
            r.map_err(|e| match e.kind() {
                csv::ErrorKind::Deserialize { pos: _, err } => match err.kind() {
                    csv::DeserializeErrorKind::UnexpectedEndOfRow => {
                        report!(e).change_context(Error::IncompleteData)
                    }
                    _ => report!(e).change_context(Error::Deserialize),
                },
                _ => report!(e).change_context(Error::Deserialize),
            })
        })
    }
}

impl DataDir {
    pub fn into_deserialize<T: DeserializeOwned + 'static>(
        self,
        file: &DataFile,
    ) -> Result<FiskeridirRecordIter<std::fs::File, T>, Error> {
        let file = std::fs::File::open(self.file_name(file)).change_context(Error::Deserialize)?;

        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .trim(csv::Trim::Fields)
            .flexible(true)
            .from_reader(file);

        Ok(FiskeridirRecordIter {
            inner: csv_reader.into_deserialize(),
        })
    }

    pub fn hash(&self, file: &DataFile) -> Result<String, Error> {
        hash_file(&self.file_name(file)).change_context(Error::Hash)
    }

    fn file_name(&self, file: &DataFile) -> PathBuf {
        self.dir_path.join(file.name())
    }
}

// Different sources within Fiskeridir
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSource {
    Landings { year: u32, url: Option<String> },
    Vms { year: u32, url: String },
    Ers { year: u32, url: Option<String> },
    AquaCultureRegister { url: String },
}

// Different files within Fiskeridir data sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFile {
    Landings { year: u32 },
    Vms { year: u32 },
    ErsDca { year: u32 },
    ErsPor { year: u32 },
    ErsDep { year: u32 },
    ErsTra { year: u32 },
    AquaCultureRegister,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct DataFileId(String);

impl FileSource {
    fn archive_name(&self) -> String {
        format!("{}.zip", self.extract_dir_name())
    }

    fn extract_dir_name(&self) -> String {
        use FileSource::*;

        match *self {
            Landings { year, .. } => format!("{year}-Landings"),
            Vms { year, .. } => format!("{year}-Vms"),
            Ers { year, .. } => format!("{year}-Ers"),
            AquaCultureRegister { .. } => "AquaCultureRegister".into(),
        }
    }

    fn url(&self) -> String {
        use FileSource::*;

        match self {
            Landings { year, url } => match url {
                Some(url) => url.clone(),
                None => format!("https://register.fiskeridir.no/uttrekk/fangstdata_{year}.csv.zip"),
            },
            Ers {year, url} => match url {
                Some(url) => url.clone(),
                None => format!("https://register.fiskeridir.no/vms-ers/ERS/elektronisk-rapportering-ers-{year}.zip"),
            },
            Vms { url, .. } => url.clone(),
            AquaCultureRegister { url } => url.clone(),
        }
    }

    pub fn year(&self) -> u32 {
        use FileSource::*;

        match *self {
            Landings { year, .. } | Vms { year, .. } | Ers { year, .. } => year,
            AquaCultureRegister { .. } => 0,
        }
    }

    pub fn files(&self) -> Vec<DataFile> {
        use FileSource::*;

        match *self {
            Landings { year, .. } => vec![DataFile::Landings { year }],
            Vms { year, .. } => vec![DataFile::Vms { year }],
            Ers { year, .. } => vec![
                DataFile::ErsDca { year },
                DataFile::ErsDep { year },
                DataFile::ErsPor { year },
                DataFile::ErsTra { year },
            ],
            AquaCultureRegister { .. } => vec![DataFile::AquaCultureRegister],
        }
    }
}

impl DataFile {
    pub fn id(&self) -> DataFileId {
        use DataFile::*;

        match self {
            Landings { year } => DataFileId(format!("landings_{year}")),
            ErsDca { year } => DataFileId(format!("ers_dca_{year}")),
            ErsDep { year } => DataFileId(format!("ers_dep_{year}")),
            ErsPor { year } => DataFileId(format!("ers_por_{year}")),
            ErsTra { year } => DataFileId(format!("ers_tra_{year}")),
            Vms { year } => DataFileId(format!("vms_{year}")),
            AquaCultureRegister => DataFileId("aqua_culture_register".into()),
        }
    }

    // Returns name of the file within the zip archive.
    fn name(&self) -> String {
        use DataFile::*;

        match *self {
            Landings { year } => format!("fangstdata_{year}.csv"),
            Vms { year } => match year {
                y if y >= 2022 => format!("{y}-VMS.csv"),
                y => format!("VMS_{y}.csv"),
            },
            ErsDca { year } => format!("elektronisk-rapportering-ers-{year}-fangstmelding-dca.csv"),
            ErsPor { year } => {
                format!("elektronisk-rapportering-ers-{year}-ankomstmelding-por.csv")
            }
            ErsDep { year } => {
                format!("elektronisk-rapportering-ers-{year}-avgangsmelding-dep.csv")
            }
            ErsTra { year } => {
                format!("elektronisk-rapportering-ers-{year}-overforingsmelding-tra.csv")
            }
            AquaCultureRegister => "AquaCultureRegister.csv".into(),
        }
    }

    pub fn year(&self) -> u32 {
        use DataFile::*;

        match *self {
            Landings { year, .. }
            | Vms { year, .. }
            | ErsDca { year, .. }
            | ErsDep { year, .. }
            | ErsPor { year, .. }
            | ErsTra { year, .. } => year,
            AquaCultureRegister { .. } => 0,
        }
    }
}

impl Display for FileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FileSource::*;

        match self {
            Landings { .. } => write!(f, "landings"),
            Ers { .. } => write!(f, "ers"),
            Vms { .. } => write!(f, "vms"),
            AquaCultureRegister { .. } => write!(f, "aqua_culture_register"),
        }
    }
}

impl Display for DataFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DataFile::*;

        match self {
            Landings { .. } => write!(f, "landings"),
            ErsDca { .. } => write!(f, "ers_dca"),
            ErsDep { .. } => write!(f, "ers_dep"),
            ErsPor { .. } => write!(f, "ers_por"),
            ErsTra { .. } => write!(f, "ers_tra"),
            Vms { .. } => write!(f, "vms"),
            AquaCultureRegister => write!(f, "aqua_culture_register"),
        }
    }
}

impl AsRef<str> for DataFileId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl DataDownloader {
    pub fn new(directory_path: PathBuf) -> Result<DataDownloader, Error> {
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::new(600, 0))
            .build()
            .change_context(Error::Download)?;

        Ok(DataDownloader {
            directory_path,
            http_client: client,
        })
    }

    pub fn clean_download_dir(&self) -> Result<(), std::io::Error> {
        std::fs::remove_dir_all(&self.directory_path)?;
        std::fs::create_dir(&self.directory_path)?;
        Ok(())
    }
    pub async fn download(&self, source: &FileSource) -> Result<DataDir, Error> {
        let url = reqwest::Url::parse(&source.url()).change_context(Error::Download)?;

        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .change_context(Error::Download)?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(report!(Error::Download)
                .attach_printable(format!("received response status {}", response.status())));
        }

        let file_path = match source {
            FileSource::Landings { .. } | FileSource::Vms { .. } | FileSource::Ers { .. } => {
                let mut zipfile_path = PathBuf::from(&self.directory_path);
                zipfile_path.push(source.archive_name());

                let mut file =
                    std::fs::File::create(&zipfile_path).change_context(Error::Download)?;

                let mut stream = response.bytes_stream();

                while let Some(item) = stream.next().await {
                    file.write_all(&item.change_context(Error::Download)?)
                        .change_context(Error::Download)?;
                }

                // Unpack zip file
                let file = std::fs::File::open(&zipfile_path).change_context(Error::Download)?;

                let mut archive = zip::ZipArchive::new(file).change_context(Error::Download)?;

                let extract_path =
                    PathBuf::from(&self.directory_path.join(source.extract_dir_name()));

                archive
                    .extract(&extract_path)
                    .change_context(Error::Download)?;

                extract_path
            }
            FileSource::AquaCultureRegister { .. } => {
                let path = &self
                    .directory_path
                    .join(DataFile::AquaCultureRegister.name());

                let mut file = std::fs::File::create(path).change_context(Error::Download)?;

                let text = response.text().await.change_context(Error::Download)?;

                // This file contains an extra line at the beginning that we need to skip:
                // 'AKVAKULTURTILLATELSER PR. 09-08-2023 ;;;;;;;;;;;;;;;;;;;;;;;;;;'
                if let Some((_, text)) = text.split_once('\n') {
                    file.write_all(text.as_bytes())
                        .change_context(Error::Download)?;
                }

                self.directory_path.clone()
            }
        };

        Ok(DataDir {
            dir_path: file_path,
        })
    }
}
