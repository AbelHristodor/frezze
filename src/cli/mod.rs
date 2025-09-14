use std::net::Ipv4Addr;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    Webhook {
        #[command(subcommand)]
        command: WebhookCommands,
    },
    Refresh {
        #[command(subcommand)]
        command: RefreshCommands,
    },
}

#[derive(Subcommand)]
pub enum ServerCommands {
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
        /// Database URL
        #[arg(
            short,
            long,
            default_value = "postgres://postgres:postgres@localhost:5432/postgres",
            env("DATABASE_URL")
        )]
        database_url: String,
        /// Github App ID
        #[arg(long, env("GITHUB_APP_ID"), default_value = "0")]
        gh_app_id: u64,
        /// Github App Private Key Path
        #[arg(
            long,
            env("GITHUB_APP_PRIVATE_KEY_PATH"),
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
            // TODO: remove this
            default_value = "gh_app_private_key.pem",
        )]
        gh_private_key_path: Option<String>,
        /// Github App Private Key in Base64 format
        #[arg(long,
            env("GITHUB_APP_PRIVATE_KEY_BASE64"),
            value_name = "BASE64",
            value_hint = clap::ValueHint::Other,
            required_unless_present("gh_private_key_path"),
        )]
        gh_private_key_base64: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum WebhookCommands {
    Start,
}

#[derive(Subcommand)]
pub enum RefreshCommands {
    /// Refresh all open PRs to sync with current freeze status
    All {
        /// Database URL
        #[arg(
            short,
            long,
            default_value = "postgres://postgres:postgres@localhost:5432/postgres",
            env("DATABASE_URL")
        )]
        database_url: String,
        /// Github App ID
        #[arg(long, env("GITHUB_APP_ID"), default_value = "0")]
        gh_app_id: u64,
        /// Github App Private Key Path
        #[arg(
            long,
            env("GITHUB_APP_PRIVATE_KEY_PATH"),
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
        )]
        gh_private_key_path: Option<String>,
        /// Github App Private Key in Base64 format
        #[arg(long,
            env("GITHUB_APP_PRIVATE_KEY_BASE64"),
            value_name = "BASE64",
            value_hint = clap::ValueHint::Other,
            required_unless_present("gh_private_key_path"),
        )]
        gh_private_key_base64: Option<String>,
    },
    /// Refresh PRs for a specific repository
    Repository {
        /// Repository in format owner/repo
        #[arg(short, long)]
        repository: String,
        /// Installation ID
        #[arg(short, long)]
        installation_id: i64,
        /// Database URL
        #[arg(
            short,
            long,
            default_value = "postgres://postgres:postgres@localhost:5432/postgres",
            env("DATABASE_URL")
        )]
        database_url: String,
        /// Github App ID
        #[arg(long, env("GITHUB_APP_ID"), default_value = "0")]
        gh_app_id: u64,
        /// Github App Private Key Path
        #[arg(
            long,
            env("GITHUB_APP_PRIVATE_KEY_PATH"),
            value_name = "PATH",
            value_hint = clap::ValueHint::FilePath,
        )]
        gh_private_key_path: Option<String>,
        /// Github App Private Key in Base64 format
        #[arg(long,
            env("GITHUB_APP_PRIVATE_KEY_BASE64"),
            value_name = "BASE64",
            value_hint = clap::ValueHint::Other,
            required_unless_present("gh_private_key_path"),
        )]
        gh_private_key_base64: Option<String>,
    },
}
