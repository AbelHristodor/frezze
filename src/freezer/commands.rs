//! Command parsing for GitHub comment-based freeze commands.
//!
//! This module handles parsing GitHub comment commands into structured command arguments
//! using the clap parser. It supports all freeze management commands including scheduling,
//! PR unlocking, and status checking.
//!
//! # Supported Commands
//!
//! - `/freeze` - Freeze the current repository or specific repositories with `--repo`
//! - `/freeze-all` - Freeze all repositories in the organization or specific repositories with `--repo`
//! - `/unfreeze` - Unfreeze the current repository
//! - `/unfreeze-all` - Unfreeze all repositories in the organization
//! - `/status` - Show freeze status for repositories
//! - `/schedule-freeze` - Schedule a freeze for specific time periods
//! - `/unlock-pr` - Unlock a specific PR during a freeze
//!
//! # Example Usage
//!
//! ```
//! use frezze::freezer::commands::parse;
//!
//! let input = "/freeze --duration 3h --reason \"Emergency maintenance\"";
//! let cli = parse(input).unwrap();
//!
//! match cli.command {
//!     Command::Freeze(freeze_args) => {
//!         println!("Freezing for {:?}", freeze_args.duration);
//!         println!("Reason: {:?}", freeze_args.reason);
//!     }
//!     _ => {}
//! }
//!
//! // Freeze specific repositories
//! let input = "/freeze --repo owner/repo1,owner/repo2 --duration 2h";
//! let cli = parse(input).unwrap();
//!
//! match cli.command {
//!     Command::Freeze(freeze_args) => {
//!         println!("Repos to freeze: {:?}", freeze_args.repos);
//!         println!("Duration: {:?}", freeze_args.duration);
//!     }
//!     _ => {}
//! }
//! ```
use clap::{Parser, Subcommand};

use chrono::{DateTime, Duration, Utc};
use clap::Args;
use tracing::error;

use crate::freezer::errors::ParsingError;

pub fn parse(input: &str) -> Result<Cli, ParsingError> {
    if input.is_empty() || !input.starts_with("/") {
        return Err(ParsingError::NotACommand);
    }

    let input = input.trim_start_matches("/");

    let args = shell_words::split(input).map_err(|e| {
        error!("MalformedCommand: {:?}", e);
        ParsingError::MalformedCommand
    })?;

    let mut argv = vec!["bin".to_string()];
    argv.extend(args);

    Cli::try_parse_from(argv).map_err(|e| {
        error!("MalformedCommand: {:?}", e);
        ParsingError::MalformedCommand
    })
}

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// All available freeze management commands that can be executed via GitHub comments.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Freeze the current repository or specific repositories with --repo
    Freeze(FreezeArgs),
    /// Freeze all repositories in the organization or specific repositories with --repo
    FreezeAll(FreezeArgs),
    /// Unfreeze the current repository
    Unfreeze(UnfreezeArgs),
    /// Unfreeze all repositories in the organization
    UnfreezeAll,
    /// Show freeze status for specified repositories
    Status(StatusArgs),
    /// Schedule a freeze for a specific time period
    ScheduleFreeze(ScheduleFreezeArgs),
    /// Unlock a specific PR during a freeze
    UnlockPr(UnlockPrArgs),
}

#[derive(Args, Debug)]
pub struct FreezeArgs {
    /// Duration to freeze (e.g. "3h", "15m"), optional
    #[arg(long, value_parser = parse_duration_2)]
    pub duration: Option<Duration>,

    /// Reason for freezing, optional
    #[arg(long)]
    pub reason: Option<String>,

    /// List of repositories to freeze (supports comma-separated values or multiple --repo flags)
    #[arg(long = "repo", value_delimiter = ',')]
    pub repos: Vec<String>,

    /// Branch to freeze (e.g. "main", "develop"), optional. If not specified, all branches are frozen.
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// List of repositories to check status for
    #[arg(long, value_delimiter = ',')]
    pub repos: Vec<String>,
}

#[derive(Args, Debug)]
pub struct UnfreezeArgs {
    /// Reason for unfreezing, optional
    #[arg(long)]
    pub reason: Option<String>,

    /// Branch to unfreeze (e.g. "main", "develop"), optional. If not specified, unfreezes all branches.
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct ScheduleFreezeArgs {
    /// Start datetime for freeze (RFC3339 format)
    #[arg(long, value_parser = parse_datetime)]
    pub from: DateTime<Utc>,

    /// End datetime for freeze (RFC3339 format), optional
    #[arg(long, value_parser = parse_datetime)]
    pub to: Option<DateTime<Utc>>,

    /// Duration to freeze, optional
    #[arg(long, value_parser = parse_duration_2)]
    pub duration: Option<Duration>,

    /// Reason for freezing, optional
    #[arg(long)]
    pub reason: Option<String>,

    /// Branch to freeze (e.g. "main", "develop"), optional. If not specified, all branches are frozen.
    #[arg(long)]
    pub branch: Option<String>,
}

/// Arguments for unlocking a specific PR during a repository freeze.
#[derive(Args, Debug)]
pub struct UnlockPrArgs {
    /// PR number to unlock (if omitted, unlocks the current PR when used in PR comments)
    #[arg(long)]
    pub pr_number: Option<u64>,

    /// Reason for unlocking, optional
    #[arg(long)]
    pub reason: Option<String>,
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    s.parse::<DateTime<Utc>>().map_err(|e| e.to_string())
}

/// Parses a duration string into a chrono::Duration.
///
/// Supports both simple format (e.g., "2h", "30m") and ISO 8601 format (e.g., "PT2H30M").
///
/// # Arguments
///
/// * `duration_str` - The duration string to parse
///
/// # Supported Simple Formats
///
/// * `<number>s` - seconds (e.g., "45s")
/// * `<number>m` - minutes (e.g., "30m")
/// * `<number>h` - hours (e.g., "2h")
/// * `<number>d` - days (e.g., "1d")
///
/// # Supported ISO 8601 Formats
///
/// * `P<number>D` - days (e.g., "P1D")
/// * `PT<number>H` - hours (e.g., "PT2H")
/// * `PT<number>M` - minutes (e.g., "PT30M")
/// * `PT<number>S` - seconds (e.g., "PT45S")
/// * Combined formats (e.g., "PT2H30M", "P1DT2H30M")
///
/// # Returns
///
/// * `Ok(Duration)` - Successfully parsed duration
/// * `Err(ParseError::InvalidDuration)` - Invalid duration format
fn parse_duration_2(duration_str: &str) -> Result<chrono::Duration, String> {
    let duration_str = duration_str.trim_matches('"');

    // Handle common duration formats like "2h", "30m", "1d", "45s"
    let duration_regex = regex::Regex::new(r"^(\d+)([smhd])$").unwrap();

    if let Some(captures) = duration_regex.captures(duration_str) {
        let value: i64 = captures[1].parse().map_err(|_| duration_str.to_string())?;

        let unit = &captures[2];
        let duration = match unit {
            "s" => chrono::Duration::seconds(value),
            "m" => chrono::Duration::minutes(value),
            "h" => chrono::Duration::hours(value),
            "d" => chrono::Duration::days(value),
            _ => return Err(duration_str.to_string()),
        };

        Ok(duration)
    } else {
        // Try to parse as ISO 8601 duration (e.g., "PT2H30M")
        parse_iso8601_duration(duration_str)
    }
}

/// Parses an ISO 8601 duration string into a chrono::Duration.
///
/// Handles ISO 8601 duration format: P[n]Y[n]M[n]DT[n]H[n]M[n]S
/// Currently supports days (D), hours (H), minutes (M), and seconds (S).
///
/// # Arguments
///
/// * `duration_str` - The ISO 8601 duration string (must start with 'P')
///
/// # Examples
///
/// * `"P1D"` - 1 day
/// * `"PT2H"` - 2 hours  
/// * `"PT30M"` - 30 minutes
/// * `"PT2H30M"` - 2 hours and 30 minutes
/// * `"P1DT2H30M"` - 1 day, 2 hours, and 30 minutes
///
/// # Returns
///
/// * `Ok(Duration)` - Successfully parsed ISO 8601 duration
/// * `Err(ParseError::InvalidDuration)` - Invalid ISO 8601 format
fn parse_iso8601_duration(duration_str: &str) -> Result<chrono::Duration, String> {
    // Basic ISO 8601 duration parsing for formats like PT2H30M, P1D, etc.
    if !duration_str.starts_with('P') {
        return Err(duration_str.to_string());
    }

    let mut total_seconds = 0i64;
    let chars = duration_str.chars().skip(1); // Skip 'P'
    let mut current_number = String::new();
    let mut in_time_section = false;

    for c in chars {
        match c {
            'T' => {
                in_time_section = true;
            }
            '0'..='9' => {
                current_number.push(c);
            }
            'D' if !in_time_section => {
                if let Ok(days) = current_number.parse::<i64>() {
                    total_seconds += days * 24 * 60 * 60;
                }
                current_number.clear();
            }
            'H' if in_time_section => {
                if let Ok(hours) = current_number.parse::<i64>() {
                    total_seconds += hours * 60 * 60;
                }
                current_number.clear();
            }
            'M' if in_time_section => {
                if let Ok(minutes) = current_number.parse::<i64>() {
                    total_seconds += minutes * 60;
                }
                current_number.clear();
            }
            'S' if in_time_section => {
                if let Ok(seconds) = current_number.parse::<i64>() {
                    total_seconds += seconds;
                }
                current_number.clear();
            }
            _ => {
                return Err(duration_str.to_string());
            }
        }
    }

    Ok(chrono::Duration::seconds(total_seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn parse_cli(args: &[&str]) -> Cli {
        let mut argv = vec!["mybin".to_string()];
        argv.extend(args.iter().map(|s| s.to_string()));
        Cli::parse_from(argv)
    }

    #[test]
    fn test_freeze_command() {
        // Basic freeze without arguments
        let cli = parse_cli(&["freeze"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert!(args.repos.is_empty());
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with duration
        let cli = parse_cli(&["freeze", "--duration", "2h"]);
        match cli.command {
            Command::Freeze(args) => {
                assert_eq!(args.duration.unwrap(), Duration::hours(2));
                assert!(args.reason.is_none());
                assert!(args.repos.is_empty());
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with reason
        let cli = parse_cli(&["freeze", "--reason", "maintenance"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert_eq!(args.reason.unwrap(), "maintenance");
                assert!(args.repos.is_empty());
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with both duration and reason
        let cli = parse_cli(&["freeze", "--duration", "2h", "--reason", "maintenance"]);
        match cli.command {
            Command::Freeze(args) => {
                assert_eq!(args.duration.unwrap(), Duration::hours(2));
                assert_eq!(args.reason.unwrap(), "maintenance");
                assert!(args.repos.is_empty());
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with single repo
        let cli = parse_cli(&["freeze", "--repo", "repo1"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1"]);
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with comma-separated repos
        let cli = parse_cli(&["freeze", "--repo", "repo1,repo2,repo3"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1", "repo2", "repo3"]);
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with multiple --repo flags
        let cli = parse_cli(&["freeze", "--repo", "repo1", "--repo", "repo2"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1", "repo2"]);
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with repos and other options
        let cli = parse_cli(&[
            "freeze",
            "--duration",
            "2h",
            "--reason",
            "maintenance",
            "--repo",
            "repo1,repo2",
        ]);
        match cli.command {
            Command::Freeze(args) => {
                assert_eq!(args.duration.unwrap(), Duration::hours(2));
                assert_eq!(args.reason.unwrap(), "maintenance");
                assert_eq!(args.repos, vec!["repo1", "repo2"]);
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with branch
        let cli = parse_cli(&["freeze", "--branch", "main"]);
        match cli.command {
            Command::Freeze(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert!(args.repos.is_empty());
                assert_eq!(args.branch.unwrap(), "main");
            }
            _ => panic!("Expected Freeze command"),
        }

        // Freeze with branch and other options
        let cli = parse_cli(&[
            "freeze",
            "--branch",
            "main",
            "--duration",
            "2h",
            "--reason",
            "deploy",
        ]);
        match cli.command {
            Command::Freeze(args) => {
                assert_eq!(args.duration.unwrap(), Duration::hours(2));
                assert_eq!(args.reason.unwrap(), "deploy");
                assert!(args.repos.is_empty());
                assert_eq!(args.branch.unwrap(), "main");
            }
            _ => panic!("Expected Freeze command"),
        }
    }

    #[test]
    fn test_freeze_all_command() {
        // Similar structure as freeze command tests
        let cli = parse_cli(&["freeze-all", "--duration", "1d", "--reason", "upgrade"]);
        match cli.command {
            Command::FreezeAll(args) => {
                assert_eq!(args.duration.unwrap(), Duration::days(1));
                assert_eq!(args.reason.unwrap(), "upgrade");
                assert!(args.repos.is_empty());
            }
            _ => panic!("Expected FreezeAll command"),
        }

        // FreezeAll with single repo
        let cli = parse_cli(&["freeze-all", "--repo", "repo1"]);
        match cli.command {
            Command::FreezeAll(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1"]);
            }
            _ => panic!("Expected FreezeAll command"),
        }

        // FreezeAll with comma-separated repos
        let cli = parse_cli(&["freeze-all", "--repo", "repo1,repo2,repo3"]);
        match cli.command {
            Command::FreezeAll(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1", "repo2", "repo3"]);
            }
            _ => panic!("Expected FreezeAll command"),
        }

        // FreezeAll with multiple --repo flags
        let cli = parse_cli(&["freeze-all", "--repo", "repo1", "--repo", "repo2"]);
        match cli.command {
            Command::FreezeAll(args) => {
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
                assert_eq!(args.repos, vec!["repo1", "repo2"]);
            }
            _ => panic!("Expected FreezeAll command"),
        }
    }

    #[test]
    fn test_unfreeze_command() {
        // Basic unfreeze without reason
        let cli = parse_cli(&["unfreeze"]);
        match cli.command {
            Command::Unfreeze(args) => {
                assert!(args.reason.is_none());
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Unfreeze command"),
        }

        // Unfreeze with reason
        let cli = parse_cli(&["unfreeze", "--reason", "emergency resolved"]);
        match cli.command {
            Command::Unfreeze(args) => {
                assert_eq!(args.reason.unwrap(), "emergency resolved");
                assert!(args.branch.is_none());
            }
            _ => panic!("Expected Unfreeze command"),
        }

        // Unfreeze with branch
        let cli = parse_cli(&["unfreeze", "--branch", "main"]);
        match cli.command {
            Command::Unfreeze(args) => {
                assert!(args.reason.is_none());
                assert_eq!(args.branch.unwrap(), "main");
            }
            _ => panic!("Expected Unfreeze command"),
        }

        // Unfreeze with branch and reason
        let cli = parse_cli(&["unfreeze", "--branch", "develop", "--reason", "rollback complete"]);
        match cli.command {
            Command::Unfreeze(args) => {
                assert_eq!(args.reason.unwrap(), "rollback complete");
                assert_eq!(args.branch.unwrap(), "develop");
            }
            _ => panic!("Expected Unfreeze command"),
        }
    }

    #[test]
    fn test_status_command() {
        // Status with single repo
        let cli = parse_cli(&["status", "--repos", "repo1"]);
        match cli.command {
            Command::Status(args) => {
                assert_eq!(args.repos, vec!["repo1"]);
            }
            _ => panic!("Expected Status command"),
        }

        // Status with multiple repos
        let cli = parse_cli(&["status", "--repos", "repo1,repo2,repo3"]);
        match cli.command {
            Command::Status(args) => {
                assert_eq!(args.repos, vec!["repo1", "repo2", "repo3"]);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_schedule_freeze_command() {
        let now = Utc::now();
        let future = now + Duration::hours(2);
        let from_str = now.to_rfc3339();
        let to_str = future.to_rfc3339();

        // Schedule freeze with from only
        let cli = parse_cli(&["schedule-freeze", "--from", &from_str]);
        match cli.command {
            Command::ScheduleFreeze(args) => {
                assert_eq!(args.from.to_rfc3339(), from_str);
                assert!(args.to.is_none());
                assert!(args.duration.is_none());
                assert!(args.reason.is_none());
            }
            _ => panic!("Expected ScheduleFreeze command"),
        }

        // Schedule freeze with all arguments
        let cli = parse_cli(&[
            "schedule-freeze",
            "--from",
            &from_str,
            "--to",
            &to_str,
            "--duration",
            "2h",
            "--reason",
            "maintenance",
        ]);
        match cli.command {
            Command::ScheduleFreeze(args) => {
                assert_eq!(args.from.to_rfc3339(), from_str);
                assert_eq!(args.to.unwrap().to_rfc3339(), to_str);
                assert_eq!(args.duration.unwrap(), Duration::hours(2));
                assert_eq!(args.reason.unwrap(), "maintenance");
            }
            _ => panic!("Expected ScheduleFreeze command"),
        }
    }

    #[test]
    fn test_duration_parsing() {
        // Test simple duration formats
        assert_eq!(parse_duration_2("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration_2("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration_2("45s").unwrap(), Duration::seconds(45));
        assert_eq!(parse_duration_2("1d").unwrap(), Duration::days(1));

        // Test ISO 8601 duration formats
        assert_eq!(parse_duration_2("PT2H").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration_2("PT30M").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration_2("P1D").unwrap(), Duration::days(1));
        assert_eq!(
            parse_duration_2("PT2H30M").unwrap(),
            Duration::hours(2) + Duration::minutes(30)
        );
        assert_eq!(
            parse_duration_2("P1DT2H30M").unwrap(),
            Duration::days(1) + Duration::hours(2) + Duration::minutes(30)
        );
    }

    #[test]
    fn test_unlock_pr_command() {
        // Basic unlock-pr without arguments
        let cli = parse_cli(&["unlock-pr"]);
        match cli.command {
            Command::UnlockPr(args) => {
                assert!(args.pr_number.is_none());
                assert!(args.reason.is_none());
            }
            _ => panic!("Expected UnlockPr command"),
        }

        // Unlock-pr with pr-number
        let cli = parse_cli(&["unlock-pr", "--pr-number", "123"]);
        match cli.command {
            Command::UnlockPr(args) => {
                assert_eq!(args.pr_number.unwrap(), 123);
                assert!(args.reason.is_none());
            }
            _ => panic!("Expected UnlockPr command"),
        }

        // Unlock-pr with reason
        let cli = parse_cli(&["unlock-pr", "--reason", "emergency fix"]);
        match cli.command {
            Command::UnlockPr(args) => {
                assert!(args.pr_number.is_none());
                assert_eq!(args.reason.unwrap(), "emergency fix");
            }
            _ => panic!("Expected UnlockPr command"),
        }

        // Unlock-pr with both pr-number and reason
        let cli = parse_cli(&[
            "unlock-pr",
            "--pr-number",
            "456",
            "--reason",
            "critical hotfix",
        ]);
        match cli.command {
            Command::UnlockPr(args) => {
                assert_eq!(args.pr_number.unwrap(), 456);
                assert_eq!(args.reason.unwrap(), "critical hotfix");
            }
            _ => panic!("Expected UnlockPr command"),
        }
    }
}
