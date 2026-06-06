//! Translation: converts an outline into output strings or commands.
//!
//! - undoable

#[cfg(test)]
mod test;

use crate::model::stroke::Stroke;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Translation {
    translated: String,
}

#[derive(Debug, Clone)]
pub struct Translator {
    //
}

impl Translator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn translate(&mut self, _outline: &[Stroke]) -> Translation {
        // TODO: translate with dictionaries
        Translation {
            translated: "dummy".to_string(),
        }
    }
}
