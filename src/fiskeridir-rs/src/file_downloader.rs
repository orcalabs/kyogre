use crate::{utils::hash_file, Error};

use csv::DeserializeRecordsIntoIter;
use error_stack::{report, Result, ResultExt};
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use std::{io::Write, path::PathBuf};

#[derive(Debug, Clone)]
pub struct FileDownloader {
    // Path to directory where file will be downloaded
    directory_path: PathBuf,
    // HTTP client instance
    http_client: reqwest::Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFile {
    file_path: PathBuf,
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

impl DataFile {
    pub fn into_deserialize<T: DeserializeOwned + 'static>(
        self,
    ) -> Result<FiskeridirRecordIter<std::fs::File, T>, Error> {
        let file = std::fs::File::open(self.file_path).change_context(Error::Deserialize)?;

        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .trim(csv::Trim::Fields)
            .flexible(true)
            .from_reader(file);

        Ok(FiskeridirRecordIter {
            inner: csv_reader.into_deserialize(),
        })
    }

    pub fn hash(&self) -> Result<String, Error> {
        hash_file(&self.file_path).change_context(Error::Hash)
    }
}

// Different sources within Fiskeridir
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSource {
    Landings { year: u32, url: Option<String> },
    Vms { year: u32, url: String },
    ErsDca { year: u32, url: Option<String> },
    ErsPor { year: u32, url: Option<String> },
    ErsDep { year: u32, url: Option<String> },
    ErsTra { year: u32, url: Option<String> },
    AquaCultureRegister { url: String },
}

impl FileSource {
    fn archive_name(&self) -> String {
        format!("{}.zip", self.extract_dir_name())
    }

    fn extract_dir_name(&self) -> String {
        use FileSource::*;

        match *self {
            Landings { year, .. } => format!("{year}-Landings"),
            Vms { year, .. } => format!("{year}-Vms"),
            ErsDca { year, .. } => format!("{year}-ErsDca"),
            ErsPor { year, .. } => format!("{year}-ErsPor"),
            ErsDep { year, .. } => format!("{year}-ErsDep"),
            ErsTra { year, .. } => format!("{year}-ErsTra"),
            AquaCultureRegister { .. } => "AquaCultureRegister".into(),
        }
    }

    // Returns name of the file within the zip archive.
    fn file_name(&self) -> String {
        use FileSource::*;

        match *self {
            Landings { year, .. } => format!("fangstdata_{year}.csv"),
            Vms { year, .. } => match year {
                y if y >= 2022 => format!("{y}-VMS.csv"),
                y => format!("VMS_{y}.csv"),
            },
            ErsDca { year, .. } => match year {
                2024 => "2024-ERS-DCA.csv".to_string(),
                y => format!("elektronisk-rapportering-ers-{y}-fangstmelding-dca.csv"),
            },
            ErsPor { year, .. } => match year {
                2024 => "2024-ERS-POR.csv".to_string(),
                y => format!("elektronisk-rapportering-ers-{y}-ankomstmelding-por.csv"),
            },
            ErsDep { year, .. } => match year {
                2024 => "2024-ERS-DEP.csv".to_string(),
                y => format!("elektronisk-rapportering-ers-{y}-avgangsmelding-dep.csv"),
            },
            ErsTra { year, .. } => match year {
                2024 => "2024-ERS-TRA.csv".to_string(),
                y => format!("elektronisk-rapportering-ers-{y}-overforingsmelding-tra.csv"),
            },
            AquaCultureRegister { .. } => "AquaCultureRegister.csv".into(),
        }
    }

    fn url(&self) -> String {
        use FileSource::*;

        match self {
            Landings { year, url } => match url {
                Some(url) => url.clone(),
                None => format!("https://register.fiskeridir.no/uttrekk/fangstdata_{year}.csv.zip"),
            },
            ErsDca { year, url } |
            ErsPor {year, url} |
            ErsDep {year, url} |
            ErsTra {year, url} => match url {
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
            Landings { year, .. } => year,
            Vms { year, .. } => year,
            ErsDca { year, .. } => year,
            ErsPor { year, .. } => year,
            ErsDep { year, .. } => year,
            ErsTra { year, .. } => year,
            AquaCultureRegister { .. } => 0,
        }
    }
}

impl FileDownloader {
    pub fn new(directory_path: PathBuf) -> Result<FileDownloader, Error> {
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::new(600, 0))
            .build()
            .change_context(Error::Download)?;

        Ok(FileDownloader {
            directory_path,
            http_client: client,
        })
    }

    pub fn clean_download_dir(&self) -> Result<(), std::io::Error> {
        std::fs::remove_dir_all(&self.directory_path)?;
        std::fs::create_dir(&self.directory_path)?;
        Ok(())
    }
    pub async fn download(&self, source: &FileSource) -> Result<DataFile, Error> {
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
            FileSource::Landings { .. }
            | FileSource::Vms { .. }
            | FileSource::ErsDca { .. }
            | FileSource::ErsPor { .. }
            | FileSource::ErsDep { .. }
            | FileSource::ErsTra { .. } => {
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

                extract_path.join(source.file_name())
            }
            FileSource::AquaCultureRegister { .. } => {
                let mut path = PathBuf::from(&self.directory_path);
                path.push(source.file_name());

                let mut file = std::fs::File::create(&path).change_context(Error::Download)?;

                let text = response.text().await.change_context(Error::Download)?;

                // This file contains an extra line at the beginning that we need to skip:
                // 'AKVAKULTURTILLATELSER PR. 09-08-2023 ;;;;;;;;;;;;;;;;;;;;;;;;;;'
                if let Some((_, text)) = text.split_once('\n') {
                    file.write_all(text.as_bytes())
                        .change_context(Error::Download)?;
                }

                path
            }
        };

        Ok(DataFile { file_path })
    }
}
