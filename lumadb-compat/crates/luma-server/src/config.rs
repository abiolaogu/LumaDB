use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    // General
    pub general: GeneralConfig,
    
    // Adapters
    #[cfg(feature = "postgres")]
    pub postgres: Option<PostgresConfig>,
    #[cfg(feature = "mysql")]
    pub mysql: Option<MySqlConfig>,
    #[cfg(feature = "cassandra")]
    pub cassandra: Option<CassandraConfig>,
    #[cfg(feature = "mongodb")]
    pub mongodb: Option<MongoDbConfig>,
    
    // Metrics
    pub metrics: MetricsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    pub data_dir: PathBuf,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PostgresConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MySqlConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CassandraConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MongoDbConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, anyhow::Error> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}
