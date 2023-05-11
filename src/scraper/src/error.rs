use error_stack::Context;

#[derive(Debug)]
pub struct ScraperError;

impl Context for ScraperError {}

impl std::fmt::Display for ScraperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred while scraping a data source")
    }
}

#[derive(Debug)]
pub struct DownloadError;

impl Context for DownloadError {}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred while downloading data")
    }
}
