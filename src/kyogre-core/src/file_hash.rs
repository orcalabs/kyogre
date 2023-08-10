#[derive(Debug, Copy, Clone)]
pub enum FileHash {
    Landings,
    ErsDca,
    ErsDep,
    ErsPor,
    ErsTra,
    Vms,
    AquaCultureRegister,
}

#[derive(Debug)]
pub struct FileHashId(String);

impl FileHashId {
    pub fn new(source: FileHash, year: u32) -> FileHashId {
        match source {
            FileHash::Landings => FileHashId(format!("landings_{year}")),
            FileHash::ErsDca => FileHashId(format!("ers_dca_{year}")),
            FileHash::ErsDep => FileHashId(format!("ers_dep_{year}")),
            FileHash::ErsPor => FileHashId(format!("ers_por_{year}")),
            FileHash::ErsTra => FileHashId(format!("ers_tra_{year}")),
            FileHash::Vms => FileHashId(format!("vms_{year}")),
            FileHash::AquaCultureRegister => FileHashId("aqua_culture_register".into()),
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
