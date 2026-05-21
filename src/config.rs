use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub node_id: String,
    pub server_url: String,
    pub max_concurrent_jobs: usize,
}

impl AppConfig {
    /// Parses parameters from command line flags first, falling back to env vars, then defaults.
    pub fn load_from_env() -> Self {
        // 1. Establish structural fallbacks from environment values first
        let mut node_id = env::var("MINER_NODE_ID").unwrap_or_else(|_| "runner_pubkey_us_001".to_string());
        let mut server_url = env::var("SERVER_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let mut max_concurrent_jobs = env::var("MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .unwrap_or(10);

        // 2. Safely capture the single-dash parameters passed via CLI
        for arg in env::args().skip(1) {
            if let Some(stripped) = arg.strip_prefix("-") {
                let parts: Vec<&str> = stripped.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0];
                    let val = parts[1].to_string();

                    match key {
                        "node_id"    => node_id = val,
                        "server_url" => server_url = val,
                        "max_jobs"   => {
                            if let Ok(num) = val.parse::<usize>() {
                                max_concurrent_jobs = num;
                            }
                        }
                        _ => {} // Discard unknown arguments smoothly (including legacy lat/lng flags)
                    }
                }
            }
        }

        Self {
            node_id,
            server_url,
            max_concurrent_jobs,
        }
    }
}
