#[derive(Debug)]
pub enum FileHashes {
    Landings,
    ErsDca,
    ErsDep,
    ErsPor,
}

#[derive(Debug)]
pub struct FileHashId(String);

impl FileHashId {
    pub fn new(source: FileHashes, year: u32) -> FileHashId {
        match source {
            FileHashes::Landings => FileHashId(format!("landings_{}", year)),
            FileHashes::ErsDca => FileHashId(format!("ers_{}", year)),
            FileHashes::ErsDep => FileHashId(format!("ers_dep_{}", year)),
            FileHashes::ErsPor => FileHashId(format!("ers_por_{}", year)),
        }
    }
}

#[derive(Debug)]
pub enum HashDiff {
    Equal,
    Changed,
}
