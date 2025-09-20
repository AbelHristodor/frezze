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

/// Success message for freeze-all operation
pub fn freeze_all_success(count: usize) -> String {
    format!(
        "## ❄️ All Repositories Frozen\n\n\
        🔒 **Successfully froze {count} repositories**\n\n\
        > 🚨 **Important**: All pull requests and pushes are now blocked for all repositories until unfrozen.\n\n\
        *Use `/unfreeze-all` to lift all freezes when ready.*"
    )
}

/// Partial success message for freeze-all operation
pub fn freeze_all_partial_success(successful: usize, failed: usize, errors: &[String]) -> String {
    let error_list = if errors.len() <= 5 {
        errors.join("\n- ")
    } else {
        format!("{}... and {} more", errors[..5].join("\n- "), errors.len() - 5)
    };
    
    format!(
        "## ⚠️ Partial Freeze Success\n\n\
        ✅ **Successfully froze {successful} repositories**\n\
        ❌ **Failed to freeze {failed} repositories**\n\n\
        > 🚨 **Important**: Successfully frozen repositories are blocked until unfrozen.\n\n\
        **Errors encountered:**\n- {error_list}\n\n\
        *Use `/unfreeze-all` to lift all freezes when ready.*"
    )
}

/// Success message for unfreeze-all operation
pub fn unfreeze_all_success(count: usize) -> String {
    format!(
        "## 🌞 All Repositories Unfrozen\n\n\
        ✅ **Successfully unfroze {count} repositories**\n\n\
        > 🎉 **All systems go**: Pull requests and pushes are now allowed for all repositories.\n\n\
        *All freezes have been successfully lifted.*"
    )
}

/// Partial success message for unfreeze-all operation
pub fn unfreeze_all_partial_success(successful: usize, failed: usize, errors: &[String]) -> String {
    let error_list = if errors.len() <= 5 {
        errors.join("\n- ")
    } else {
        format!("{}... and {} more", errors[..5].join("\n- "), errors.len() - 5)
    };
    
    format!(
        "## ⚠️ Partial Unfreeze Success\n\n\
        ✅ **Successfully unfroze {successful} repositories**\n\
        ❌ **Failed to unfreeze {failed} repositories**\n\n\
        > 🎉 **Partially restored**: Some repositories are now accepting pull requests and pushes.\n\n\
        **Errors encountered:**\n- {error_list}\n\n\
        *Check repository statuses for details.*"
    )
}

/// Error message for status operation
pub fn status_error(error: &str) -> String {
    format!(
        "## ❌ Status Check Failed\n\n\
        🚫 **Failed to get repository status**\n\n\
        ```\n{error}\n```\n\n\
        *Please check your permissions and try again.*"
    )
}

/// Format freeze status table for multiple repositories
pub fn format_status_table(entries: Vec<(String, super::manager::StatusEntry)>) -> String {
    use super::manager::FreezeStatus;
    
    let mut table = String::from("## 📊 Repository Freeze Status\n\n");
    table.push_str("| Repository | Status | Duration | Start | End | Reason |\n");
    table.push_str("|------------|--------|----------|-------|-----|--------|\n");
    
    for (repo_name, entry) in entries {
        let status = match entry.freeze_status {
            FreezeStatus::Active => "🔒 Active",
            FreezeStatus::Scheduled => "⏰ Scheduled",
            FreezeStatus::Off => "🌞 Off",
            FreezeStatus::Error(ref err) => &format!("❌ Error: {}", err),
        };
        
        let duration = entry.duration.unwrap_or_else(|| "-".to_string());
        let start = entry.start.unwrap_or_else(|| "-".to_string());
        let end = entry.end.unwrap_or_else(|| "-".to_string());
        let reason = entry.reason.unwrap_or_else(|| "-".to_string());
        
        table.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            repo_name, status, duration, start, end, reason
        ));
    }
    
    table.push_str("\n*Use `/freeze` or `/unfreeze` to manage individual repositories.*");
    table
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

/// Message displayed when a user is denied access to a command.
///
/// # Arguments
///
/// * `username` - The GitHub username that was denied access
/// * `reason` - The specific reason for the denial
///
/// # Returns
///
/// A formatted markdown message explaining the permission denial
pub fn permission_denied(username: &str, reason: &str) -> String {
    format!(
        "## ❌ Permission Denied\n\n\
        🚫 **Access denied for user `{}`**\n\n\
        **Reason**: {}\n\n\
        *Contact your repository administrator to request access.*",
        username, reason
    )
}

/// Message displayed when permission checking fails due to an error.
///
/// # Arguments
///
/// * `username` - The GitHub username for which permission checking failed
/// * `error` - The error that occurred during permission checking
///
/// # Returns
///
/// A formatted markdown message explaining the permission check failure
pub fn permission_check_failed(username: &str, error: &str) -> String {
    format!(
        "## ❌ Permission Check Failed\n\n\
        🚫 **Unable to verify permissions for user `{}`**\n\n\
        **Error**: {}\n\n\
        *Please try again later or contact support.*",
        username, error
    )
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

    #[test]
    fn test_freeze_all_success_message() {
        let msg = freeze_all_success(5);
        assert!(msg.contains("All Repositories Frozen"));
        assert!(msg.contains("5 repositories"));
        assert!(msg.contains("❄️"));
        assert!(msg.contains("/unfreeze-all"));
    }

    #[test]
    fn test_freeze_all_partial_success_message() {
        let errors = vec!["Error 1".to_string(), "Error 2".to_string()];
        let msg = freeze_all_partial_success(3, 2, &errors);
        assert!(msg.contains("Partial Freeze Success"));
        assert!(msg.contains("3 repositories"));
        assert!(msg.contains("2 repositories"));
        assert!(msg.contains("Error 1"));
        assert!(msg.contains("Error 2"));
        assert!(msg.contains("⚠️"));
    }

    #[test]
    fn test_unfreeze_all_success_message() {
        let msg = unfreeze_all_success(3);
        assert!(msg.contains("All Repositories Unfrozen"));
        assert!(msg.contains("3 repositories"));
        assert!(msg.contains("🌞"));
        assert!(msg.contains("All systems go"));
    }

    #[test]
    fn test_status_error_message() {
        let msg = status_error("Database error");
        assert!(msg.contains("Status Check Failed"));
        assert!(msg.contains("Database error"));
        assert!(msg.contains("❌"));
    }

    #[test]
    fn test_format_status_table() {
        use crate::freezer::manager::{StatusEntry, FreezeStatus};
        
        let entries = vec![
            ("owner/repo1".to_string(), StatusEntry {
                freeze_status: FreezeStatus::Active,
                duration: Some("2h".to_string()),
                start: Some("2023-01-01 10:00:00 UTC".to_string()),
                end: Some("2023-01-01 12:00:00 UTC".to_string()),
                reason: Some("maintenance".to_string()),
            }),
            ("owner/repo2".to_string(), StatusEntry {
                freeze_status: FreezeStatus::Off,
                duration: None,
                start: None,
                end: None,
                reason: None,
            }),
        ];
        
        let table = format_status_table(entries);
        assert!(table.contains("📊 Repository Freeze Status"));
        assert!(table.contains("owner/repo1"));
        assert!(table.contains("owner/repo2"));
        assert!(table.contains("🔒 Active"));
        assert!(table.contains("🌞 Off"));
        assert!(table.contains("maintenance"));
        assert!(table.contains("| Repository | Status |"));
    }

    #[test]
    fn test_permission_denied_message() {
        let msg = permission_denied("testuser", "User role 'contributor' does not have freeze permissions");
        assert!(msg.contains("Permission Denied"));
        assert!(msg.contains("testuser"));
        assert!(msg.contains("contributor"));
        assert!(msg.contains("❌"));
        assert!(msg.contains("🚫"));
        assert!(msg.contains("Contact your repository administrator"));
    }

    #[test]
    fn test_permission_check_failed_message() {
        let msg = permission_check_failed("testuser", "Configuration file not found");
        assert!(msg.contains("Permission Check Failed"));
        assert!(msg.contains("testuser"));
        assert!(msg.contains("Configuration file not found"));
        assert!(msg.contains("❌"));
        assert!(msg.contains("🚫"));
        assert!(msg.contains("try again later"));
    }
}
