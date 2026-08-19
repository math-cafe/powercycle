use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use powercycle_config::{Config, LoggingConfig, PathsConfig};
use serde::{Deserialize, Serialize};

const RUN_FORMAT: &str = "powercycle-run";
const RUN_VERSION: u32 = 1;

const STATUS_FORMAT: &str = "powercycle-status";
const STATUS_VERSION: u32 = 1;

/// Captured process invocation.
///
/// `argv` is the authoritative representation available to the program.
/// `command` is a shell-readable reconstruction and is not guaranteed to
/// reproduce the exact quoting or whitespace typed by the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub program: String,
    pub argv: Vec<String>,
    pub command: String,
    pub cwd: PathBuf,
}

impl Invocation {
    /// Captures the current process invocation.
    pub fn capture() -> Result<Self, RunError> {
        let cwd = env::current_dir().map_err(RunError::CurrentDirectory)?;

        let args: Vec<OsString> = env::args_os().collect();

        Ok(Self::from_os_args(args, cwd))
    }

    /// Creates an invocation from explicit arguments.
    ///
    /// Primarily useful for tests and higher-level wrappers.
    #[must_use]
    pub fn from_args<I, S>(args: I, cwd: impl Into<PathBuf>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();

        Self::from_strings(args, cwd.into())
    }

    fn from_os_args(args: Vec<OsString>, cwd: PathBuf) -> Self {
        let strings: Vec<String> = args
            .into_iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();

        Self::from_strings(strings, cwd)
    }

    fn from_strings(args: Vec<String>, cwd: PathBuf) -> Self {
        let program = args
            .first()
            .cloned()
            .unwrap_or_else(|| "pc-search".to_owned());

        let argv = if args.len() > 1 {
            args[1..].to_vec()
        } else {
            Vec::new()
        };

        let command = args
            .iter()
            .map(|argument| quote_shell_argument(argument))
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            program,
            argv,
            command,
            cwd,
        }
    }
}

/// Filesystem paths owned by a search run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunPaths {
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub log: PathBuf,
    pub status: PathBuf,
    pub checkpoint: PathBuf,
    pub dumps: PathBuf,
}

impl RunPaths {
    fn new(root: PathBuf) -> Self {
        Self {
            manifest: root.join("run.toml"),
            log: root.join("search.log"),
            status: root.join("status.json"),
            checkpoint: root.join("checkpoint"),
            dumps: root.join("dumps"),
            root,
        }
    }
}

/// Lifecycle state of a search run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunState {
    Starting,
    Running,
    Stopping,
    Completed,
    Failed,
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Completed => "completed",
            Self::Failed => "failed",
        };

        f.write_str(text)
    }
}

/// Machine-readable current state of a search run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStatus {
    pub format: String,
    pub version: u32,
    pub run_id: String,
    pub state: RunState,
    pub started_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub tour: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Active search-run context.
#[derive(Debug)]
pub struct RunContext {
    run_id: String,
    k: u32,
    n: usize,
    started_unix_ms: u64,
    paths: RunPaths,
    status: RunStatus,
}

impl RunContext {
    /// Creates a new search run.
    ///
    /// The shared working directories are created if necessary, followed by
    /// the run directory, checkpoint directory and run-local dump directory.
    ///
    /// Initial `run.toml`, `search.log` and `status.json` files are written
    /// before this function returns.
    pub fn create(
        config: &Config,
        k: u32,
        n: usize,
        invocation: Invocation,
    ) -> Result<Self, RunError> {
        let started_unix_ms = unix_time_ms()?;

        let run_id = generate_run_id(started_unix_ms, k, n);

        Self::create_with_id(config, k, n, invocation, run_id, started_unix_ms)
    }

    /// Returns the run identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.run_id
    }

    /// Returns the problem exponent.
    #[must_use]
    pub const fn k(&self) -> u32 {
        self.k
    }

    /// Returns the problem vertex count.
    #[must_use]
    pub const fn n(&self) -> usize {
        self.n
    }

    /// Returns all paths associated with the run.
    #[must_use]
    pub fn paths(&self) -> &RunPaths {
        &self.paths
    }

    /// Returns the current in-memory status.
    #[must_use]
    pub fn status(&self) -> &RunStatus {
        &self.status
    }

    /// Writes an informational line to the persistent search log.
    pub fn log_info(&self, message: impl AsRef<str>) -> Result<(), RunError> {
        self.append_log("INFO", message.as_ref())
    }

    /// Writes a warning line to the persistent search log.
    pub fn log_warn(&self, message: impl AsRef<str>) -> Result<(), RunError> {
        self.append_log("WARN", message.as_ref())
    }

    /// Writes an error line to the persistent search log.
    pub fn log_error(&self, message: impl AsRef<str>) -> Result<(), RunError> {
        self.append_log("ERROR", message.as_ref())
    }

    /// Updates the current tour and rewrites `status.json`.
    pub fn set_tour(&mut self, tour: u64) -> Result<(), RunError> {
        self.status.tour = tour;
        self.status.updated_unix_ms = unix_time_ms()?;

        self.write_status()
    }

    /// Changes the run state and rewrites `status.json`.
    pub fn set_state(&mut self, state: RunState, message: Option<String>) -> Result<(), RunError> {
        self.status.state = state;
        self.status.message = message;
        self.status.updated_unix_ms = unix_time_ms()?;

        self.write_status()
    }

    /// Marks the run as stopping.
    pub fn mark_stopping(&mut self) -> Result<(), RunError> {
        self.set_state(RunState::Stopping, None)?;
        self.log_info("run stopping")
    }

    /// Marks the run as completed.
    pub fn mark_completed(&mut self) -> Result<(), RunError> {
        self.set_state(RunState::Completed, None)?;
        self.log_info("run completed")
    }

    /// Marks the run as failed.
    pub fn mark_failed(&mut self, message: impl Into<String>) -> Result<(), RunError> {
        let message = message.into();

        self.set_state(RunState::Failed, Some(message.clone()))?;

        self.log_error(format!("run failed: {message}"))
    }

    fn create_with_id(
        config: &Config,
        k: u32,
        n: usize,
        invocation: Invocation,
        run_id: String,
        started_unix_ms: u64,
    ) -> Result<Self, RunError> {
        config.ensure_directories()?;

        let root = config.paths.runs_dir.join(&run_id);

        if root.exists() {
            return Err(RunError::RunAlreadyExists { path: root });
        }

        fs::create_dir_all(&root).map_err(|source| RunError::CreateDirectory {
            path: root.clone(),
            source,
        })?;

        let paths = RunPaths::new(root);

        fs::create_dir_all(&paths.checkpoint).map_err(|source| RunError::CreateDirectory {
            path: paths.checkpoint.clone(),
            source,
        })?;

        fs::create_dir_all(&paths.dumps).map_err(|source| RunError::CreateDirectory {
            path: paths.dumps.clone(),
            source,
        })?;

        let manifest = RunManifest {
            format: RUN_FORMAT.to_owned(),
            version: RUN_VERSION,
            run_id: run_id.clone(),
            started_unix_ms,
            problem: ProblemManifest { k, n },
            invocation: invocation.clone(),
            configuration: ConfigurationManifest {
                project_root: config.sources.project_root.clone(),

                config_path: config.sources.config_path.clone(),

                local_config_path: config.sources.local_config_path.clone(),

                environment_variables: config.sources.environment_variables.clone(),

                resolved: ResolvedConfiguration {
                    paths: config.paths.clone(),
                    logging: config.logging.clone(),
                },
            },
        };

        write_toml_file(&paths.manifest, &manifest)?;

        let status = RunStatus {
            format: STATUS_FORMAT.to_owned(),
            version: STATUS_VERSION,
            run_id: run_id.clone(),
            state: RunState::Running,
            started_unix_ms,
            updated_unix_ms: started_unix_ms,
            tour: 0,
            message: None,
        };

        let context = Self {
            run_id,
            k,
            n,
            started_unix_ms,
            paths,
            status,
        };

        context.write_status()?;

        context.log_info("run started")?;

        context.log_info(format!("command: {}", invocation.command))?;

        context.log_info(format!("cwd: {}", invocation.cwd.display()))?;

        context.log_info(format!("problem: k={k} n={n}"))?;

        context.log_info(format!("config: {}", config.sources.config_path.display()))?;

        match &config.sources.local_config_path {
            Some(path) => {
                context.log_info(format!("local config: {}", path.display()))?;
            }

            None => {
                context.log_info("local config: none")?;
            }
        }

        context.log_info(format!("data: {}", config.paths.data_dir.display()))?;

        Ok(context)
    }

    fn append_log(&self, level: &str, message: &str) -> Result<(), RunError> {
        let timestamp = unix_time_ms()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.paths.log)
            .map_err(|source| RunError::OpenLog {
                path: self.paths.log.clone(),
                source,
            })?;

        writeln!(file, "{timestamp} {level} {message}").map_err(|source| RunError::WriteLog {
            path: self.paths.log.clone(),
            source,
        })
    }

    fn write_status(&self) -> Result<(), RunError> {
        write_json_atomic(&self.paths.status, &self.status)
    }
}

#[derive(Debug, Serialize)]
struct RunManifest {
    format: String,
    version: u32,
    run_id: String,
    started_unix_ms: u64,
    problem: ProblemManifest,
    invocation: Invocation,
    configuration: ConfigurationManifest,
}

#[derive(Debug, Serialize)]
struct ProblemManifest {
    k: u32,
    n: usize,
}

#[derive(Debug, Serialize)]
struct ConfigurationManifest {
    project_root: PathBuf,
    config_path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    local_config_path: Option<PathBuf>,

    environment_variables: Vec<String>,
    resolved: ResolvedConfiguration,
}

#[derive(Debug, Serialize)]
struct ResolvedConfiguration {
    paths: PathsConfig,
    logging: LoggingConfig,
}

fn generate_run_id(unix_ms: u64, k: u32, n: usize) -> String {
    format!("{unix_ms}-p{}-k{k}-n{n}", std::process::id())
}

fn unix_time_ms() -> Result<u64, RunError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(RunError::SystemTime)?;

    let milliseconds = duration.as_millis();

    u64::try_from(milliseconds).map_err(|_| RunError::TimestampOverflow)
}

fn write_toml_file<T>(path: &Path, value: &T) -> Result<(), RunError>
where
    T: Serialize,
{
    let contents = toml::to_string_pretty(value).map_err(RunError::SerializeToml)?;

    fs::write(path, contents).map_err(|source| RunError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json_atomic<T>(path: &Path, value: &T) -> Result<(), RunError>
where
    T: Serialize,
{
    let mut contents = serde_json::to_string_pretty(value).map_err(RunError::SerializeJson)?;

    contents.push('\n');

    let temporary = path.with_extension("json.tmp");

    fs::write(&temporary, contents).map_err(|source| RunError::WriteFile {
        path: temporary.clone(),
        source,
    })?;

    fs::rename(&temporary, path).map_err(|source| RunError::ReplaceFile {
        from: temporary,
        to: path.to_path_buf(),
        source,
    })
}

fn quote_shell_argument(argument: &str) -> String {
    if argument.is_empty() {
        return "''".to_owned();
    }

    if argument.chars().all(is_safe_shell_character) {
        return argument.to_owned();
    }

    format!("'{}'", argument.replace('\'', "'\\''"))
}

fn is_safe_shell_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '_' | '-' | '.' | '/' | ':' | '=' | '+' | ',' | '%' | '@'
        )
}

/// Errors produced by search-run infrastructure.
#[derive(Debug)]
pub enum RunError {
    Config(powercycle_config::ConfigError),

    CurrentDirectory(std::io::Error),

    SystemTime(std::time::SystemTimeError),

    TimestampOverflow,

    RunAlreadyExists {
        path: PathBuf,
    },

    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    ReplaceFile {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },

    OpenLog {
        path: PathBuf,
        source: std::io::Error,
    },

    WriteLog {
        path: PathBuf,
        source: std::io::Error,
    },

    SerializeToml(toml::ser::Error),

    SerializeJson(serde_json::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(f),

            Self::CurrentDirectory(error) => {
                write!(f, "cannot determine current working directory: {error}")
            }

            Self::SystemTime(error) => {
                write!(f, "cannot determine system time: {error}")
            }

            Self::TimestampOverflow => {
                write!(f, "current timestamp exceeds supported range")
            }

            Self::RunAlreadyExists { path } => {
                write!(f, "run directory already exists: {}", path.display())
            }

            Self::CreateDirectory { path, source } => {
                write!(f, "cannot create directory {}: {source}", path.display())
            }

            Self::WriteFile { path, source } => {
                write!(f, "cannot write file {}: {source}", path.display())
            }

            Self::ReplaceFile { from, to, source } => {
                write!(
                    f,
                    "cannot replace {} with {}: {source}",
                    to.display(),
                    from.display()
                )
            }

            Self::OpenLog { path, source } => {
                write!(f, "cannot open log file {}: {source}", path.display())
            }

            Self::WriteLog { path, source } => {
                write!(f, "cannot write log file {}: {source}", path.display())
            }

            Self::SerializeToml(error) => {
                write!(f, "cannot serialize run manifest: {error}")
            }

            Self::SerializeJson(error) => {
                write!(f, "cannot serialize run status: {error}")
            }
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::CurrentDirectory(error) => Some(error),
            Self::SystemTime(error) => Some(error),

            Self::CreateDirectory { source, .. }
            | Self::WriteFile { source, .. }
            | Self::ReplaceFile { source, .. }
            | Self::OpenLog { source, .. }
            | Self::WriteLog { source, .. } => Some(source),

            Self::SerializeToml(error) => Some(error),
            Self::SerializeJson(error) => Some(error),

            Self::TimestampOverflow | Self::RunAlreadyExists { .. } => None,
        }
    }
}

impl From<powercycle_config::ConfigError> for RunError {
    fn from(error: powercycle_config::ConfigError) -> Self {
        Self::Config(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    use powercycle_config::{ConfigSources, LogLevel};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory {
        path: PathBuf,
    }

    impl TempDirectory {
        fn new() -> Self {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);

            let path = env::temp_dir().join(format!(
                "powercycle-run-test-{}-{counter}",
                std::process::id()
            ));

            if path.exists() {
                fs::remove_dir_all(&path).unwrap();
            }

            fs::create_dir_all(&path).unwrap();

            Self { path }
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_config(temporary: &TempDirectory) -> Config {
        let data_dir = temporary.path.join("data");

        Config {
            paths: PathsConfig {
                runs_dir: data_dir.join("runs"),
                dumps_dir: data_dir.join("dumps"),
                certificates_dir: data_dir.join("certificates"),
                data_dir: data_dir.clone(),
            },

            logging: LoggingConfig {
                level: LogLevel::Info,
                timestamps: true,
            },

            sources: ConfigSources {
                project_root: temporary.path.clone(),

                config_path: temporary.path.join("powercycle.toml"),

                local_config_path: None,

                environment_variables: Vec::new(),
            },
        }
    }

    #[test]
    fn reconstructs_command_from_argv() {
        let invocation = Invocation::from_args(
            ["pc-search", "5", "490463", "--label", "hello world"],
            "/tmp",
        );

        assert_eq!(invocation.program, "pc-search");

        assert_eq!(invocation.argv, ["5", "490463", "--label", "hello world",]);

        assert_eq!(
            invocation.command,
            "pc-search 5 490463 --label 'hello world'"
        );
    }

    #[test]
    fn quotes_empty_and_special_arguments() {
        assert_eq!(quote_shell_argument(""), "''");

        assert_eq!(quote_shell_argument("abc-123"), "abc-123");

        assert_eq!(quote_shell_argument("a b"), "'a b'");

        assert_eq!(quote_shell_argument("it's"), "'it'\\''s'");
    }

    #[test]
    fn creates_complete_run_layout() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        let invocation = Invocation::from_args(["pc-search", "5", "490463"], &temporary.path);

        let run = RunContext::create_with_id(
            &config,
            5,
            490_463,
            invocation,
            "test-run".to_owned(),
            1_000,
        )
        .unwrap();

        assert_eq!(run.id(), "test-run");
        assert_eq!(run.k(), 5);
        assert_eq!(run.n(), 490_463);

        assert!(run.paths().root.is_dir());
        assert!(run.paths().checkpoint.is_dir());
        assert!(run.paths().dumps.is_dir());

        assert!(run.paths().manifest.is_file());
        assert!(run.paths().log.is_file());
        assert!(run.paths().status.is_file());

        assert!(config.paths.data_dir.is_dir());

        assert!(config.paths.runs_dir.is_dir());

        assert!(config.paths.dumps_dir.is_dir());

        assert!(config.paths.certificates_dir.is_dir());
    }

    #[test]
    fn manifest_contains_invocation_and_problem() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        let invocation = Invocation::from_args(
            ["pc-search", "5", "490463", "--time-limit", "0"],
            &temporary.path,
        );

        let run = RunContext::create_with_id(
            &config,
            5,
            490_463,
            invocation,
            "manifest-test".to_owned(),
            2_000,
        )
        .unwrap();

        let contents = fs::read_to_string(&run.paths().manifest).unwrap();

        assert!(contents.contains("format = \"powercycle-run\""));

        assert!(contents.contains("k = 5"));

        assert!(contents.contains("n = 490463"));

        assert!(contents.contains("pc-search 5 490463 --time-limit 0"));

        assert!(contents.contains("[configuration.resolved.paths]"));
    }

    #[test]
    fn initial_status_is_running() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        let invocation = Invocation::from_args(["pc-search", "5", "490463"], &temporary.path);

        let run = RunContext::create_with_id(
            &config,
            5,
            490_463,
            invocation,
            "status-test".to_owned(),
            3_000,
        )
        .unwrap();

        assert_eq!(run.status().state, RunState::Running);

        assert_eq!(run.status().tour, 0);

        let contents = fs::read_to_string(&run.paths().status).unwrap();

        let status: RunStatus = serde_json::from_str(&contents).unwrap();

        assert_eq!(status.state, RunState::Running);

        assert_eq!(status.started_unix_ms, 3_000);
    }

    #[test]
    fn status_updates_are_persisted() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        let invocation = Invocation::from_args(["pc-search", "5", "490463"], &temporary.path);

        let mut run = RunContext::create_with_id(
            &config,
            5,
            490_463,
            invocation,
            "update-test".to_owned(),
            4_000,
        )
        .unwrap();

        run.set_tour(767).unwrap();

        assert_eq!(run.status().tour, 767);

        run.mark_completed().unwrap();

        assert_eq!(run.status().state, RunState::Completed);

        let contents = fs::read_to_string(&run.paths().status).unwrap();

        let status: RunStatus = serde_json::from_str(&contents).unwrap();

        assert_eq!(status.tour, 767);

        assert_eq!(status.state, RunState::Completed);
    }

    #[test]
    fn log_contains_reconstructed_command() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        let invocation = Invocation::from_args(
            ["pc-search", "5", "490463", "--solver", "sat"],
            &temporary.path,
        );

        let run = RunContext::create_with_id(
            &config,
            5,
            490_463,
            invocation,
            "log-test".to_owned(),
            5_000,
        )
        .unwrap();

        let contents = fs::read_to_string(&run.paths().log).unwrap();

        assert!(contents.contains("command: pc-search 5 490463 --solver sat"));

        assert!(contents.contains("problem: k=5 n=490463"));

        assert!(contents.contains("run started"));
    }

    #[test]
    fn refuses_existing_run_directory() {
        let temporary = TempDirectory::new();
        let config = test_config(&temporary);

        config.ensure_directories().unwrap();

        let path = config.paths.runs_dir.join("duplicate");

        fs::create_dir_all(&path).unwrap();

        let invocation = Invocation::from_args(["pc-search", "5", "490463"], &temporary.path);

        assert!(matches!(
            RunContext::create_with_id(
                &config,
                5,
                490_463,
                invocation,
                "duplicate".to_owned(),
                6_000,
            ),
            Err(RunError::RunAlreadyExists { .. })
        ));
    }
}
