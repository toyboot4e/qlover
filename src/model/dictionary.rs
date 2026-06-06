//! Dictionary.
//!
//! - No `{plover:deleted}` support.

#[cfg(test)]
mod test;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::fmt;

/// String notation of an outline.
///
/// # Serialization format
///
/// It's serialized/deserialized as a string, storkes separated by "/".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Outline(pub Vec<String>);

impl fmt::Display for Outline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        for (i, s) in self.0.iter().enumerate() {
            write!(f, "{}", s)?;
            if i != self.0.len() - 1 {
                write!(f, "{}", "/")?;
            }
        }
        Ok(())
    }
}

impl Serialize for Outline {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.join("/").serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Outline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ss = s.split("/").map(|s| s.to_string()).collect::<Vec<_>>();
        // TODO: validate each stroke
        Ok(Self(ss))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    String(String),
    // Command
}

/// Maps sequences to translations and tracks the length of the longest key.
///
/// # Serialization format
///
/// The dictionary is serialized/deserialized as a map from an outline to the output.
#[derive(Debug, Clone)]
pub struct StenoDictionary {
    entries: FxHashMap<Outline, Output>,
    rev: FxHashMap<Output, Vec<Outline>>,
}

impl Serialize for StenoDictionary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.entries.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StenoDictionary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = FxHashMap::<Outline, Output>::deserialize(deserializer)?;
        Ok(Self::new(entries))
    }
}

impl StenoDictionary {
    pub fn new(entries: FxHashMap<Outline, Output>) -> Self {
        let mut rev = FxHashMap::<Output, Vec<Outline>>::default();
        for (outline, output) in &entries {
            rev.entry(output.clone()).or_default().push(outline.clone());
        }
        Self { entries, rev }
    }

    /// Looks up an output for the outline.
    pub fn get(&self, outline: &Outline) -> Option<&Output> {
        self.entries.get(outline)
    }

    /// Looks up outlines for the output.
    pub fn rev_get(&self, output: &Output) -> Option<&Vec<Outline>> {
        self.rev.get(output)
    }
}

/// A stack of dictionaries.
#[derive(Debug, Clone, Default)]
pub struct Dictionaries {
    /// Stack of dictionaries.
    pub stack: Vec<StenoDictionary>,
}

impl Dictionaries {
    /// Looks up an output for the outline.
    pub fn get(&self, outline: &Outline) -> Option<&Output> {
        self.stack.iter().find_map(|d| d.get(outline))
    }

    /// Looks up outlines for the output.
    pub fn rev_get(&self, output: &Output) -> Option<&Vec<Outline>> {
        self.stack.iter().find_map(|d| d.rev_get(output))
    }
}
