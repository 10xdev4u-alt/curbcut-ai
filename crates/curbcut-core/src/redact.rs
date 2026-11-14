//! Redaction layer for diagnostics.
//!
//! The never list forbids content in logs, dumps, crash reports, and telemetry.
//! This module is the backstop: a [`Redactor`] holds the strings that must
//! never appear in a diagnostic, and every diagnostic passes through it before
//! it is written.
//!
//! The canonical list is `tests/fixtures/content_canaries.txt`, one string per
//! line, `#` for comments. [`Redactor::canaries`] loads it, and
//! `scripts/check_no_content_logging.py` reads the same file to scan artifacts
//! in CI. Adding a canary means adding one line.
//!
//! [`Logger`] is the diagnostics facade. It redacts before it hands a line to
//! its sink, so a future call site that accidentally formats a label cannot
//! leak it. The process-wide logger is installed with [`set_global`] and
//! reached through [`global`] or the `log_*!` macros.

use std::fmt;
use std::sync::OnceLock;

/// Prefix shared by every content canary.
pub const CANARY_PREFIX: &str = "CC-CANARY-";

/// What a redacted secret is replaced with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactMode {
    /// A fixed placeholder. Default.
    Placeholder,
    /// A short deterministic hash, for values that are safe to correlate.
    /// Not reversible, but stable across runs.
    Hash,
}

/// Replaces registered secrets before a string is logged.
#[derive(Clone)]
pub struct Redactor {
    secrets: Vec<String>,
    mode: RedactMode,
    placeholder: String,
}

impl fmt::Debug for Redactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Redactor")
            .field("secrets", &self.secrets.len())
            .field("mode", &self.mode)
            .finish()
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    /// An empty redactor, which changes nothing.
    pub fn new() -> Self {
        Self {
            secrets: Vec::new(),
            mode: RedactMode::Placeholder,
            placeholder: "[redacted]".to_string(),
        }
    }

    /// Sets the replacement mode.
    pub fn with_mode(mut self, mode: RedactMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the placeholder used by [`RedactMode::Placeholder`].
    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    /// Registers one secret. Empty and duplicate strings are ignored.
    pub fn add_secret(&mut self, secret: &str) {
        if secret.is_empty() || self.secrets.iter().any(|known| known == secret) {
            return;
        }
        self.secrets.push(secret.to_string());
    }

    /// Registers many secrets.
    pub fn add_secrets<I, S>(&mut self, secrets: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for secret in secrets {
            self.add_secret(secret.as_ref());
        }
    }

    /// A redactor loaded from the repository canary fixture.
    pub fn canaries() -> Self {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/content_canaries.txt"
        );
        Self::from_canary_file(path)
    }

    /// Loads canaries from a file, one per line, `#` for comments.
    ///
    /// A missing file yields an empty redactor, never a panic.
    pub fn from_canary_file(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_canary_list(&text),
            Err(_) => Self::new(),
        }
    }

    /// Parses a canary list.
    pub fn from_canary_list(text: &str) -> Self {
        let mut redactor = Self::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            redactor.add_secret(line);
        }
        redactor
    }

    /// The registered secrets.
    pub fn secrets(&self) -> &[String] {
        &self.secrets
    }

    /// Whether `text` contains any registered secret.
    pub fn contains_any(&self, text: &str) -> bool {
        self.secrets
            .iter()
            .any(|secret| text.contains(secret.as_str()))
    }

    /// Returns `text` with every registered secret replaced.
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        // Longest first so overlapping secrets are fully replaced.
        let mut secrets = self.secrets.clone();
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        for secret in secrets {
            let replacement = match self.mode {
                RedactMode::Placeholder => self.placeholder.clone(),
                RedactMode::Hash => format!("[{}]", short_hash(&secret)),
            };
            out = out.replace(&secret, &replacement);
        }
        out
    }
}

/// FNV-1a as 16 hex characters. Correlatable, not reversible.
fn short_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Severity of a diagnostic line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Fine-grained developer detail.
    Debug,
    /// A normal lifecycle event.
    Info,
    /// A recoverable problem.
    Warn,
    /// A failure the user or operator must see.
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        })
    }
}

/// A diagnostics sink that redacts before writing.
pub struct Logger {
    redactor: Redactor,
    sink: Box<dyn Fn(&str) + Send + Sync>,
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logger")
            .field("redactor", &self.redactor)
            .finish()
    }
}

impl Logger {
    /// A logger that writes redacted lines to stderr.
    pub fn new(redactor: Redactor) -> Self {
        Self {
            redactor,
            sink: Box::new(|line| eprintln!("{line}")),
        }
    }

    /// A logger with a caller-supplied sink, for tests and embedders.
    pub fn with_sink(redactor: Redactor, sink: Box<dyn Fn(&str) + Send + Sync>) -> Self {
        Self { redactor, sink }
    }

    /// The redactor this logger applies.
    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// Redacts `message` and writes one line.
    pub fn log(&self, level: Level, message: &str) {
        let redacted = self.redactor.redact(message);
        (self.sink)(&format!("{level} {redacted}"));
    }

    /// Redacts and writes at [`Level::Debug`].
    pub fn debug(&self, message: &str) {
        self.log(Level::Debug, message);
    }

    /// Redacts and writes at [`Level::Info`].
    pub fn info(&self, message: &str) {
        self.log(Level::Info, message);
    }

    /// Redacts and writes at [`Level::Warn`].
    pub fn warn(&self, message: &str) {
        self.log(Level::Warn, message);
    }

    /// Redacts and writes at [`Level::Error`].
    pub fn error(&self, message: &str) {
        self.log(Level::Error, message);
    }
}

static GLOBAL: OnceLock<Logger> = OnceLock::new();

/// Installs the process-wide logger. The first call wins; later calls are
/// ignored, so a test or embedder can install its own before startup.
pub fn set_global(logger: Logger) {
    let _ = GLOBAL.set(logger);
}

/// The process-wide logger, defaulting to stderr and the canary fixture.
pub fn global() -> &'static Logger {
    GLOBAL.get_or_init(|| Logger::new(Redactor::canaries()))
}

/// Writes a redacted line at debug level through [`global`].
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::redact::global().debug(&format!($($arg)*))
    };
}

/// Writes a redacted line at info level through [`global`].
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::redact::global().info(&format!($($arg)*))
    };
}

/// Writes a redacted line at warn level through [`global`].
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::redact::global().warn(&format!($($arg)*))
    };
}

/// Writes a redacted line at error level through [`global`].
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::redact::global().error(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANARIES: &str = "\
# a comment
CC-CANARY-ONE
CC-CANARY-TWO-Longer

CC-CANARY-THREE
";

    fn redactor() -> Redactor {
        Redactor::from_canary_list(CANARIES)
    }

    #[test]
    fn an_empty_redactor_changes_nothing() {
        let redactor = Redactor::new();
        assert!(!redactor.contains_any("anything at all"));
        assert_eq!(redactor.redact("anything at all"), "anything at all");
    }

    #[test]
    fn redact_replaces_every_registered_secret() {
        let redactor = redactor();
        assert_eq!(
            redactor.secrets().len(),
            3,
            "comments and blanks are skipped"
        );
        let text = "before CC-CANARY-ONE middle CC-CANARY-THREE after";
        let redacted = redactor.redact(text);
        assert!(!redacted.contains("CC-CANARY-ONE"), "{redacted}");
        assert!(!redacted.contains("CC-CANARY-THREE"), "{redacted}");
        assert!(redacted.contains("[redacted]"), "{redacted}");
        assert!(redacted.starts_with("before "), "{redacted}");
    }

    #[test]
    fn redact_is_idempotent() {
        let redactor = redactor();
        let once = redactor.redact("CC-CANARY-TWO-Longer");
        assert_eq!(redactor.redact(&once), once);
    }

    #[test]
    fn the_longest_secret_wins_over_an_overlap() {
        let mut redactor = Redactor::new();
        redactor.add_secrets(["CC-CANARY", "CC-CANARY-OVERLAP"]);
        let redacted = redactor.redact("xx CC-CANARY-OVERLAP yy");
        assert_eq!(redacted, "xx [redacted] yy");
    }

    #[test]
    fn hash_mode_is_stable_and_content_free() {
        let redactor = redactor().with_mode(RedactMode::Hash);
        let first = redactor.redact("CC-CANARY-ONE");
        let second = redactor.redact("CC-CANARY-ONE");
        assert_eq!(first, second, "the hash must be deterministic");
        assert_eq!(first, "[ade265140ba386f3]");
        assert!(!first.contains("CC-CANARY-ONE"));
    }

    #[test]
    fn a_missing_canary_file_is_not_an_error() {
        let redactor = Redactor::from_canary_file("/nonexistent/canaries.txt");
        assert!(redactor.secrets().is_empty());
    }

    #[test]
    fn the_repository_fixture_loads() {
        let redactor = Redactor::canaries();
        assert!(
            !redactor.secrets().is_empty(),
            "the canary fixture must be present"
        );
        assert!(
            redactor
                .secrets()
                .iter()
                .all(|secret| secret.starts_with(CANARY_PREFIX)),
            "every canary must carry the prefix"
        );
    }

    #[test]
    fn the_logger_redacts_before_the_sink() {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured = sink.clone();
        let logger = Logger::with_sink(
            redactor(),
            Box::new(move |line| {
                let mut buffer = captured.lock().expect("sink");
                buffer.push_str(line);
                buffer.push('\n');
            }),
        );
        logger.info("label=CC-CANARY-ONE count=1");
        let output = sink.lock().expect("sink").clone();
        assert!(!output.contains("CC-CANARY-ONE"), "{output}");
        assert!(output.contains("[redacted]"), "{output}");
        assert!(output.starts_with("INFO "), "{output}");
    }

    #[test]
    fn the_global_logger_is_installable_and_redacts() {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured = sink.clone();
        set_global(Logger::with_sink(
            redactor(),
            Box::new(move |line| {
                let mut buffer = captured.lock().expect("sink");
                buffer.push_str(line);
            }),
        ));
        log_warn!("leak=CC-CANARY-THREE");
        let output = sink.lock().expect("sink").clone();
        assert!(!output.contains("CC-CANARY-THREE"), "{output}");
    }
}
