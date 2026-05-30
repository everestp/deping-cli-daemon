use std::env;
use std::fs;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub node_id: String,
    pub server_url: String,
    pub max_concurrent_jobs: usize,
}

impl AppConfig {
    pub fn load_from_env() -> Self {
        // 1. Try to load node_id from Identity file as the primary source
        let default_node_id = Self::load_id_from_file().unwrap_or_else(|_| "anonymous_node".to_string());

        let mut node_id = env::var("MINER_NODE_ID").unwrap_or(default_node_id);
        let mut server_url = env::var("SERVER_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let mut max_concurrent_jobs = env::var("MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .unwrap_or(10);

        // 2. CLI Overrides
        for arg in env::args().skip(1) {
            if let Some(stripped) = arg.strip_prefix("-") {
                let parts: Vec<&str> = stripped.splitn(2, '=').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "node_id"    => node_id = parts[1].to_string(),
                        "server_url" => server_url = parts[1].to_string(),
                        "max_jobs"   => {
                            if let Ok(num) = parts[1].parse::<usize>() {
                                max_concurrent_jobs = num;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Self { node_id, server_url, max_concurrent_jobs }
    }

    /// Helper to grab pubkey from local identity store without instantiating full Identity struct
    fn load_id_from_file() -> anyhow::Result<String> {
        let path = dirs::home_dir().unwrap().join(".deping/id.json");
        let data = fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&data)?;
        Ok(json["public_key"].as_str().unwrap_or("").to_string())
    }
}
