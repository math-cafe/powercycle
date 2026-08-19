//! Shared configuration handling for powercycle.
//!
//! Configuration precedence is:
//!
//! 1. built-in defaults;
//! 2. `powercycle.toml`;
//! 3. `powercycle.local.toml`;
//! 4. environment variables;
//! 5. command-line overrides, applied later by each executable.
//!
//! This crate contains no mathematical, solver or patching code.

use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE: &str = "powercycle.toml";
pub const DEFAULT_LOCAL_CONFIG_FILE: &str = "powercycle.local.toml";

pub const ENV_ROOT: &str = "POWERCYCLE_ROOT";
pub const ENV_CONFIG: &str = "POWERCYCLE_CONFIG";
pub const ENV_LOCAL_CONFIG: &str = "POWERCYCLE_LOCAL_CONFIG";
pub const ENV_DATA: &str = "POWERCYCLE_DATA";

/// Fully resolved powercycle configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub paths: PathsConfig,
    pub logging: LoggingConfig,
    pub sources: ConfigSources,
}

/// Fully resolved filesystem paths.
///
/// All paths are absolute after configuration loading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PathsConfig {
    pub data_dir: PathBuf,

    #[serde(rename = "runs")]
    pub runs_dir: PathBuf,

    #[serde(rename = "dumps")]
    pub dumps_dir: PathBuf,

    #[serde(rename = "certificates")]
    pub certificates_dir: PathBuf,
}

/// Logging configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub timestamps: bool,
}

/// Supported logging levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        };

        f.write_str(value)
    }
}

/// Describes where the resolved configuration came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigSources {
    pub project_root: PathBuf,
    pub config_path: PathBuf,
    pub local_config_path: Option<PathBuf>,
    pub environment_variables: Vec<String>,
}

impl Config {
    /// Loads configuration using the current working directory and process
    /// environment.
    pub fn load() -> Result<Self, ConfigError> {
        let current_dir = env::current_dir().map_err(ConfigError::CurrentDirectory)?;

        let environment = EnvironmentConfig::from_process();

        Self::load_with_environment(&current_dir, environment)
    }

    /// Loads configuration while starting project-root discovery from
    /// `start_dir`.
    ///
    /// This is useful for tools invoked from a subdirectory of the project.
    pub fn load_from(start_dir: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let current_dir = env::current_dir().map_err(ConfigError::CurrentDirectory)?;

        let start_dir = absolute_path(start_dir.as_ref(), &current_dir);

        let environment = EnvironmentConfig::from_process();

        Self::load_with_environment(&start_dir, environment)
    }

    /// Creates all configured working directories if necessary.
    ///
    /// The operation is idempotent.
    pub fn ensure_directories(&self) -> Result<(), ConfigError> {
        for path in [
            &self.paths.data_dir,
            &self.paths.runs_dir,
            &self.paths.dumps_dir,
            &self.paths.certificates_dir,
        ] {
            fs::create_dir_all(path).map_err(|source| ConfigError::CreateDirectory {
                path: path.clone(),
                source,
            })?;
        }

        Ok(())
    }

    /// Returns the effective configuration as TOML.
    ///
    /// Paths are fully resolved absolute paths.
    pub fn to_resolved_toml(&self) -> Result<String, ConfigError> {
        #[derive(Serialize)]
        struct ResolvedConfig<'a> {
            paths: &'a PathsConfig,
            logging: &'a LoggingConfig,
        }

        let resolved = ResolvedConfig {
            paths: &self.paths,
            logging: &self.logging,
        };

        toml::to_string_pretty(&resolved).map_err(ConfigError::Serialize)
    }

    fn load_with_environment(
        start_dir: &Path,
        environment: EnvironmentConfig,
    ) -> Result<Self, ConfigError> {
        let current_dir = env::current_dir().map_err(ConfigError::CurrentDirectory)?;

        let explicit_root = environment
            .root
            .as_deref()
            .map(|path| absolute_path(path, &current_dir));

        let explicit_config = environment
            .config
            .as_deref()
            .map(|path| absolute_path(path, &current_dir));

        let project_root = if let Some(root) = explicit_root {
            root
        } else if let Some(config_path) = explicit_config.as_ref() {
            config_path
                .parent()
                .ok_or_else(|| ConfigError::InvalidConfigPath {
                    path: config_path.clone(),
                })?
                .to_path_buf()
        } else {
            find_project_root(start_dir).ok_or_else(|| ConfigError::ProjectRootNotFound {
                start: start_dir.to_path_buf(),
            })?
        };

        let config_path = explicit_config.unwrap_or_else(|| project_root.join(DEFAULT_CONFIG_FILE));

        if !config_path.is_file() {
            return Err(ConfigError::MissingConfig { path: config_path });
        }

        let explicit_local = environment
            .local_config
            .as_deref()
            .map(|path| absolute_path(path, &current_dir));

        let local_candidate = explicit_local
            .clone()
            .unwrap_or_else(|| project_root.join(DEFAULT_LOCAL_CONFIG_FILE));

        let local_config_path = if local_candidate.is_file() {
            Some(local_candidate)
        } else if explicit_local.is_some() {
            return Err(ConfigError::MissingConfig {
                path: local_candidate,
            });
        } else {
            None
        };

        let mut merged = MergedConfig::default();

        let base = read_partial_config(&config_path)?;
        merged.apply(base);

        if let Some(local_path) = &local_config_path {
            let local = read_partial_config(local_path)?;
            merged.apply(local);
        }

        let mut environment_variables = Vec::new();

        if environment.root.is_some() {
            environment_variables.push(ENV_ROOT.to_owned());
        }

        if environment.config.is_some() {
            environment_variables.push(ENV_CONFIG.to_owned());
        }

        if environment.local_config.is_some() {
            environment_variables.push(ENV_LOCAL_CONFIG.to_owned());
        }

        if let Some(data_dir) = environment.data {
            merged.data_dir = data_dir;
            environment_variables.push(ENV_DATA.to_owned());
        }

        let data_dir = absolute_path(&merged.data_dir, &project_root);

        let runs_dir = absolute_path(&merged.runs, &data_dir);

        let dumps_dir = absolute_path(&merged.dumps, &data_dir);

        let certificates_dir = absolute_path(&merged.certificates, &data_dir);

        Ok(Self {
            paths: PathsConfig {
                data_dir,
                runs_dir,
                dumps_dir,
                certificates_dir,
            },

            logging: LoggingConfig {
                level: merged.log_level,
                timestamps: merged.timestamps,
            },

            sources: ConfigSources {
                project_root,
                config_path,
                local_config_path,
                environment_variables,
            },
        })
    }
}

#[derive(Clone, Debug, Default)]
struct EnvironmentConfig {
    root: Option<PathBuf>,
    config: Option<PathBuf>,
    local_config: Option<PathBuf>,
    data: Option<PathBuf>,
}

impl EnvironmentConfig {
    fn from_process() -> Self {
        Self {
            root: env::var_os(ENV_ROOT).map(PathBuf::from),
            config: env::var_os(ENV_CONFIG).map(PathBuf::from),
            local_config: env::var_os(ENV_LOCAL_CONFIG).map(PathBuf::from),
            data: env::var_os(ENV_DATA).map(PathBuf::from),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PartialConfig {
    paths: PartialPathsConfig,
    logging: PartialLoggingConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PartialPathsConfig {
    data_dir: Option<PathBuf>,
    runs: Option<PathBuf>,
    dumps: Option<PathBuf>,
    certificates: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PartialLoggingConfig {
    level: Option<LogLevel>,
    timestamps: Option<bool>,
}

#[derive(Clone, Debug)]
struct MergedConfig {
    data_dir: PathBuf,
    runs: PathBuf,
    dumps: PathBuf,
    certificates: PathBuf,
    log_level: LogLevel,
    timestamps: bool,
}

impl Default for MergedConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            runs: PathBuf::from("runs"),
            dumps: PathBuf::from("dumps"),
            certificates: PathBuf::from("certificates"),
            log_level: LogLevel::Info,
            timestamps: true,
        }
    }
}

impl MergedConfig {
    fn apply(&mut self, partial: PartialConfig) {
        if let Some(value) = partial.paths.data_dir {
            self.data_dir = value;
        }

        if let Some(value) = partial.paths.runs {
            self.runs = value;
        }

        if let Some(value) = partial.paths.dumps {
            self.dumps = value;
        }

        if let Some(value) = partial.paths.certificates {
            self.certificates = value;
        }

        if let Some(value) = partial.logging.level {
            self.log_level = value;
        }

        if let Some(value) = partial.logging.timestamps {
            self.timestamps = value;
        }
    }
}

fn read_partial_config(path: &Path) -> Result<PartialConfig, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::ReadConfig {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&contents).map_err(|source| ConfigError::ParseConfig {
        path: path.to_path_buf(),
        source,
    })
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| directory.join(DEFAULT_CONFIG_FILE).is_file())
        .map(Path::to_path_buf)
}

fn absolute_path(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Errors produced while loading or preparing configuration.
#[derive(Debug)]
pub enum ConfigError {
    CurrentDirectory(std::io::Error),

    ProjectRootNotFound {
        start: PathBuf,
    },

    InvalidConfigPath {
        path: PathBuf,
    },

    MissingConfig {
        path: PathBuf,
    },

    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },

    ParseConfig {
        path: PathBuf,
        source: toml::de::Error,
    },

    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    Serialize(toml::ser::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirectory(error) => {
                write!(f, "cannot determine current working directory: {error}")
            }

            Self::ProjectRootNotFound { start } => {
                write!(
                    f,
                    "cannot find {DEFAULT_CONFIG_FILE} from {} or any parent directory",
                    start.display()
                )
            }

            Self::InvalidConfigPath { path } => {
                write!(
                    f,
                    "configuration path has no parent directory: {}",
                    path.display()
                )
            }

            Self::MissingConfig { path } => {
                write!(f, "configuration file does not exist: {}", path.display())
            }

            Self::ReadConfig { path, source } => {
                write!(
                    f,
                    "cannot read configuration file {}: {source}",
                    path.display()
                )
            }

            Self::ParseConfig { path, source } => {
                write!(
                    f,
                    "cannot parse configuration file {}: {source}",
                    path.display()
                )
            }

            Self::CreateDirectory { path, source } => {
                write!(f, "cannot create directory {}: {source}", path.display())
            }

            Self::Serialize(error) => {
                write!(f, "cannot serialize resolved configuration: {error}")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentDirectory(error) => Some(error),

            Self::ReadConfig { source, .. } => Some(source),

            Self::ParseConfig { source, .. } => Some(source),

            Self::CreateDirectory { source, .. } => Some(source),

            Self::Serialize(error) => Some(error),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new() -> Self {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);

            let root = env::temp_dir().join(format!(
                "powercycle-config-test-{}-{counter}",
                std::process::id()
            ));

            if root.exists() {
                fs::remove_dir_all(&root).unwrap();
            }

            fs::create_dir_all(&root).unwrap();

            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn standard_config() -> &'static str {
        r#"
[paths]
data_dir = "data"
runs = "runs"
dumps = "dumps"
certificates = "certificates"

[logging]
level = "info"
timestamps = true
"#
    }

    #[test]
    fn loads_base_configuration() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        let config =
            Config::load_with_environment(&project.root, EnvironmentConfig::default()).unwrap();

        assert_eq!(config.paths.data_dir, project.root.join("data"));

        assert_eq!(config.paths.runs_dir, project.root.join("data/runs"));

        assert_eq!(config.paths.dumps_dir, project.root.join("data/dumps"));

        assert_eq!(
            config.paths.certificates_dir,
            project.root.join("data/certificates")
        );

        assert_eq!(config.logging.level, LogLevel::Info);
        assert!(config.logging.timestamps);
    }

    #[test]
    fn local_configuration_overrides_base() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        project.write(
            DEFAULT_LOCAL_CONFIG_FILE,
            r#"
[paths]
dumps = "interesting-dumps"

[logging]
level = "debug"
"#,
        );

        let config =
            Config::load_with_environment(&project.root, EnvironmentConfig::default()).unwrap();

        assert_eq!(
            config.paths.dumps_dir,
            project.root.join("data/interesting-dumps")
        );

        assert_eq!(config.logging.level, LogLevel::Debug);
        assert!(config.logging.timestamps);

        assert_eq!(
            config.sources.local_config_path,
            Some(project.root.join(DEFAULT_LOCAL_CONFIG_FILE))
        );
    }

    #[test]
    fn data_environment_variable_has_priority() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        project.write(
            DEFAULT_LOCAL_CONFIG_FILE,
            r#"
[paths]
data_dir = "local-data"
"#,
        );

        let environment = EnvironmentConfig {
            data: Some(PathBuf::from("environment-data")),
            ..EnvironmentConfig::default()
        };

        let config = Config::load_with_environment(&project.root, environment).unwrap();

        assert_eq!(config.paths.data_dir, project.root.join("environment-data"));

        assert_eq!(
            config.paths.runs_dir,
            project.root.join("environment-data/runs")
        );

        assert_eq!(
            config.sources.environment_variables,
            vec![ENV_DATA.to_owned()]
        );
    }

    #[test]
    fn finds_project_root_from_subdirectory() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        let nested = project.root.join("src/powercycle/core");

        fs::create_dir_all(&nested).unwrap();

        let config = Config::load_with_environment(&nested, EnvironmentConfig::default()).unwrap();

        assert_eq!(config.sources.project_root, project.root);
    }

    #[test]
    fn explicit_root_is_supported() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        let unrelated = env::temp_dir().join("powercycle-unrelated");

        let environment = EnvironmentConfig {
            root: Some(project.root.clone()),
            ..EnvironmentConfig::default()
        };

        let config = Config::load_with_environment(&unrelated, environment).unwrap();

        assert_eq!(config.sources.project_root, project.root);

        assert_eq!(
            config.sources.environment_variables,
            vec![ENV_ROOT.to_owned()]
        );
    }

    #[test]
    fn creates_missing_working_directories() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        let config =
            Config::load_with_environment(&project.root, EnvironmentConfig::default()).unwrap();

        assert!(!config.paths.runs_dir.exists());
        assert!(!config.paths.dumps_dir.exists());
        assert!(!config.paths.certificates_dir.exists());

        config.ensure_directories().unwrap();

        assert!(config.paths.data_dir.is_dir());
        assert!(config.paths.runs_dir.is_dir());
        assert!(config.paths.dumps_dir.is_dir());
        assert!(config.paths.certificates_dir.is_dir());
    }

    #[test]
    fn rejects_unknown_configuration_keys() {
        let project = TempProject::new();

        project.write(
            DEFAULT_CONFIG_FILE,
            r#"
[paths]
data_dir = "data"
typo = "oops"
"#,
        );

        assert!(matches!(
            Config::load_with_environment(&project.root, EnvironmentConfig::default(),),
            Err(ConfigError::ParseConfig { .. })
        ));
    }

    #[test]
    fn serializes_resolved_configuration() {
        let project = TempProject::new();

        project.write(DEFAULT_CONFIG_FILE, standard_config());

        let config =
            Config::load_with_environment(&project.root, EnvironmentConfig::default()).unwrap();

        let toml = config.to_resolved_toml().unwrap();

        assert!(toml.contains("[paths]"));
        assert!(toml.contains("[logging]"));
        assert!(toml.contains("level = \"info\""));
        assert!(toml.contains("timestamps = true"));
    }
}
