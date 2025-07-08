use sqlx::pool::PoolOptions;

pub mod models;

/// Database connection details
pub struct Database {
    url: String,
    max_conn: u32,
    pub conn: Option<sqlx::Pool<sqlx::Postgres>>,
}

impl Database {
    pub fn new(url: String, max_conn: u32) -> Self {
        Database {
            url,
            max_conn,
            conn: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), anyhow::Error> {
        let pool = PoolOptions::<sqlx::Postgres>::new()
            .max_connections(self.max_conn)
            .connect(&self.url)
            .await?;

        self.conn = Some(pool);
        Ok(())
    }

    pub fn get_connection(&self) -> Result<&sqlx::Pool<sqlx::Postgres>, anyhow::Error> {
        self.conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))
    }
}
