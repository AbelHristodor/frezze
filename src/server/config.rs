#[derive(Debug)]
pub struct ServerConfig {
    pub address: std::net::Ipv4Addr,
    pub port: u16,
    pub database_url: String,
    pub gh_app_id: u64,
    pub migrations_path: String,
    pub gh_private_key_path: Option<String>,
    pub gh_private_key_base64: Option<String>,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
