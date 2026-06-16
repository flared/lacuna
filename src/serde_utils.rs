use serde::{Deserialize, Serialize};

pub fn serialize_patterns<S: serde::Serializer>(
    patterns: &[glob::Pattern],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let strings: Vec<&str> = patterns.iter().map(|p| p.as_str()).collect();
    strings.serialize(serializer)
}

pub fn deserialize_patterns<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<glob::Pattern>, D::Error> {
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| glob::Pattern::new(&s).map_err(serde::de::Error::custom))
        .collect()
}
