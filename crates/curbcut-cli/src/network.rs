//! Content-free network inspection and a revoke recipe.
//!
//! [`network_status`] parses a config document supplied on stdin, builds a
//! [`KillSwitch`] with `from_config`, and prints only fixed labels. It never
//! dials and never echoes a config token or value. [`network_revoke`] prints
//! the canonical `update_check = false` line as a recipe; it writes no file and
//! cannot reach a running app. Nothing here is an opt-in or a persisted change.

use std::fmt;

use curbcut_core::config::Config;
use curbcut_core::error::CurbCutError;
use curbcut_core::net::{KillSwitch, NetworkFeature, NetworkMode};

use crate::command::EXIT_USAGE;
use crate::EXIT_OK;

/// Root keys whose names may be reported in a usage error.
///
/// Only fixed names from this list are printed. A value or an unknown key is
/// never echoed, even when the parser's error message contains it.
const KNOWN_FIELDS: [&str; 6] = [
    "update_check",
    "active_profile",
    "hot_reload",
    "version",
    "panic_key",
    "profile",
];

/// Parses `input` into a config, or writes a content-free error to `err`.
fn parse_config(input: &str, err: &mut dyn fmt::Write) -> Result<Config, ()> {
    match Config::from_toml(input) {
        Ok(config) => Ok(config),
        Err(error) => {
            match content_free_detail(&error) {
                Some(detail) => {
                    let _ = writeln!(err, "error: invalid configuration: {detail}");
                }
                None => {
                    let _ = writeln!(err, "error: invalid configuration");
                }
            }
            Err(())
        }
    }
}

/// Extracts only a line number or a known field name from a parser error.
fn content_free_detail(error: &CurbCutError) -> Option<String> {
    let message = match error {
        CurbCutError::Config { message } => message,
        _ => return None,
    };
    if let Some(rest) = message.strip_prefix("line ") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(format!("line {digits}"));
        }
    }
    for field in KNOWN_FIELDS {
        if message.contains(field) {
            return Some(format!("field={field}"));
        }
    }
    None
}

/// Prints the config-built switch decision for a TOML document on stdin.
///
/// Output is four fixed lines: `update_check=allowed|blocked`,
/// `policy=updates-only|offline|online`, `switch=engaged|released`, and
/// `scope=process-local RAM only`. `from_config` only produces the first two
/// policy modes; `online` is named for completeness. No content is echoed.
/// Malformed input writes a content-free error to `err` and returns
/// [`EXIT_USAGE`].
pub fn network_status(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    let config = match parse_config(input, err) {
        Ok(config) => config,
        Err(()) => return EXIT_USAGE,
    };
    let switch = KillSwitch::from_config(&config);
    let allowed = switch.allows(NetworkFeature::UpdateCheck);
    let _ = writeln!(
        out,
        "update_check={}",
        if allowed { "allowed" } else { "blocked" }
    );
    let policy = match switch.policy().mode() {
        NetworkMode::UpdatesOnly => "updates-only",
        NetworkMode::Offline => "offline",
        NetworkMode::Online => "online",
    };
    let _ = writeln!(out, "policy={policy}");
    let _ = writeln!(
        out,
        "switch={}",
        if switch.is_engaged() {
            "engaged"
        } else {
            "released"
        }
    );
    let _ = writeln!(out, "scope=process-local RAM only");
    EXIT_OK
}

/// Prints a revoke recipe for a TOML document supplied on stdin.
///
/// The output is the canonical config line `update_check = false` and the
/// process-local scope note. This is a recipe the reader may apply by hand;
/// the function writes no file, opens no socket, and changes no persisted
/// state. Malformed input writes a content-free error to `err` and returns
/// [`EXIT_USAGE`].
pub fn network_revoke(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    if parse_config(input, err).is_err() {
        return EXIT_USAGE;
    }
    let _ = writeln!(out, "update_check = false");
    let _ = writeln!(out, "scope=process-local RAM only");
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = network_status(input, &mut out, &mut err);
        (code, out, err)
    }

    fn revoke(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = network_revoke(input, &mut out, &mut err);
        (code, out, err)
    }

    #[test]
    fn an_absent_update_check_key_blocks() {
        let (code, out, err) = status("version = 1\n");
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("update_check=blocked"), "{out}");
        assert!(out.contains("policy=offline"), "{out}");
        assert!(out.contains("switch=engaged"), "{out}");
        assert!(out.contains("scope=process-local RAM only"), "{out}");
    }

    #[test]
    fn an_explicit_false_blocks() {
        let (code, out, _) = status("version = 1\nupdate_check = false\n");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("update_check=blocked"), "{out}");
        assert!(out.contains("switch=engaged"), "{out}");
        assert!(!out.contains("update_check = false"), "{out}");
    }

    #[test]
    fn true_allows_only_the_update_channel() {
        let (code, out, _) = status("version = 1\nupdate_check = true\n");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("update_check=allowed"), "{out}");
        assert!(out.contains("policy=updates-only"), "{out}");
        assert!(out.contains("switch=released"), "{out}");
        assert!(!out.contains("true"), "{out}");
        assert!(!out.contains("update_check = true"), "{out}");
    }

    #[test]
    fn malformed_input_exits_two_and_reports_only_a_line() {
        let (code, out, err) = status("version = 1\nactive_profile = \n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("line 2"), "{err}");
        assert!(!err.contains("active_profile ="), "{err}");
    }

    #[test]
    fn an_unknown_key_is_never_echoed() {
        let (code, out, err) = status("version = 1\ncc_secret_canary = 7\n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(!err.contains("cc_secret_canary"), "{err}");
        assert!(err.contains("invalid configuration"), "{err}");
    }

    #[test]
    fn revoke_prints_the_recipe_and_writes_nothing() {
        let (code, out, err) = revoke("version = 1\nupdate_check = true\n");
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert_eq!(out, "update_check = false\nscope=process-local RAM only\n");
        assert!(!out.contains("true"), "{out}");
    }

    #[test]
    fn revoke_rejects_malformed_input_without_echo() {
        let (code, out, err) = revoke("version = 1\ncc_secret_canary = 7\n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(!err.contains("cc_secret_canary"), "{err}");
    }
}
