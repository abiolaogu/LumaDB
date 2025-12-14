
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::interval;
use reqwest::Client;

/// Scraper configuration
#[derive(Clone, Debug)]
pub struct ScraperConfig {
    pub global_interval: Duration,
    pub global_timeout: Duration,
    pub jobs: Vec<ScrapeJob>,
}

#[derive(Clone, Debug)]
pub struct ScrapeJob {
    pub name: String,
    pub interval: Option<Duration>,
    pub timeout: Option<Duration>,
    pub metrics_path: String,
    pub targets: Vec<String>, // Simplified for now (Static only)
}

use crate::storage::metric_store::{MetricsStorage, Metric};

pub struct PrometheusScraper {
    config: ScraperConfig,
    targets: Arc<RwLock<HashMap<String, Target>>>,
    client: Client,
    storage: Arc<MetricsStorage>,
}

pub struct Target {
    pub url: String,
    pub labels: HashMap<String, String>,
    pub last_scrape: Option<Instant>,
    pub last_scrape_duration: Option<Duration>,
    pub error_count: u64,
}

impl PrometheusScraper {
    pub fn new(config: ScraperConfig, storage: Arc<MetricsStorage>) -> Self {
        Self {
            config,
            targets: Arc::new(RwLock::new(HashMap::new())),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            storage,
        }
    }

    pub async fn start(&self) {
        // Simplified start: just Loop over static targets
        for job in &self.config.jobs {
            for target_url in &job.targets {
                let scraper = self.clone(); // Needs clone trait or Arc wrapper, will fix structure
                let job_name = job.name.clone();
                let url = target_url.clone();
                let path = job.metrics_path.clone();
                let interval_dur = job.interval.unwrap_or(self.config.global_interval);
                
                tokio::spawn(async move {
                    let mut ticker = interval(interval_dur);
                    loop {
                        ticker.tick().await;
                        // Scrape
                         match reqwest::get(format!("{}{}", url, path)).await {
                             Ok(resp) => {
                                 let text = resp.text().await.unwrap_or_default();
                                 println!("Scraped {} bytes from {}:{}", text.len(), job_name, url);
                                 // TODO: Parse and Store
                             }
                             Err(e) => eprintln!("Scrape failed for {}: {}", url, e),
                         }
                    }
                });
            }
        }
    }
}

fn parse_prometheus_metrics(text: &str) -> Result<Vec<Metric>, String> {
    // Placeholder for actual parsing logic
    // This function would parse the Prometheus text format into a vector of Metric structs.
    // For now, it returns an empty vector.
    Ok(vec![])
}

// Clone implementation to allow spawning (Arc wrapper pattern is better but keeping simple for now)
impl Clone for PrometheusScraper {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            targets: self.targets.clone(),
            client: self.client.clone(),
            storage: self.storage.clone(),
        }
    }
}
