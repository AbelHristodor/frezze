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
    pub fn new() -> Self {
        Self
    }

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

    fn parse_freeze(&self, args: &[&str]) -> Result<Command, ParseError> {
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
                    reason = Some(args[i + 1].trim_matches('"').to_string());
                    i += 2;
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
                    reason = Some(args[i + 1].trim_matches('"').to_string());
                    i += 2;
                }
                arg => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
            }
        }

        Ok(Command::FreezeAll { duration, reason })
    }

    fn parse_unfreeze(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    reason = Some(args[i + 1].trim_matches('"').to_string());
                    i += 2;
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

    fn parse_unfreeze_all(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "--reason" => {
                    if i + 1 >= args.len() {
                        return Err(ParseError::MissingRequiredArgument("reason".to_string()));
                    }
                    reason = Some(args[i + 1].trim_matches('"').to_string());
                    i += 2;
                }
                arg => {
                    return Err(ParseError::InvalidArgument(arg.to_string()));
                }
            }
        }

        Ok(Command::UnfreezeAll { reason })
    }

    fn parse_status(&self, args: &[&str]) -> Result<Command, ParseError> {
        let repos = args.iter().map(|s| s.to_string()).collect();
        Ok(Command::Status { repos })
    }

    fn parse_schedule_freeze(&self, args: &[&str]) -> Result<Command, ParseError> {
        let mut from_time = None;
        let mut to_time = None;
        let mut reason = None;
        let mut duration = None;
        let mut i = 0;

        while i < args.len() {
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
                    reason = Some(args[i + 1].trim_matches('"').to_string());
                    i += 2;
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
        match parser
            .parse("/freeze --duration 2h --reason \"Release v1.2.3\"")
            .unwrap()
        {
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
    fn test_invalid_commands() {
        let parser = CommandParser::new();

        assert!(parser.parse("/invalid-command").is_err());
        assert!(parser.parse("freeze").is_err()); // Missing leading slash
        assert!(parser.parse("/freeze --duration").is_err()); // Missing argument
        assert!(parser.parse("/freeze --invalid-flag").is_err()); // Invalid flag
    }
}
