#[derive(Debug, Copy, Clone)]
pub enum FileHash {
    Landings,
    ErsDca,
    ErsDep,
    ErsPor,
}

#[derive(Debug)]
pub struct FileHashId(String);

impl FileHashId {
    pub fn new(source: FileHash, year: u32) -> FileHashId {
        match source {
            FileHash::Landings => FileHashId(format!("landings_{year}")),
            FileHash::ErsDca => FileHashId(format!("ers_{year}")),
            FileHash::ErsDep => FileHashId(format!("ers_dep_{year}")),
            FileHash::ErsPor => FileHashId(format!("ers_por_{year}")),
        }
    }
}

#[derive(Debug)]
pub enum HashDiff {
    Equal,
    Changed,
}

impl AsRef<str> for FileHashId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
