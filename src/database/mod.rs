//! Database module for Frezze application.
//!
//! This module provides database connectivity, migration management, and data models
//! for the Frezze GitHub repository freeze management system. It uses PostgreSQL
//! as the backend database with SQLx for async database operations.
//!
//! # Modules
//!
//! - [`freeze`] - CRUD operations for freeze records, permissions, and command logs
//! - [`models`] - Data structures representing database entities
//!
//! # Example
//!
//! ```rust,no_run
//! use frezze::database::Database;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let db = Database::new(
//!     "postgresql://user:pass@localhost/frezze",
//!     "./migrations",
//!     10
//! )
//! .connect()
//! .await?
//! .migrate()
//! .await?;
//!
//! let pool = db.get_connection()?;
//! # Ok(())
//! # }
//! ```

use sqlx::{migrate::Migrator, pool::PoolOptions};
use std::path::Path;
use tracing::info;

pub mod freeze;
pub mod models;

/// Database connection manager for the Frezze application.
///
/// Handles PostgreSQL connection pooling, migrations, and provides
/// access to the database connection pool for other components.
///
/// # Fields
///
/// - `url` - PostgreSQL connection string
/// - `max_conn` - Maximum number of connections in the pool
/// - `migrations_path` - Path to SQL migration files
/// - `conn` - Optional connection pool (available after calling `connect()`)
#[derive(Debug)]
pub struct Database {
    url: String,
    max_conn: u32,
    migrations_path: String,
    pub conn: Option<sqlx::Pool<sqlx::Postgres>>,
}

impl Database {
    /// Gets a reference to the database connection pool.
    ///
    /// # Returns
    ///
    /// Returns a reference to the PostgreSQL connection pool if available.
    ///
    /// # Panics
    ///
    /// Panics if the database connection has not been established.
    /// Call `connect()` first to establish the connection.
    pub fn pool(&self) -> &sqlx::Pool<sqlx::Postgres> {
        self.conn.as_ref().expect("Database connection not established")
    }
}

impl Database {
    /// Creates a new Database instance with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - PostgreSQL connection string (e.g., "postgresql://user:pass@localhost/db")
    /// * `migrations_path` - Path to directory containing SQL migration files
    /// * `max_conn` - Maximum number of connections in the connection pool
    ///
    /// # Returns
    ///
    /// Returns a new `Database` instance. Call `connect()` to establish the connection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use frezze::database::Database;
    ///
    /// let db = Database::new(
    ///     "postgresql://user:pass@localhost/frezze",
    ///     "./migrations",
    ///     10
    /// );
    /// ```
    pub fn new(url: &str, migrations_path: &str, max_conn: u32) -> Self {
        Database {
            url: url.into(),
            max_conn,
            migrations_path: migrations_path.into(),
            conn: None,
        }
    }

    /// Creates a mock Database instance for testing.
    ///
    /// This creates a Database with no actual connection, useful for unit tests
    /// where database operations are not required.
    #[cfg(test)]
    pub fn new_mock() -> Self {
        Database {
            url: "mock://database".to_string(),
            max_conn: 1,
            migrations_path: "".to_string(),
            conn: None,
        }
    }

    /// Establishes a connection to the PostgreSQL database.
    ///
    /// Creates a connection pool with the configured maximum connections
    /// and tests the database connectivity.
    ///
    /// # Returns
    ///
    /// Returns `Self` on successful connection, or an error if connection fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The database URL is invalid
    /// - The database server is unreachable
    /// - Authentication fails
    /// - Connection pool creation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::Database;
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::new("postgresql://user:pass@localhost/frezze", "./migrations", 10)
    ///     .connect()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(mut self) -> Result<Self, anyhow::Error> {
        let pool = PoolOptions::<sqlx::Postgres>::new()
            .max_connections(self.max_conn)
            .connect(&self.url)
            .await?;
        self.conn = Some(pool);
        Ok(self)
    }

    /// Runs database migrations to ensure schema is up to date.
    ///
    /// Applies all pending SQL migrations from the configured migrations directory.
    /// This should be called after `connect()` and before using the database.
    ///
    /// # Returns
    ///
    /// Returns `Self` on successful migration, or an error if migration fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Database connection is not established (call `connect()` first)
    /// - Migration files are not found or invalid
    /// - SQL migration execution fails
    /// - Database permissions are insufficient
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::Database;
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::new("postgresql://user:pass@localhost/frezze", "./migrations", 10)
    ///     .connect()
    ///     .await?
    ///     .migrate()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn migrate(self) -> Result<Self, anyhow::Error> {
        let conn = match self.conn {
            Some(ref pool) => pool.clone(),
            None => return Err(anyhow::anyhow!("Database connection not established")),
        };

        Migrator::new(Path::new(&self.migrations_path))
            .await?
            .run(&conn)
            .await?;
        info!("Database migrations applied successfully");
        Ok(self)
    }

    /// Gets a reference to the database connection pool.
    ///
    /// # Returns
    ///
    /// Returns a reference to the PostgreSQL connection pool if available.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection has not been established.
    /// Call `connect()` first to establish the connection.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::Database;
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::new("postgresql://user:pass@localhost/frezze", "./migrations", 10)
    ///     .connect()
    ///     .await?;
    ///
    /// let pool = db.get_connection()?;
    /// // Use pool for database operations
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_connection(&self) -> Result<&sqlx::Pool<sqlx::Postgres>, anyhow::Error> {
        self.conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))
    }
}
