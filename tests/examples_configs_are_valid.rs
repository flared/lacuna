use anyhow::Context;
use lacuna::config::Config;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[test]
fn examples_directory_configs_are_valid() -> Result<(), anyhow::Error> {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let pattern = examples_dir.join("**/lacuna.config.json");
    let pattern_str = pattern.to_str().context("pattern is not valid UTF-8")?;
    let entries: Vec<_> = glob::glob(pattern_str)?.collect();

    assert!(!entries.is_empty(), "no example configs found");

    for entry in entries {
        let path = entry?;
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Config::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
    }

    Ok(())
}

#[test]
fn readme_configs_are_valid() -> Result<(), anyhow::Error> {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let contents = fs::read_to_string(&readme_path).context("failed to read README.md")?;

    let re = Regex::new(r"(?s)```json\n(.*?)```").unwrap();
    let blocks: Vec<&str> = re
        .captures_iter(&contents)
        .map(|c| c.get(1).unwrap().as_str())
        .collect();

    assert!(!blocks.is_empty(), "no JSON blocks found in README.md");

    let blocks_found_got = blocks.len();
    let blocks_found_expected = 3;
    assert!(
        blocks_found_got >= blocks_found_expected,
        "expected to find at least {} JSON blocks found in README.md, found {}",
        blocks_found_expected,
        blocks_found_got,
    );

    for block in blocks {
        if block.contains("flare.io/cap/lacuna") {
            continue;
        }
        Config::from_str(block)
            .with_context(|| format!("failed to parse README JSON block:\n{block}"))?;
    }

    Ok(())
}
