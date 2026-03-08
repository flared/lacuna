use anyhow::Context;
use lacuna::config::Config;
use std::fs;
use std::path::Path;

#[test]
fn examples_configs_are_valid() -> Result<(), anyhow::Error> {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let pattern = examples_dir.join("**/lacuna.config.json");
    let pattern_str = pattern.to_str().context("pattern is not valid UTF-8")?;
    let entries: Vec<_> = glob::glob(pattern_str)?.collect();

    assert!(!entries.is_empty(), "no example configs found");

    for entry in entries {
        let path = entry?;
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Config::parse(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    }

    Ok(())
}
