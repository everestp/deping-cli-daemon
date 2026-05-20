use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub node_id: String,
    pub region: String,
    pub server_url: String,
    pub max_concurrent_jobs: usize,
}

impl AppConfig {
    pub fn load_from_env() -> Self {
        Self {
            node_id: env::var("MINER_NODE_ID").unwrap_or_else(|_| "runner_pubkey_us_001".to_string()),
            region: env::var("MINER_REGION").unwrap_or_else(|_| "us-east".to_string()),
            server_url: env::var("SERVER_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string()),
            max_concurrent_jobs: env::var("MAX_CONCURRENT_JOBS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        }
    }
}
