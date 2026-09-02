use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,
    #[serde(default)]
    pub notification_cleaner: NotificationCleanerConfig,
    #[serde(default)]
    pub logs: LogsConfig,
    pub systems: Vec<SystemConfig>,
}

/// 日志平台对接配置（ClickHouse + Loki 组合方案，详见 README 日志平台集成方案）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogsConfig {
    /// ClickHouse HTTP 接口地址（如 http://10.0.5.21:8123）
    pub clickhouse_url: String,
    /// ClickHouse 数据库名（默认 meridianops_logs）
    pub clickhouse_database: String,
    /// Loki HTTP 接口地址（用于生成 Grafana Explore 跳转 URL，不直接查询）
    pub loki_url: String,
    /// 单次查询最大返回行数（防止前端拉爆）
    pub max_query_rows: u32,
    /// 查询超时（秒）
    pub query_timeout_secs: u64,
    /// 日志告警联动配置（Phase 5）
    #[serde(default)]
    pub alerting: LogsAlertingConfig,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            clickhouse_url: "http://127.0.0.1:8123".to_string(),
            clickhouse_database: "meridianops_logs".to_string(),
            loki_url: "http://127.0.0.1:3100".to_string(),
            max_query_rows: 1000,
            query_timeout_secs: 30,
            alerting: LogsAlertingConfig::default(),
        }
    }
}

/// 日志告警联动配置：监控 ClickHouse 中 ERROR+ 级别日志突增，触发告警回流 MeridianOps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogsAlertingConfig {
    /// 总开关（默认关，需显式开启）
    pub enabled: bool,
    /// 检查间隔（秒），默认 60
    pub check_interval_secs: u64,
    /// 统计窗口（分钟），默认 5
    pub window_minutes: u32,
    /// 触发的级别（逗号分隔），默认 error,critical,fatal
    pub levels: String,
    /// 窗口内日志数超过此阈值触发告警，默认 50
    pub error_threshold: u32,
    /// 同主机告警后静默时间（分钟），避免重复触发，默认 30
    pub silence_minutes: u32,
}

impl Default for LogsAlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_secs: 60,
            window_minutes: 5,
            levels: "error,critical,fatal".to_string(),
            error_threshold: 50,
            silence_minutes: 30,
        }
    }
}

/// 通知发送日志自动清理
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationCleanerConfig {
    /// 是否启用自动清理（默认开）
    pub enabled: bool,
    /// 保留天数（默认 30 天）。小于等于 0 表示按 30 天保底
    pub retention_days: u32,
    /// 执行间隔（秒），默认 86400 = 每 24 小时一次
    pub interval_secs: u64,
    /// 每批删除上限，避免长事务锁表；默认 1000
    pub batch_size: u32,
}

impl Default for NotificationCleanerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 30,
            interval_secs: 86400,
            batch_size: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertsConfig {
    /// Eventide webhook 推送鉴权 token，双方共享。
    /// MeridianOps 端校验 `Authorization: Bearer <ingress_token>`。
    pub ingress_token: String,
    /// 是否启用 ingress 接收端（false 时返回 404，避免暴露未配置的端点）
    pub ingress_enabled: bool,
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            ingress_token: "change-me-to-a-random-secret".to_string(),
            ingress_enabled: false,
        }
    }
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
    "mysql://root:886363@120.26.67.180:3306/meridianops".to_string()
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
            alerts: AlertsConfig::default(),
            notification_cleaner: NotificationCleanerConfig::default(),
            logs: LogsConfig::default(),
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
        if let Ok(v) = std::env::var("MERIDIANOPS_ALERT_INGRESS_TOKEN") {
            self.alerts.ingress_token = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_ALERT_INGRESS_ENABLED") {
            if let Ok(b) = v.parse::<bool>() {
                self.alerts.ingress_enabled = b;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_NOTIF_CLEAN_ENABLED") {
            if let Ok(b) = v.parse::<bool>() {
                self.notification_cleaner.enabled = b;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_NOTIF_CLEAN_RETENTION_DAYS") {
            if let Ok(n) = v.parse::<u32>() {
                self.notification_cleaner.retention_days = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_NOTIF_CLEAN_INTERVAL_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                self.notification_cleaner.interval_secs = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_NOTIF_CLEAN_BATCH_SIZE") {
            if let Ok(n) = v.parse::<u32>() {
                self.notification_cleaner.batch_size = n;
            }
        }
        // 日志平台覆盖
        if let Ok(v) = std::env::var("MERIDIANOPS_LOGS_CLICKHOUSE_URL") {
            self.logs.clickhouse_url = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOGS_CLICKHOUSE_DB") {
            self.logs.clickhouse_database = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOGS_LOKI_URL") {
            self.logs.loki_url = v;
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOGS_MAX_ROWS") {
            if let Ok(n) = v.parse::<u32>() {
                self.logs.max_query_rows = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOGS_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                self.logs.query_timeout_secs = n;
            }
        }
        // 日志告警联动覆盖
        if let Ok(v) = std::env::var("MERIDIANOPS_LOG_ALERT_ENABLED") {
            self.logs.alerting.enabled = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOG_ALERT_INTERVAL") {
            if let Ok(n) = v.parse::<u64>() {
                self.logs.alerting.check_interval_secs = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOG_ALERT_THRESHOLD") {
            if let Ok(n) = v.parse::<u32>() {
                self.logs.alerting.error_threshold = n;
            }
        }
        if let Ok(v) = std::env::var("MERIDIANOPS_LOG_ALERT_SILENCE") {
            if let Ok(n) = v.parse::<u32>() {
                self.logs.alerting.silence_minutes = n;
            }
        }
    }
}
