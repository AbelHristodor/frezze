use sqlx::{migrate::Migrator, pool::PoolOptions};
use std::path::Path;
use tracing::info;

pub mod freeze;
pub mod models;

/// Database connection details
pub struct Database {
    url: String,
    max_conn: u32,
    migrations_path: String,
    pub conn: Option<sqlx::Pool<sqlx::Postgres>>,
}

impl Database {
    pub fn new(url: &str, migrations_path: &str, max_conn: u32) -> Self {
        Database {
            url: url.into(),
            max_conn,
            migrations_path: migrations_path.into(),
            conn: None,
        }
    }

    pub async fn connect(mut self) -> Result<Self, anyhow::Error> {
        let pool = PoolOptions::<sqlx::Postgres>::new()
            .max_connections(self.max_conn)
            .connect(&self.url)
            .await?;
        self.conn = Some(pool);
        Ok(self)
    }

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

    pub fn get_connection(&self) -> Result<&sqlx::Pool<sqlx::Postgres>, anyhow::Error> {
        self.conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))
    }
}
