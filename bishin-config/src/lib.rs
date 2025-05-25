use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The default name of the config file.
pub const CONFIG_FILENAME: &str = "bishin.toml";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("couldn't load config file at '{}': {}", .0.display(), .1)]
    MissingConfig(PathBuf, std::io::Error),
    #[error(transparent)]
    Parse(#[from] toml::de::Error),
}

/// The configuration for bishin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// the relative path of the directory to look for bishin files in.
    #[serde(default = "default_test_dir")]
    pub test_dir: PathBuf,
    /// The relative path of the directory in which bishin will store results,
    /// test files, intermediate data, etc.
    #[serde(default = "default_work_dir")]
    pub work_dir: PathBuf,
}

impl Config {
    /// Load the config file from disk given an absolute or relative path.
    fn load_inner(path: impl AsRef<Path>) -> Result<Self, Error> {
        let full_path = std::path::absolute(path).map_err(Error::IO)?;
        let contents = std::fs::read_to_string(&full_path)
            .map_err(|err| Error::MissingConfig(full_path, err))?;
        toml::from_str(&contents).map_err(Error::Parse)
    }

    /// Compute the path of the config file with an optional override for its
    /// location.
    fn get_path(maybe_override: Option<&PathBuf>) -> Result<PathBuf, Error> {
        if let Some(ref relpath) = maybe_override {
            std::path::absolute(relpath).map_err(Error::IO)
        } else {
            std::env::current_dir()
                .map_err(Error::IO)
                .map(|p| p.join(CONFIG_FILENAME))
        }
    }

    /// Load the config file from disk from either the default location or a
    /// user-supplied override location.
    pub fn load(path_override: Option<&PathBuf>) -> Result<Self, Error> {
        let path = Self::get_path(path_override)?;
        Self::load_inner(path)
    }
}

fn default_test_dir() -> PathBuf {
    PathBuf::from("tests")
}

fn default_work_dir() -> PathBuf {
    PathBuf::from(".bishin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.test_dir, PathBuf::from("tests"));
        assert_eq!(config.work_dir, PathBuf::from(".bishin"));
    }

    #[test]
    fn parses_full() {
        let input = r#"
            test-dir = "testdir"
            work-dir = "workdir"
        "#;
        let config: Config = toml::from_str(input).unwrap();
        assert_eq!(config.test_dir, PathBuf::from("testdir"));
        assert_eq!(config.work_dir, PathBuf::from("workdir"));
    }
}
