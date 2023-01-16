use std::fmt::Display;

use error_stack::Context;

#[derive(Debug)]
pub enum EngineError {
    Transition,
}

impl Context for EngineError {}

impl Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Transition => {
                f.write_str("an error occured when trying to figure out the next state transition")
            }
        }
    }
}
