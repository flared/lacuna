#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModelRewrite {
    pub original: String,
    pub new_name: String,
}

impl ResolvedModelRewrite {
    pub fn apply_to_path(&self, path: &str) -> String {
        let encoded = percent_encoding::utf8_percent_encode(
            &self.new_name,
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        path.replace(&self.original, &encoded)
    }
}

pub fn rewrite_request_path(
    mut request: axum::extract::Request,
    rewrite: &ResolvedModelRewrite,
) -> anyhow::Result<axum::extract::Request> {
    let pq = request.uri().path_and_query();
    let path = pq.map(|pq| pq.path()).unwrap_or("/");
    let query_suffix = pq
        .and_then(|pq| pq.query())
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    let new_path = rewrite.apply_to_path(path);

    let mut parts = request.uri().clone().into_parts();
    parts.path_and_query = Some(format!("{new_path}{query_suffix}").parse()?);
    *request.uri_mut() = http::Uri::from_parts(parts)?;
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_to_path_does_rewrite_and_encode() {
        let arn =
            "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/abcd1234567";
        let resolved_model_rewrite = ResolvedModelRewrite {
            original: "us.anthropic.claude-opus-4-5x".to_owned(),
            new_name: arn.to_owned(),
        };
        let out =
            resolved_model_rewrite.apply_to_path("/model/us.anthropic.claude-opus-4-5x/invoke");
        assert!(!out.contains("us.anthropic.claude-opus-4-5x"));

        // `:` and `/` and `-` are all encoded.
        assert_eq!(arn.matches(":").count(), out.matches("%3A").count());
        assert_eq!(arn.matches("/").count(), out.matches("%2F").count());
        assert_eq!(arn.matches("-").count(), out.matches("%2D").count());
    }

    #[test]
    fn apply_to_path_is_noop_when_original_absent() {
        let resolved_model_rewrite = ResolvedModelRewrite {
            original: "not-in-path".to_owned(),
            new_name: "should-not-rewrite".to_owned(),
        };
        let path = "/model/some-other-model/invoke";
        assert_eq!(resolved_model_rewrite.apply_to_path(path), path);
    }
}
