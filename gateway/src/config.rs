use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    pub systems: Vec<SystemConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default = "default_mysql_url")]
    pub url: String,
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    #[serde(default = "default_min_conn")]
    pub min_connections: u32,
}

fn default_mysql_url() -> String {
    // 与 Eventide 共用同一 MySQL 实例，独立库名 meridianops
    "mysql://root:886363@120.26.105.115:3306/meridianops".to_string()
}
fn default_max_conn() -> u32 {
    10
}
fn default_min_conn() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// HMAC secret for JWT signing. 生产环境必须通过 MERIDIANOPS_JWT_SECRET 覆盖。
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    /// Token 有效期（小时）。
    #[serde(default = "default_jwt_ttl")]
    pub token_ttl_hours: u64,
    /// 首次启动 users 表为空时创建的 admin 用户名。
    #[serde(default = "default_seed_username")]
    pub seed_username: String,
    /// 首次启动 admin 密码（明文，仅启动期用一次）。
    #[serde(default = "default_seed_password")]
    pub seed_password: String,
    /// 是否启用 JWT 鉴权（开发期可关，默认 true）。
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            token_ttl_hours: default_jwt_ttl(),
            seed_username: default_seed_username(),
            seed_password: default_seed_password(),
            enabled: default_true(),
        }
    }
}

fn default_jwt_secret() -> String {
    "meridianops-dev-secret-change-me".to_string()
}
fn default_jwt_ttl() -> u64 {
    24
}
fn default_seed_username() -> String {
    "admin".to_string()
}
fn default_seed_password() -> String {
    "Admin123!".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub id: String,
    pub name: String,
    pub system_type: String,
    pub base_url: String,
    pub auth_type: String,
    #[serde(default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub auth_username: Option<String>,
    #[serde(default)]
    pub auth_password: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub iframe_url: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: "0.0.0.0:8000".to_string(),
                cors_origins: vec!["*".to_string()],
            },
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
            systems: vec![
                SystemConfig {
                    id: "axleops".to_string(),
                    name: "AxleOps 服务管理".to_string(),
                    system_type: "service-mgmt".to_string(),
                    base_url: "http://axleops-admin:9000".to_string(),
                    auth_type: "session".to_string(),
                    auth_token: None,
                    auth_username: None,
                    auth_password: None,
                    status: "online".to_string(),
                    version: Some("v0.4.1".to_string()),
                    iframe_url: None,
                },
                SystemConfig {
                    id: "eventide".to_string(),
                    name: "Eventide 告警中心".to_string(),
                    system_type: "alert-center".to_string(),
                    base_url: "http://eventide:8080".to_string(),
                    auth_type: "token".to_string(),
                    auth_token: Some("eventide-admin-token".to_string()),
                    auth_username: None,
                    auth_password: None,
                    status: "online".to_string(),
                    version: Some("v0.3.0".to_string()),
                    iframe_url: None,
                },
                SystemConfig {
                    id: "zabbix".to_string(),
                    name: "Zabbix 监控".to_string(),
                    system_type: "monitoring".to_string(),
                    base_url: "http://zabbix:10051".to_string(),
                    auth_type: "api_key".to_string(),
                    auth_token: Some("zabbix-api-key".to_string()),
                    auth_username: None,
                    auth_password: None,
                    status: "online".to_string(),
                    version: Some("v7.0".to_string()),
                    iframe_url: None,
                },
                SystemConfig {
                    id: "elk".to_string(),
                    name: "ELK 日志".to_string(),
                    system_type: "logging".to_string(),
                    base_url: "http://elk:5601".to_string(),
                    auth_type: "basic_auth".to_string(),
                    auth_token: None,
                    auth_username: Some("elastic".to_string()),
                    auth_password: Some("changeme".to_string()),
                    status: "online".to_string(),
                    version: Some("v8.12".to_string()),
                    iframe_url: Some("http://elk:5601/app/kibana".to_string()),
                },
                SystemConfig {
                    id: "prometheus".to_string(),
                    name: "Prometheus 指标".to_string(),
                    system_type: "metrics".to_string(),
                    base_url: "http://prometheus:9090".to_string(),
                    auth_type: "none".to_string(),
                    auth_token: None,
                    auth_username: None,
                    auth_password: None,
                    status: "online".to_string(),
                    version: Some("v2.50".to_string()),
                    iframe_url: None,
                },
            ],
        }
    }
}

impl GatewayConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: GatewayConfig = toml::from_str(&content)?;
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn system_by_id(&self, id: &str) -> Option<&SystemConfig> {
        self.systems.iter().find(|s| s.id == id)
    }

    /// 环境变量覆盖 toml 配置（参考 AxleOps 做法），便于容器化部署。
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("MERIDIANOPS_SERVER_BIND") {
            self.server.bind = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_DB_URL") {
            self.database.url = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_JWT_SECRET") {
            self.auth.jwt_secret = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_JWT_TTL_HOURS") {
            if let Ok(n) = v.parse::<u64>() {
                self.auth.token_ttl_hours = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_SEED_USERNAME") {
            self.auth.seed_username = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_SEED_PASSWORD") {
            self.auth.seed_password = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_AUTH_ENABLED") {
            if let Ok(b) = v.parse::<bool>() {
                self.auth.enabled = b;
            }
        }
    }
}
