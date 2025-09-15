/// Formatted user-facing messages for Frezze GitHub bot responses.
///
/// All messages use Markdown formatting and maintain the freeze theme
/// with appropriate emojis and professional tone.

/// Success message for repository freeze operation
pub fn freeze_success(repository: &str, duration_str: &str, reason_str: &str) -> String {
    format!(
        "## ❄️ Repository Frozen\n\n\
        🔒 **Repository `{repository}` has been frozen**{duration_str}{reason_str}\n\n\
        > 🚨 **Important**: All pull requests and pushes are now blocked until the freeze is lifted.\n\n\
        *Use `/unfreeze` to lift the freeze when ready.*"
    )
}

/// Error message for repository freeze operation failure
pub fn freeze_error(error: &str) -> String {
    format!(
        "## ❌ Freeze Failed\n\n\
        🚫 **Failed to freeze repository**\n\n\
        ```\n{error}\n```\n\n\
        *Please check your permissions and try again.*"
    )
}

/// Success message for repository unfreeze operation
pub fn unfreeze_success(repository: &str) -> String {
    format!(
        "## 🌞 Repository Unfrozen\n\n\
        ✅ **Repository `{repository}` has been unfrozen**\n\n\
        > 🎉 **All systems go**: Pull requests and pushes are now allowed.\n\n\
        *The freeze has been successfully lifted.*"
    )
}

/// Error message for repository unfreeze operation failure
pub fn unfreeze_error(error: &str) -> String {
    format!(
        "## ❌ Unfreeze Failed\n\n\
        🚫 **Failed to unfreeze repository**\n\n\
        ```\n{error}\n```\n\n\
        *Please check your permissions and try again.*"
    )
}

/// Message for commands not yet implemented
pub fn command_not_implemented() -> String {
    "## ⚠️ Command Not Available\n\n\
    🚧 **This command is not yet implemented**\n\n\
    Available commands:\n\
    - `/freeze` - Freeze the repository\n\
    - `/unfreeze` - Unfreeze the repository\n\n\
    *More commands coming soon!*"
        .to_string()
}

/// Helper function to format duration for display
pub fn format_duration_display(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 {
        format!(" for **{}h {}m**", hours, minutes)
    } else if minutes > 0 {
        format!(" for **{}m**", minutes)
    } else {
        " for a **short duration**".to_string()
    }
}

/// Helper function to format reason for display
pub fn format_reason_display(reason: Option<String>) -> String {
    match reason {
        Some(r) if !r.trim().is_empty() => format!("\n\n**Reason**: _{}_", r.trim()),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_freeze_success_message() {
        let msg = freeze_success(
            "owner/repo",
            " for **2h 30m**",
            "\n\n**Reason**: _Deployment in progress_",
        );
        assert!(msg.contains("Repository Frozen"));
        assert!(msg.contains("owner/repo"));
        assert!(msg.contains("2h 30m"));
        assert!(msg.contains("Deployment in progress"));
        assert!(msg.contains("❄️"));
    }

    #[test]
    fn test_freeze_error_message() {
        let msg = freeze_error("Permission denied");
        assert!(msg.contains("Freeze Failed"));
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("❌"));
    }

    #[test]
    fn test_unfreeze_success_message() {
        let msg = unfreeze_success("owner/repo");
        assert!(msg.contains("Repository Unfrozen"));
        assert!(msg.contains("owner/repo"));
        assert!(msg.contains("🌞"));
    }

    #[test]
    fn test_unfreeze_error_message() {
        let msg = unfreeze_error("Database error");
        assert!(msg.contains("Unfreeze Failed"));
        assert!(msg.contains("Database error"));
        assert!(msg.contains("❌"));
    }

    #[test]
    fn test_command_not_implemented_message() {
        let msg = command_not_implemented();
        assert!(msg.contains("Command Not Available"));
        assert!(msg.contains("/freeze"));
        assert!(msg.contains("/unfreeze"));
        assert!(msg.contains("⚠️"));
    }

    #[test]
    fn test_format_duration_display() {
        assert_eq!(
            format_duration_display(Duration::hours(2) + Duration::minutes(30)),
            " for **2h 30m**"
        );
        assert_eq!(
            format_duration_display(Duration::minutes(45)),
            " for **45m**"
        );
        assert_eq!(
            format_duration_display(Duration::seconds(30)),
            " for a **short duration**"
        );
    }

    #[test]
    fn test_format_reason_display() {
        assert_eq!(
            format_reason_display(Some("Test reason".to_string())),
            "\n\n**Reason**: _Test reason_"
        );
        assert_eq!(format_reason_display(Some("  ".to_string())), "");
        assert_eq!(format_reason_display(None), "");
    }
}
