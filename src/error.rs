use thiserror::Error;

#[derive(Error, Debug, Clone)] 
pub enum MinerError {
    #[error("Stream dead: connection to backend lost")]
    StreamDead,

    #[error("Connection timeout executing network job")]
    ConnectionTimeout,

    #[error("Execution dropped due to runtime constraint")]
    ExecutionDropped,

    #[error("Invalid host configuration or target URL: {0}")]
    InvalidHost(String),

    #[error("Target DDOS protection triggered (HTTP 429/403)")]
    TargetDdosProtectionTriggered,

    #[error("Internal system channel error: {0}")]
    ChannelError(String),
}
