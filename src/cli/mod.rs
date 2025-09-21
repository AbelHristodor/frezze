//! Command-line interface for the Frezze GitHub repository freeze bot.
//!
//! This module defines the CLI structure using clap for parsing command-line arguments.
//! It provides commands for running the server, managing webhooks, and refreshing PR status.

use std::net::Ipv4Addr;

use clap::{Parser, Subcommand};

/// Main CLI structure for the Frezze application.
///
/// Provides a command-line interface for managing GitHub repository freezes,
/// running the server, and performing maintenance operations.
#[derive(Parser)]
#[command(version, about, long_about = None, propagate_version = true)]
pub struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level commands available in the Frezze CLI.
///
/// Each command provides different functionality for managing the Frezze application
/// and GitHub repository freezes.
#[derive(Subcommand)]
pub enum Commands {
    /// Server management commands
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Webhook management commands
    Webhook {
        #[command(subcommand)]
        command: WebhookCommands,
    },
}

/// Commands for managing the Frezze server.
///
/// Provides functionality to start and configure the web server that handles
/// GitHub webhooks and freeze operations.
#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start the Frezze web server
    Start {
        /// The server address to bind to
        #[arg(short, long, default_value = "0.0.0.0", env("SERVER_ADDRES"))]
        address: Ipv4Addr,
        /// The port to run the server on
        #[arg(short, long, default_value = "8080", env("SERVER_PORT"))]
        port: u16,
        /// Path to the database migrations directory
        #[arg(short,
            long,
            value_name = "PATH",
            value_hint = clap::ValueHint::DirPath,
            env("DATABASE_MIGRATIONS_PATH"),
            default_value = "migrations")]
        migrations_path: String,
        /// Database URL for connection
        #[arg(short, long, default_value = "my_db", env("DATABASE_URL"))]
        database_url: String,
        /// GitHub App ID for API authentication
        #[arg(long, env("GITHUB_APP_ID"), default_value = "0")]
        gh_app_id: u64,
        /// Path to GitHub App private key file
        #[arg(
            long,
            env("GITHUB_APP_PRIVATE_KEY_PATH"),
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
            // TODO: remove this
            default_value = "gh_app_private_key.pem",
        )]
        gh_private_key_path: Option<String>,
        /// GitHub App private key in Base64 format (alternative to file path)
        #[arg(long,
            env("GITHUB_APP_PRIVATE_KEY_BASE64"),
            value_name = "BASE64",
            value_hint = clap::ValueHint::Other,
            required_unless_present("gh_private_key_path"),
        )]
        gh_private_key_base64: Option<String>,
        /// Path to user permissions configuration file (YAML)
        #[arg(
            long,
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
            env("USER_PERMISSIONS_CONFIG"),
            help = "Path to YAML file containing user permissions configuration"
        )]
        user_config: Option<String>,
    },
}

/// Commands for managing GitHub webhooks.
///
/// Provides functionality to start and manage webhook processing for GitHub events.
#[derive(Subcommand)]
pub enum WebhookCommands {
    /// Start the webhook processing service
    Start,
}
