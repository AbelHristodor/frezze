use chrono::{DateTime, Utc};
use tracing::error;

pub enum Command {
    Freeze {
        duration: Option<chrono::Duration>,
        reason: Option<String>,
    },
    FreezeAll {
        duration: Option<chrono::Duration>,
        reason: Option<String>,
    },
    Unfreeze {
        reason: Option<String>,
    },
    UnfreezeAll {
        reason: Option<String>,
    },
    Status {
        repos: Vec<String>,
    },
    Help,
    ScheduleFreeze {
        from: chrono::DateTime<chrono::Utc>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum ParseError {
    InvalidCommand(String),
    InvalidDuration(String),
    InvalidDateTime(String),
    MissingRequiredArgument(String),
    InvalidArgument(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidCommand(cmd) => write!(f, "Invalid command: {cmd}"),
            ParseError::InvalidDuration(dur) => write!(f, "Invalid duration: {dur}"),
            ParseError::InvalidDateTime(dt) => write!(f, "Invalid datetime: {dt}"),
            ParseError::MissingRequiredArgument(arg) => {
                write!(f, "Missing required argument: {arg}")
            }
            ParseError::InvalidArgument(arg) => write!(f, "Invalid argument: {arg}"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct CommandParser;

impl CommandParser {
    /// Creates a new instance of the command parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a quoted argument from the command line arguments.
    ///
    /// This function handles both quoted and unquoted arguments. For quoted arguments,
    /// it reconstructs the original string by joining multiple array elements that
    /// were split by whitespace during tokenization.
    ///
    /// # Arguments
    ///
    /// * `args` - The array of command line arguments
    /// * `start_idx` - The index to start parsing from
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The parsed string (with quotes removed if present)
    /// * The next index to continue parsing from
    ///
    /// # Examples
    ///
    /// ```
    /// let args = ["\"hello", "world\""];
    /// let (result, next_idx) = parser.parse_quoted_arg(&args, 0);
    /// assert_eq!(result, "hello world");
    /// assert_eq!(next_idx, 2);
    /// ```
    fn parse_quoted_arg(&self, args: &[&str], start_idx: usize) -> (String, usize) {
        let mut parts = Vec::new();
        let mut j = start_idx;

        if args[j].starts_with('"') {
            parts.push(args[j].trim_start_matches('"'));
            j += 1;

            while j < args.len() && !args[j - 1].ends_with('"') {
                parts.push(args[j]);
                j += 1;
            }

            if let Some(last) = parts.last_mut() {
                *last = last.trim_end_matches('"');
            }
        } else {
            parts.push(args[j]);
            j += 1;
        }

        (parts.join(" "), j)
    }

    /// Parses a command string into a structured Command enum.
    ///
    /// This is the main entry point for command parsing. It handles all supported
    /// freeze bot commands and their arguments.
    ///
    /// # Arguments
    ///
    /// * `input` - The raw command string (must start with '/')
    ///
    /// # Returns
    ///
    /// * `Ok(Command)` - Successfully parsed command
    /// * `Err(ParseError)` - Parse error with details
    ///
    /// # Supported Commands
    ///
    /// * `/freeze` - Freeze repositories with optional duration and reason
    /// * `/freeze-all` - Freeze all repositories in organization
    /// * `/unfreeze` - Unfreeze repositories with optional reason
    /// * `/unfreeze-all` - Unfreeze all repositories in organization
    /// * `/freeze-status` - Show current freeze status
    /// * `/freeze-help` - Show command help
    /// * `/schedule-freeze` - Schedule a freeze for later execution
    ///
    /// # Examples
    ///
    /// ```
    /// let parser = CommandParser::new();
    /// let cmd = parser.parse("/freeze --duration 2h --reason \"Release v1.0\"").unwrap();
    /// ```
    pub fn parse(&self, input: &str) -> Result<Command, ParseError> {
        let input = input.trim();
        if !input.starts_with('/') {
            return Err(ParseError::InvalidCommand(
                "Commands must start with '/'".to_string(),
            ));
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ParseError::InvalidCommand("Empty command".to_string()));
        }

        match parts[0] {
            "/freeze" => self.parse_freeze(&parts[1..]),
            "/freeze-all" => self.parse_freeze_all(&parts[1..]),
            "/unfreeze" => self.parse_unfreeze(&parts[1..]),
            "/unfreeze-all" => self.parse_unfreeze_all(&parts[1..]),
            "/freeze-status" => self.parse_status(&parts[1..]),
            "/freeze-help" => Ok(Command::Help),
            "/schedule-freeze" => self.parse_schedule_freeze(&parts[1..]),
            cmd => Err(ParseError::InvalidCommand(format!(
                "Unknown command: {cmd}",
            ))),
        }
    }

    /// Parses arguments for the `/freeze` command.
    ///
    /// The freeze command allows freezing repositories with optional duration and reason.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/freeze` command
    ///
    /// # Supported Arguments
    ///
    /// * `--duration <duration>` - How long to freeze (e.g., "2h", "30m", "1d", "PT2H30M")
    /// * `--reason <reason>` - Reason for freezing (supports quoted strings with spaces)
    ///
    /// # Returns
    ///
    /// * `Ok(Command::Freeze)` - Successfully parsed freeze command
    /// * `Err(ParseError)` - Parse error for invalid arguments or missing required values
    ///
    /// # Examples
    ///
    /// ```
    /// // Basic freeze
    /// parse_freeze(&[]) -> Command::Freeze { duration: None, reason: None }
    ///
    /// // With duration
    /// parse_freeze(&["--duration", "2h"]) -> Command::Freeze { duration: Some(2h), reason: None }
    ///
    /// // With quoted reason
    /// parse_freeze(&["--reason", "\"Release", "v1.0\""]) -> Command::Freeze { reason: Some("Release v1.0") }
    /// ```
    fn parse_freeze(&self, args: &[&str]) -> Result<Command, ParseError> {
        println!("{:?}", args);
        let mut duration = None;
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            println!("{:?}", args[i]);
            match args[i] {
                "--duration" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("duration".to_string()));
                    }
                    duration = Some(self.parse_duration(args[i + 1])?);
                    i += 2;
                }
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    let (parsed_reason, next_idx) = self.parse_quoted_arg(args, i + 1);
                    reason = Some(parsed_reason);
                    i = next_idx;
                }
                arg if arg.starts_with("--") => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
                _ => {
                    error!("Wrong command");
                }
            }
        }

        Ok(Command::Freeze { duration, reason })
    }

    /// Parses arguments for the `/freeze-all` command.
    ///
    /// The freeze-all command freezes all repositories in the organization.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/freeze-all` command
    ///
    /// # Supported Arguments
    ///
    /// * `--duration <duration>` - How long to freeze all repositories
    /// * `--reason <reason>` - Reason for the organization-wide freeze
    ///
    /// # Returns
    ///
    /// * `Ok(Command::FreezeAll)` - Successfully parsed freeze-all command
    /// * `Err(ParseError)` - Parse error for invalid arguments
    fn parse_freeze_all(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut duration = None;
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "--duration" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("duration".to_string()));
                    }
                    duration = Some(self.parse_duration(args[i + 1])?);
                    i += 2;
                }
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    let (parsed_reason, next_idx) = self.parse_quoted_arg(args, i + 1);
                    reason = Some(parsed_reason);
                    i = next_idx;
                }
                arg => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
            }
        }

        Ok(Command::FreezeAll { duration, reason })
    }

    /// Parses arguments for the `/unfreeze` command.
    ///
    /// The unfreeze command removes freeze restrictions from repositories.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/unfreeze` command
    ///
    /// # Supported Arguments
    ///
    /// * `--reason <reason>` - Reason for unfreezing (supports quoted strings)
    ///
    /// # Returns
    ///
    /// * `Ok(Command::Unfreeze)` - Successfully parsed unfreeze command
    /// * `Err(ParseError)` - Parse error for invalid arguments
    fn parse_unfreeze(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    let (parsed_reason, next_idx) = self.parse_quoted_arg(args, i + 1);
                    reason = Some(parsed_reason);
                    i = next_idx;
                }
                arg if arg.starts_with("--") => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
                _ => {
                    // If no specific repository is mentioned, unfreeze all
                    error!("Wrong command format");
                }
            }
        }

        Ok(Command::Unfreeze { reason })
    }

    /// Parses arguments for the `/unfreeze-all` command.
    ///
    /// The unfreeze-all command removes freeze restrictions from all repositories in the organization.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/unfreeze-all` command
    ///
    /// # Supported Arguments
    ///
    /// * `--reason <reason>` - Reason for the organization-wide unfreeze
    ///
    /// # Returns
    ///
    /// * `Ok(Command::UnfreezeAll)` - Successfully parsed unfreeze-all command
    /// * `Err(ParseError)` - Parse error for invalid arguments
    fn parse_unfreeze_all(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    let (parsed_reason, next_idx) = self.parse_quoted_arg(args, i + 1);
                    reason = Some(parsed_reason);
                    i = next_idx;
                }
                arg => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
            }
        }

        Ok(Command::UnfreezeAll { reason })
    }

    /// Parses arguments for the `/freeze-status` command.
    ///
    /// The status command shows the current freeze status of repositories.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/freeze-status` command (repository names)
    ///
    /// # Returns
    ///
    /// * `Ok(Command::Status)` - Successfully parsed status command with repository list
    ///
    /// # Examples
    ///
    /// ```
    /// // Status for all repositories
    /// parse_status(&[]) -> Command::Status { repos: [] }
    ///
    /// // Status for specific repositories
    /// parse_status(&["repo1", "repo2"]) -> Command::Status { repos: ["repo1", "repo2"] }
    /// ```
    fn parse_status(&self, args: &[&str]) -> Result<Command, ParseError> {
        let repos = args.iter().map(|s| s.to_string()).collect();
        Ok(Command::Status { repos })
    }

    /// Parses arguments for the `/schedule-freeze` command.
    ///
    /// The schedule-freeze command allows scheduling a freeze to start at a specific time.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments after the `/schedule-freeze` command
    ///
    /// # Required Arguments
    ///
    /// * `--from <datetime>` - When to start the freeze (RFC 3339 format, e.g., "2024-01-15T10:00:00Z")
    ///
    /// # Optional Arguments
    ///
    /// * `--to <datetime>` - When to end the freeze (alternative to --duration)
    /// * `--duration <duration>` - How long the freeze should last (alternative to --to)
    /// * `--reason <reason>` - Reason for the scheduled freeze
    ///
    /// # Returns
    ///
    /// * `Ok(Command::ScheduleFreeze)` - Successfully parsed schedule command
    /// * `Err(ParseError)` - Parse error for missing required arguments or invalid values
    ///
    /// # Notes
    ///
    /// If both `--to` and `--duration` are provided, `--to` takes precedence and duration
    /// is calculated automatically. The `--to` time must be after the `--from` time.
    fn parse_schedule_freeze(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut from_time = None;
        let mut to_time = None;
        let mut reason = None;
        let mut duration = None;
        let mut i = 0;

        while i < args.len() {
            println!("{:?}", args[i]);
            match args[i] {
                "--from" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("from".to_string()));
                    }
                    from_time = Some(self.parse_datetime(args[i + 1])?);
                    i += 2;
                }
                "--to" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("to".to_string()));
                    }
                    to_time = Some(self.parse_datetime(args[i + 1])?);
                    i += 2;
                }
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    let (parsed_reason, next_idx) = self.parse_quoted_arg(args, i + 1);
                    reason = Some(parsed_reason);
                    i = next_idx;
                }
                "--duration" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("duration".to_string()));
                    }
                    duration = Some(self.parse_duration(args[i + 1])?);
                    i += 2;
                }
                arg => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
            }
        }

        let from = from_time.ok_or_else(|| {
            ParseError::MissingRequiredArgument(
                "--from is required for schedule-freeze".to_string(),
            )
        })?;

        // Calculate duration from --to if provided
        let calculated_duration = if let Some(to) = to_time {
            let duration_secs = (to - from).num_seconds();
            if duration_secs <= 0 {
                return Err(ParseError::InvalidDateTime(
                    "'to' time must be after 'from' time".to_string(),
                ));
            }
            Some(chrono::Duration::seconds(duration_secs))
        } else {
            duration
        };

        Ok(Command::ScheduleFreeze {
            from,
            to: to_time,
            duration: calculated_duration,
            reason,
        })
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
    fn parse_duration(&self, duration_str: &str) -> Result<chrono::Duration, ParseError> {
        let duration_str = duration_str.trim_matches('"');

        // Handle common duration formats like "2h", "30m", "1d", "45s"
        let duration_regex = regex::Regex::new(r"^(\d+)([smhd])$").unwrap();

        if let Some(captures) = duration_regex.captures(duration_str) {
            let value: i64 = captures[1]
                .parse()
                .map_err(|_| ParseError::InvalidDuration(duration_str.to_string()))?;

            let unit = &captures[2];
            let duration = match unit {
                "s" => chrono::Duration::seconds(value),
                "m" => chrono::Duration::minutes(value),
                "h" => chrono::Duration::hours(value),
                "d" => chrono::Duration::days(value),
                _ => return Err(ParseError::InvalidDuration(duration_str.to_string())),
            };

            Ok(duration)
        } else {
            // Try to parse as ISO 8601 duration (e.g., "PT2H30M")
            self.parse_iso8601_duration(duration_str)
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
    fn parse_iso8601_duration(&self, duration_str: &str) -> Result<chrono::Duration, ParseError> {
        // Basic ISO 8601 duration parsing for formats like PT2H30M, P1D, etc.
        if !duration_str.starts_with('P') {
            return Err(ParseError::InvalidDuration(duration_str.to_string()));
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
                    return Err(ParseError::InvalidDuration(duration_str.to_string()));
                }
            }
        }

        Ok(chrono::Duration::seconds(total_seconds))
    }

    /// Parses a datetime string into a UTC DateTime.
    ///
    /// Accepts RFC 3339 format (ISO 8601) datetime strings and converts them to UTC.
    ///
    /// # Arguments
    ///
    /// * `datetime_str` - The datetime string to parse (quotes are automatically stripped)
    ///
    /// # Supported Formats
    ///
    /// * `"2024-01-15T10:00:00Z"` - UTC time
    /// * `"2024-01-15T10:00:00+02:00"` - With timezone offset
    /// * `"2024-01-15T10:00:00.123Z"` - With milliseconds
    ///
    /// # Returns
    ///
    /// * `Ok(DateTime<Utc>)` - Successfully parsed datetime in UTC
    /// * `Err(ParseError::InvalidDateTime)` - Invalid datetime format
    ///
    /// # Examples
    ///
    /// ```
    /// parse_datetime("\"2024-01-15T10:00:00Z\"") -> Ok(DateTime<Utc>)
    /// parse_datetime("2024-01-15T10:00:00+02:00") -> Ok(DateTime<Utc>) // Converted to UTC
    /// ```
    fn parse_datetime(&self, datetime_str: &str) -> Result<DateTime<Utc>, ParseError> {
        let datetime_str = datetime_str.trim_matches('"');

        // Try to parse as RFC 3339 format (ISO 8601)
        DateTime::parse_from_rfc3339(datetime_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| ParseError::InvalidDateTime(datetime_str.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_freeze_basic() {
        let parser = CommandParser::new();
        match parser.parse("/freeze").unwrap() {
            Command::Freeze { duration, reason } => {
                assert_eq!(duration, None);
                assert_eq!(reason, None);
            }
            _ => panic!("Expected Freeze command"),
        }
    }

    #[test]
    fn test_freeze_with_duration_and_reason() {
        let parser = CommandParser::new();
        let cmd = String::from("/freeze --duration 2h --reason \"Release v1.2.3\"");
        match parser.parse(&cmd).unwrap() {
            Command::Freeze { duration, reason } => {
                assert_eq!(duration, Some(chrono::Duration::hours(2)));
                assert_eq!(reason, Some("Release v1.2.3".to_string()));
            }
            _ => panic!("Expected Freeze command"),
        }
    }

    #[test]
    fn test_freeze_all() {
        let parser = CommandParser::new();
        match parser.parse("/freeze-all").unwrap() {
            Command::FreezeAll { duration, reason } => {
                assert_eq!(duration, None);
                assert_eq!(reason, None);
            }
            _ => panic!("Expected FreezeAll command"),
        }
    }

    #[test]
    fn test_unfreeze() {
        let parser = CommandParser::new();
        match parser
            .parse("/unfreeze --reason \"Hotfix applied\"")
            .unwrap()
        {
            Command::Unfreeze { reason } => {
                assert_eq!(reason, Some("Hotfix applied".to_string()));
            }
            _ => panic!("Expected Unfreeze command"),
        }
    }

    #[test]
    fn test_unfreeze_all() {
        let parser = CommandParser::new();
        match parser.parse("/unfreeze-all").unwrap() {
            Command::UnfreezeAll { reason } => {
                assert_eq!(reason, None);
            }
            _ => panic!("Expected UnfreezeAll command"),
        }
    }

    #[test]
    fn test_status() {
        let parser = CommandParser::new();
        match parser.parse("/freeze-status").unwrap() {
            Command::Status { repos } => {
                assert_eq!(repos.len(), 0);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_help() {
        let parser = CommandParser::new();
        match parser.parse("/freeze-help").unwrap() {
            Command::Help => {}
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_schedule_freeze() {
        let parser = CommandParser::new();
        match parser.parse("/schedule-freeze --from \"2024-01-15T10:00:00Z\" --to \"2024-01-15T12:00:00Z\" --reason \"Release\"").unwrap() {
            Command::ScheduleFreeze { from, to, duration, reason } => {
                assert_eq!(from, Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap());
                assert_eq!(to, Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap()));
                assert_eq!(duration, Some(chrono::Duration::hours(2)));
                assert_eq!(reason, Some("Release".to_string()));
            }
            _ => panic!("Expected ScheduleFreeze command"),
        }
    }

    #[test]
    fn test_duration_parsing() {
        let parser = CommandParser::new();

        assert_eq!(
            parser.parse_duration("2h").unwrap(),
            chrono::Duration::hours(2)
        );
        assert_eq!(
            parser.parse_duration("30m").unwrap(),
            chrono::Duration::minutes(30)
        );
        assert_eq!(
            parser.parse_duration("45s").unwrap(),
            chrono::Duration::seconds(45)
        );
        assert_eq!(
            parser.parse_duration("1d").unwrap(),
            chrono::Duration::days(1)
        );

        // Test ISO 8601 format
        assert_eq!(
            parser.parse_duration("PT2H30M").unwrap(),
            chrono::Duration::hours(2) + chrono::Duration::minutes(30)
        );
        assert_eq!(
            parser.parse_duration("P1D").unwrap(),
            chrono::Duration::days(1)
        );
    }

    #[test]
    fn test_quoted_reason_parsing() {
        let parser = CommandParser::new();

        // Test quoted reason with spaces
        match parser.parse("/freeze --reason \"Release v1.2.3\"").unwrap() {
            Command::Freeze { reason, .. } => {
                assert_eq!(reason, Some("Release v1.2.3".to_string()));
            }
            _ => panic!("Expected Freeze command"),
        }

        // Test unquoted reason
        match parser.parse("/freeze --reason hotfix").unwrap() {
            Command::Freeze { reason, .. } => {
                assert_eq!(reason, Some("hotfix".to_string()));
            }
            _ => panic!("Expected Freeze command"),
        }

        // Test empty quoted reason
        match parser.parse("/freeze --reason \"\"").unwrap() {
            Command::Freeze { reason, .. } => {
                assert_eq!(reason, Some("".to_string()));
            }
            _ => panic!("Expected Freeze command"),
        }
    }

    #[test]
    fn test_freeze_all_quoted_reason() {
        let parser = CommandParser::new();
        match parser
            .parse("/freeze-all --reason \"Emergency maintenance\"")
            .unwrap()
        {
            Command::FreezeAll { reason, .. } => {
                assert_eq!(reason, Some("Emergency maintenance".to_string()));
            }
            _ => panic!("Expected FreezeAll command"),
        }
    }

    #[test]
    fn test_unfreeze_quoted_reason() {
        let parser = CommandParser::new();
        match parser
            .parse("/unfreeze --reason \"Issue resolved\"")
            .unwrap()
        {
            Command::Unfreeze { reason } => {
                assert_eq!(reason, Some("Issue resolved".to_string()));
            }
            _ => panic!("Expected Unfreeze command"),
        }
    }

    #[test]
    fn test_unfreeze_all_quoted_reason() {
        let parser = CommandParser::new();
        match parser
            .parse("/unfreeze-all --reason \"All clear\"")
            .unwrap()
        {
            Command::UnfreezeAll { reason } => {
                assert_eq!(reason, Some("All clear".to_string()));
            }
            _ => panic!("Expected UnfreezeAll command"),
        }
    }

    #[test]
    fn test_schedule_freeze_quoted_reason() {
        let parser = CommandParser::new();
        match parser.parse("/schedule-freeze --from \"2024-01-15T10:00:00Z\" --reason \"Scheduled maintenance\"").unwrap() {
            Command::ScheduleFreeze { reason, .. } => {
                assert_eq!(reason, Some("Scheduled maintenance".to_string()));
            }
            _ => panic!("Expected ScheduleFreeze command"),
        }
    }

    #[test]
    fn test_complex_quoted_reasons() {
        let parser = CommandParser::new();

        // Test reason with special characters
        match parser
            .parse("/freeze --reason \"Release v2.0.0 - fixes #123 & #456\"")
            .unwrap()
        {
            Command::Freeze { reason, .. } => {
                assert_eq!(
                    reason,
                    Some("Release v2.0.0 - fixes #123 & #456".to_string())
                );
            }
            _ => panic!("Expected Freeze command"),
        }

        // Test multiple arguments with quoted reason
        match parser
            .parse("/freeze --duration 1h --reason \"Critical security patch\"")
            .unwrap()
        {
            Command::Freeze { duration, reason } => {
                assert_eq!(duration, Some(chrono::Duration::hours(1)));
                assert_eq!(reason, Some("Critical security patch".to_string()));
            }
            _ => panic!("Expected Freeze command"),
        }
    }

    #[test]
    fn test_invalid_commands() {
        let parser = CommandParser::new();

        assert!(parser.parse("/invalid-command").is_err());
        assert!(parser.parse("freeze").is_err()); // Missing leading slash
        assert!(parser.parse("/freeze --duration").is_err()); // Missing argument
        assert!(parser.parse("/freeze --invalid-flag").is_err()); // Invalid flag
    }
}
