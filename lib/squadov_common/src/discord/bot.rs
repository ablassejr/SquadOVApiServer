use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct DiscordBotRoleConfig {
    pub silver: u64,
    pub gold: u64,
    pub diamond: u64,
    pub early_access: u64,
}

#[derive(Clone, Deserialize)]
pub struct DiscordBotConfig {
    pub token: String,
    pub server_id: u64,
    pub roles: DiscordBotRoleConfig,
}