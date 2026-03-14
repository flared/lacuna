use serde::Deserialize;
use serde::Serialize;
use serde_with::DefaultOnError;
use serde_with::serde_as;

#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Capabilities {
    #[serde(
        serialize_with = "serialize_patterns",
        deserialize_with = "deserialize_patterns"
    )]
    pub providers: Vec<glob::Pattern>,
}

fn serialize_patterns<S: serde::Serializer>(
    patterns: &[glob::Pattern],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let strings: Vec<&str> = patterns.iter().map(|p| p.as_str()).collect();
    strings.serialize(serializer)
}

fn deserialize_patterns<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<glob::Pattern>, D::Error> {
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| glob::Pattern::new(&s).map_err(serde::de::Error::custom))
        .collect()
}

impl Capabilities {
    pub fn is_provider_allowed(&self, provider: &str) -> bool {
        self.providers.iter().any(|p| p.matches(provider))
    }

    pub fn deny_all() -> Self {
        Self::default()
    }

    pub fn from_capabilities(capabilities: Vec<Capabilities>) -> Self {
        Self {
            providers: capabilities.into_iter().flat_map(|c| c.providers).collect(),
        }
    }
}

// Tailscale capabilities are a JSON object where keys are capability
// names and values are arrays of arbitrary JSON objects.
//
// Ref: https://tailscale.com/docs/features/access-control/grants/grants-app-capabilities
#[serde_as]
#[derive(Debug, Deserialize)]
struct TailscaleCapabilities {
    #[serde_as(as = "Vec<DefaultOnError<Option<_>>>")]
    #[serde(default, rename = "flare.io/cap/lacuna")]
    app_capabilities: Vec<Option<Capabilities>>,
}

pub fn parse_capabilities(header_value: &str) -> Result<Capabilities, anyhow::Error> {
    let decoded = rfc2047_decoder::Decoder::new()
        .too_long_encoded_word_strategy(rfc2047_decoder::RecoverStrategy::Decode)
        .decode(header_value.as_bytes())
        .unwrap_or_else(|_| header_value.to_owned());

    let ts_capabilities: TailscaleCapabilities = match serde_json::from_str(&decoded) {
        Ok(v) => v,
        Err(e) => {
            return Err(anyhow::anyhow!("failed to parse capabilities header: {e}"));
        }
    };

    let capabilities = Capabilities::from_capabilities(
        ts_capabilities
            .app_capabilities
            .into_iter()
            .flatten()
            .collect(),
    );

    Ok(capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(s: &str) -> glob::Pattern {
        glob::Pattern::new(s).unwrap()
    }

    #[test]
    fn test_is_provider_allowed() {
        let cap = Capabilities {
            providers: vec![pattern("providerone"), pattern("providerprefix-*")],
        };
        assert!(cap.is_provider_allowed("providerone"));
        assert!(cap.is_provider_allowed("providerprefix-suffix"));
        assert!(!cap.is_provider_allowed("providertwo"));
    }

    #[test]
    fn test_capabilities_deserialize() {
        let json = r#"{"providers": ["myprovider", "prefix-*"]}"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert_eq!(
            caps,
            Capabilities {
                providers: vec![pattern("myprovider"), pattern("prefix-*")],
            },
        );
    }

    #[test]
    fn test_capabilities_deserialize_invalid_pattern() {
        let json = r#"{"providers": ["valid", "[invalid"]}"#;
        let result: Result<Capabilities, _> = serde_json::from_str(json);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Pattern syntax error")
        );
    }

    #[test]
    fn test_capabilities_serialize() {
        let caps = Capabilities {
            providers: vec![pattern("myprovider"), pattern("prefix-*")],
        };
        let json = serde_json::to_value(&caps).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"providers": ["myprovider", "prefix-*"]}),
        );
    }

    #[test]
    fn parse_valid_capabilities() {
        let json = r#"{
            "flare.io/cap/lacuna": [
                {"providers": ["firstprovider"]},
                {"providers": ["secondprofider", "thirdprovider"]}
            ]
        }"#;
        let caps = parse_capabilities(json).unwrap();
        assert_eq!(
            caps,
            Capabilities {
                providers: vec![
                    pattern("firstprovider"),
                    pattern("secondprofider"),
                    pattern("thirdprovider")
                ],
            },
        );
    }

    #[test]
    fn parse_capabilities_invalid_ignored() {
        let json = r#"{
            "flare.io/cap/lacuna": [
                {"providers": ["firstprovider"]},
                ["something-bad"],
                {"providers": ["secondprofider"]}
            ]
        }"#;
        let caps = parse_capabilities(json).unwrap();
        assert_eq!(
            caps,
            Capabilities {
                providers: vec![pattern("firstprovider"), pattern("secondprofider")],
            },
        );
    }

    #[test]
    fn parse_missing_key_returns_none() {
        let json = r#"{"other.key": []}"#;
        let caps = parse_capabilities(json).unwrap();
        assert_eq!(caps, Capabilities::default());
    }

    #[test]
    fn parse_malformed_json_returns_err() {
        assert!(parse_capabilities("not json").is_err());
    }

    #[test]
    fn parse_rfc2047_encoded() {
        // Tailscale uses RFC2047 "Q" encoding for values that contain non-ASCII characters.
        // Ref: https://tailscale.com/docs/features/tailscale-serve#app-capabilities-header
        // Q-encoded: {"flare.io/cap/lacuna":[{"providers":["🐿️"]}]}
        let encoded =
            r#"=?utf-8?q?{"flare.io/cap/lacuna":[{"providers":["=F0=9F=90=BF=EF=B8=8F"]}]}?="#;
        let caps = parse_capabilities(encoded).unwrap();
        assert_eq!(
            caps,
            Capabilities {
                providers: vec![pattern("🐿️")],
            },
        );
    }
}
