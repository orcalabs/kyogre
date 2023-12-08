#[derive(Debug, Copy, Clone, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum FileId {
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
    pub fn new(source: FileId, year: u32) -> FileHashId {
        match source {
            FileId::Landings => FileHashId(format!("landings_{year}")),
            FileId::ErsDca => FileHashId(format!("ers_dca_{year}")),
            FileId::ErsDep => FileHashId(format!("ers_dep_{year}")),
            FileId::ErsPor => FileHashId(format!("ers_por_{year}")),
            FileId::ErsTra => FileHashId(format!("ers_tra_{year}")),
            FileId::Vms => FileHashId(format!("vms_{year}")),
            FileId::AquaCultureRegister => FileHashId("aqua_culture_register".into()),
        }
    }
}

impl AsRef<str> for FileHashId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
