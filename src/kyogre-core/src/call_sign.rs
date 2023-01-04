use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct CallSign(String);

impl AsRef<str> for CallSign {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CallSign {
    type Error = CallSignError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let pruned_value = value.replace(['_', '-', ' '], "");
        if pruned_value.is_empty() {
            Err(CallSignError)
        } else {
            Ok(CallSign(pruned_value))
        }
    }
}
impl TryFrom<String> for CallSign {
    type Error = CallSignError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        CallSign::try_from(value.as_str())
    }
}

impl CallSign {
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub struct CallSignError;

impl std::error::Error for CallSignError {}

impl Display for CallSignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("call_sign was empty")
    }
}
