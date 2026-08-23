//! AIOps 智能运维路由
//!
//! 四层 AIOps 分析能力：
//!   GET  /api/aiops/recommend          第一层：相似案例推荐（知识库 + 历史工单）
//!   GET  /api/aiops/anomalies           第二层：异常趋势检测（3-sigma + 告警风暴）
//!   GET  /api/aiops/rca/:alertId        第三层：根因分析（CMDB 拓扑图遍历 + 打分）
//!   POST /api/aiops/llm-diagnose        第四层：LLM 大模型诊断（RAG 模式）
//!   GET  /api/aiops/overview            AIOps 首页概览

use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDateTime;
use jieba_rs::Jieba as JiebaRs;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/aiops/overview", get(aiops_overview))
        .route("/api/aiops/recommend", get(recommend))
        .route("/api/aiops/anomalies", get(anomalies))
        .route("/api/aiops/rca/:alertId", get(root_cause_analysis))
        .route("/api/aiops/llm-diagnose", post(llm_diagnose))
}

// ============ 工具函数 ============

static JIEBA: OnceLock<JiebaRs> = OnceLock::new();

fn jieba() -> &'static JiebaRs {
    JIEBA.get_or_init(JiebaRs::new)
}

fn segment_chinese(text: &str) -> String {
    jieba().cut(text, true).join(" ")
}

fn dt_str(r: &sqlx::mysql::MySqlRow, col: &str) -> String {
    r.try_get::<Option<NaiveDateTime>, _>(col)
        .ok()
        .flatten()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn dt_opt(r: &sqlx::mysql::MySqlRow, col: &str) -> Option<String> {
    r.try_get::<Option<NaiveDateTime>, _>(col)
        .ok()
        .flatten()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn severity_to_score(sev: &str) -> f64 {
    let s = sev.trim().to_ascii_lowercase();
    match s.as_str() {
        "5" | "p5" | "disaster" | "dis" => 5.0,
        "4" | "p4" | "high" | "major" | "critical" | "crit" => 4.0,
        "3" | "p3" | "average" | "avg" | "medium" => 3.0,
        "2" | "p2" | "warning" | "warn" => 2.0,
        "1" | "p1" | "information" | "info" | "notice" => 1.0,
        _ => 0.0,
    }
}

/// 记录 AIOps 分析日志
async fn log_analysis(
    pool: &sqlx::MySqlPool,
    atype: &str,
    trigger_id: Option<&str>,
    input: &serde_json::Value,
    result: &serde_json::Value,
    created_by: &str,
) {
    let id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO aiops_analysis_logs (id, type, trigger_id, input_json, result_json, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, NOW())",
    )
    .bind(&id)
    .bind(atype)
    .bind(trigger_id)
    .bind(input.to_string())
    .bind(result.to_string())
    .bind(created_by)
    .execute(pool)
    .await;
}

// ============ AIOps 首页概览 ============

/// GET /api/aiops/overview — AIOps 首页概览
async fn aiops_overview(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 并行执行所有统计查询，大幅缩短响应时间
    let db = &state.db;
    let (
        active_count_res,
        today_new_res,
        sev_rows_res,
        src_rows_res,
        kb_count_res,
        ticket_count_res,
        trend_rows_res,
        llm_enabled_res,
    ) = futures_util::join!(
        // 活跃告警数
        async move {
            sqlx::query("SELECT COUNT(*) AS cnt FROM alert_events WHERE status IN ('firing','acknowledged')")
                .fetch_one(db)
                .await
        },
        // 今日新增
        async {
            sqlx::query(
                "SELECT COUNT(*) AS cnt FROM alert_events WHERE SUBSTRING(fired_at, 1, 10) = DATE_FORMAT(UTC_DATE(), '%Y-%m-%d')",
            )
            .fetch_one(db)
            .await
        },
        // 按 severity 分组
        async {
            sqlx::query(
                "SELECT severity, CAST(COUNT(*) AS SIGNED) AS cnt \
                 FROM alert_events WHERE status IN ('firing','acknowledged') \
                 GROUP BY severity",
            )
            .fetch_all(db)
            .await
        },
        // 按 source 分组
        async {
            sqlx::query(
                "SELECT source, CAST(COUNT(*) AS SIGNED) AS cnt \
                 FROM alert_events WHERE status IN ('firing','acknowledged') \
                 GROUP BY source",
            )
            .fetch_all(db)
            .await
        },
        // 知识库总数
        async {
            sqlx::query("SELECT COUNT(*) AS cnt FROM knowledge_items WHERE status = 'published'")
                .fetch_one(db)
                .await
        },
        // 工单总数
        async {
            sqlx::query("SELECT COUNT(*) AS cnt FROM tickets")
                .fetch_one(db)
                .await
        },
        // 近 24h 趋势（子查询方式兼容 ONLY_FULL_GROUP_BY）
        async {
            sqlx::query(
                "SELECT hr_key, CAST(COUNT(*) AS SIGNED) AS cnt \
                 FROM ( \
                   SELECT SUBSTRING(fired_at, 1, 13) AS hr_key \
                   FROM alert_events \
                   WHERE fired_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 24 HOUR), '%Y-%m-%dT%H:%i:%s') \
                 ) t \
                 GROUP BY hr_key \
                 ORDER BY hr_key",
            )
            .fetch_all(db)
            .await
        },
        // LLM 是否启用
        async { db::get_setting(db, "aiops_llm_enabled").await },
    );

    // 逐个解包结果
    let active_count = active_count_res?.try_get::<i64, _>("cnt").unwrap_or(0);
    let today_new = today_new_res?.try_get::<i64, _>("cnt").unwrap_or(0);

    let mut by_severity = serde_json::Map::new();
    if let Ok(rows) = sev_rows_res {
        for r in &rows {
            let k = r.try_get::<String, _>("severity").unwrap_or_default();
            let v = r.try_get::<i64, _>("cnt").unwrap_or(0);
            by_severity.insert(k, serde_json::Value::from(v));
        }
    }

    let mut by_source = serde_json::Map::new();
    if let Ok(rows) = src_rows_res {
        for r in &rows {
            let k = r.try_get::<String, _>("source").unwrap_or_default();
            let v = r.try_get::<i64, _>("cnt").unwrap_or(0);
            by_source.insert(k, serde_json::Value::from(v));
        }
    }

    let kb_count = kb_count_res?.try_get::<i64, _>("cnt").unwrap_or(0);
    let ticket_count = ticket_count_res?.try_get::<i64, _>("cnt").unwrap_or(0);

    let trend: Vec<serde_json::Value> = if let Ok(rows) = trend_rows_res {
        rows.iter()
            .map(|r| {
                let key = r.try_get::<String, _>("hr_key").unwrap_or_default();
                let hour = key.get(11..13).unwrap_or("00").to_string();
                serde_json::json!({
                    "hour": format!("{}:00", hour),
                    "count": r.try_get::<i64, _>("cnt").unwrap_or(0),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let llm_enabled = llm_enabled_res
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "activeAlerts": active_count,
            "todayNew": today_new,
            "bySeverity": by_severity,
            "bySource": by_source,
            "knowledgeCount": kb_count,
            "ticketCount": ticket_count,
            "trend24h": trend,
            "llmEnabled": llm_enabled,
        }
    })))
}

// ============ 第一层：相似案例推荐 ============

#[derive(Debug, Deserialize)]
struct RecommendQuery {
    #[serde(rename = "alertId")]
    alert_id: Option<String>,
    q: Option<String>,
}

/// GET /api/aiops/recommend — 相似案例推荐
/// 从告警标题/描述提取关键词 → 知识库全文检索 + 历史工单匹配
async fn recommend(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<RecommendQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 提取搜索关键词
    let (keyword, alert_info) = if let Some(aid) = &q.alert_id {
        let row = sqlx::query("SELECT title, message, ci_name_snapshot FROM alert_events WHERE id = ?")
            .bind(aid)
            .fetch_optional(&state.db)
            .await?;
        match row {
            Some(r) => {
                let title = r.try_get::<String, _>("title").unwrap_or_default();
                let message = r.try_get::<Option<String>, _>("message").unwrap_or(None);
                let ci_name = r.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None);
                let kw = if let Some(msg) = &message {
                    format!("{} {}", title, msg)
                } else {
                    title.clone()
                };
                (kw, Some(serde_json::json!({"title": title, "message": message, "ciName": ci_name})))
            }
            None => return Err(AppError::not_found("告警不存在")),
        }
    } else if let Some(kw) = &q.q {
        (kw.clone(), None)
    } else {
        return Err(AppError::bad("请提供 alertId 或 q 参数"));
    };

    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(AppError::bad("关键词不能为空"));
    }

    // jieba 分词
    let segmented = segment_chinese(keyword);
    let like_q = format!("%{}%", segmented.replace(' ', "%"));
    let ft_keyword = format!("\"{}\"", keyword.replace(['"', '+', '-', '*', '(', ')', '~', '<', '>', '\\', '/'], " "));

    // 知识库 FULLTEXT 检索 Top 3
    let kb_rows = sqlx::query(
        "SELECT id, title, category, summary, view_count, helpful_count \
         FROM knowledge_items \
         WHERE status = 'published' \
         AND (MATCH(title) AGAINST(? IN BOOLEAN MODE) \
              OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) \
              OR title LIKE ? OR content_text LIKE ?) \
         ORDER BY updated_at DESC \
         LIMIT 3",
    )
    .bind(&ft_keyword)
    .bind(&ft_keyword)
    .bind(format!("%{}%", keyword))
    .bind(&like_q)
    .fetch_all(&state.db)
    .await?;

    let knowledge: Vec<serde_json::Value> = kb_rows
        .iter()
        .map(|r| {
            let title = r.try_get::<String, _>("title").unwrap_or_default();
            let summary = r.try_get::<Option<String>, _>("summary").unwrap_or(None);
            // 简单匹配度：标题命中 90%，内容命中 70%
            let score = if title.contains(keyword) || keyword.contains(&title) { 90 } else { 70 };
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "title": title,
                "category": r.try_get::<String, _>("category").unwrap_or_default(),
                "summary": summary,
                "score": score,
            })
        })
        .collect();

    // 历史工单匹配 Top 3（标题/描述 LIKE）
    let ticket_rows = sqlx::query(
        "SELECT id, ticket_no, title, status, priority, resolution, created_at, closed_at \
         FROM tickets \
         WHERE title LIKE ? OR description LIKE ? \
         ORDER BY created_at DESC LIMIT 3",
    )
    .bind(format!("%{}%", keyword))
    .bind(format!("%{}%", keyword))
    .fetch_all(&state.db)
    .await?;

    let tickets: Vec<serde_json::Value> = ticket_rows
        .iter()
        .map(|r| {
            let title = r.try_get::<String, _>("title").unwrap_or_default();
            let score = if title.contains(keyword) || keyword.contains(&title) { 85 } else { 65 };
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "ticketNo": r.try_get::<String, _>("ticket_no").unwrap_or_default(),
                "title": title,
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "priority": r.try_get::<String, _>("priority").unwrap_or_default(),
                "resolution": r.try_get::<Option<String>, _>("resolution").unwrap_or(None),
                "createdAt": dt_str(r, "created_at"),
                "closedAt": dt_opt(r, "closed_at"),
                "score": score,
            })
        })
        .collect();

    let result = serde_json::json!({
        "alert": alert_info,
        "keyword": keyword,
        "knowledge": knowledge,
        "tickets": tickets,
    });

    // 记录分析日志
    log_analysis(
        &state.db,
        "recommend",
        q.alert_id.as_deref(),
        &serde_json::json!({"alertId": q.alert_id, "q": q.q}),
        &result,
        &auth.0.sub,
    )
    .await;

    Ok(Json(serde_json::json!({"code": 0, "data": result})))
}

// ============ 第二层：异常趋势检测 ============

/// GET /api/aiops/anomalies — 异常趋势检测
/// 告警量突变(3-sigma) + 重复告警 + MTTR退化 + 告警风暴
async fn anomalies(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let db = &state.db;
    let mut anomalies_list: Vec<serde_json::Value> = Vec::new();

    // 第一阶段：并行执行所有可独立运行的查询
    let (daily_rows_res, repeat_rows_res, mttr_rows_res, recent_active_res, trend_rows_res, total_events_res, resolved_count_res) = futures_util::join!(
        // 1) 近 7 天每天告警数（用于 3-sigma 告警量突变检测）
        async {
            sqlx::query(
                "SELECT d, CAST(COUNT(*) AS SIGNED) AS cnt \
                 FROM ( \
                   SELECT SUBSTRING(fired_at, 1, 10) AS d \
                   FROM alert_events WHERE fired_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 7 DAY), '%Y-%m-%dT%H:%i:%s') \
                 ) t \
                 GROUP BY d ORDER BY d",
            )
            .fetch_all(db)
            .await
        },
        // 2) 近 1 小时重复告警（fire_count > 5）
        async {
            sqlx::query(
                "SELECT fingerprint, title, source, fire_count, severity, fired_at \
                 FROM alert_events \
                 WHERE fired_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 1 HOUR), '%Y-%m-%dT%H:%i:%s') AND fire_count > 5 \
                 ORDER BY fire_count DESC LIMIT 10",
            )
            .fetch_all(db)
            .await
        },
        // 3) 近 7 天 MTTR 统计
        async {
            sqlx::query(
                "SELECT d, \
                 AVG(TIMESTAMPDIFF(MINUTE, \
                    STR_TO_DATE(SUBSTRING(first_fired_at, 1, 19), '%Y-%m-%dT%H:%i:%s'), \
                    STR_TO_DATE(SUBSTRING(resolved_at, 1, 19), '%Y-%m-%dT%H:%i:%s'))) AS avg_min \
                 FROM ( \
                   SELECT SUBSTRING(resolved_at, 1, 10) AS d, first_fired_at, resolved_at \
                   FROM alert_events \
                   WHERE resolved_at IS NOT NULL AND resolved_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 7 DAY), '%Y-%m-%dT%H:%i:%s') \
                 ) t \
                 GROUP BY d ORDER BY d",
            )
            .fetch_all(db)
            .await
        },
        // 4) 近 30 分钟活跃告警数（告警风暴检测）
        async {
            sqlx::query(
                "SELECT COUNT(*) AS cnt FROM alert_events \
                 WHERE status IN ('firing','acknowledged') \
                 AND fired_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 30 MINUTE), '%Y-%m-%dT%H:%i:%s')",
            )
            .fetch_one(db)
            .await
        },
        // 5) 近 24h 趋势数据
        async {
            sqlx::query(
                "SELECT hr_key, \
                 CAST(COUNT(*) AS SIGNED) AS cnt \
                 FROM ( \
                   SELECT SUBSTRING(fired_at, 1, 13) AS hr_key \
                   FROM alert_events WHERE fired_at >= DATE_FORMAT(DATE_SUB(UTC_TIMESTAMP(), INTERVAL 24 HOUR), '%Y-%m-%dT%H:%i:%s') \
                 ) t \
                 GROUP BY hr_key ORDER BY hr_key",
            )
            .fetch_all(db)
            .await
        },
        // 6) 总事件数
        async {
            sqlx::query("SELECT COUNT(*) AS cnt FROM alert_events")
                .fetch_one(db)
                .await
        },
        // 7) 已解决告警数
        async {
            sqlx::query("SELECT COUNT(*) AS cnt FROM alert_events WHERE status = 'resolved'")
                .fetch_one(db)
                .await
        },
    );

    // ---- 处理告警量突变检测 ----
    if let Ok(daily_rows) = daily_rows_res {
        let daily_counts: Vec<f64> = daily_rows
            .iter()
            .map(|r| r.try_get::<i64, _>("cnt").unwrap_or(0) as f64)
            .collect();

        if daily_counts.len() >= 3 {
            let mean = daily_counts.iter().sum::<f64>() / daily_counts.len() as f64;
            let variance = daily_counts.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / daily_counts.len() as f64;
            let std = variance.sqrt();

            // 今日告警数（从 daily_rows 最后一条取，避免再查一次）
            let today_count = daily_counts.last().copied().unwrap_or(0.0) as i64;

            // 3-sigma 检测
            if std > 0.0 && today_count as f64 > mean + 3.0 * std {
                anomalies_list.push(serde_json::json!({
                    "type": "alert_surge",
                    "severity": "high",
                    "title": "告警量异常激增",
                    "message": format!("今日告警 {} 条，过去 7 天日均 {} 条，标准差 {:.1}，超出 3-sigma 阈值", today_count, mean.round() as i64, std),
                    "data": { "today": today_count, "mean": (mean * 10.0).round() / 10.0, "std": (std * 10.0).round() / 10.0 },
                }));
            } else if today_count as f64 > mean * 2.0 && mean > 0.0 {
                anomalies_list.push(serde_json::json!({
                    "type": "alert_surge",
                    "severity": "medium",
                    "title": "告警量明显上升",
                    "message": format!("今日告警 {} 条，是日均 {} 条的 2 倍以上", today_count, mean.round() as i64),
                    "data": { "today": today_count, "mean": (mean * 10.0).round() / 10.0 },
                }));
            }
        }
    }

    // ---- 处理重复告警 ----
    if let Ok(repeat_rows) = repeat_rows_res {
        if !repeat_rows.is_empty() {
            let repeated: Vec<serde_json::Value> = repeat_rows
                .iter()
                .map(|r| serde_json::json!({
                    "fingerprint": r.try_get::<String, _>("fingerprint").unwrap_or_default(),
                    "title": r.try_get::<String, _>("title").unwrap_or_default(),
                    "source": r.try_get::<String, _>("source").unwrap_or_default(),
                    "fireCount": r.try_get::<i64, _>("fire_count").unwrap_or(0),
                    "severity": r.try_get::<String, _>("severity").unwrap_or_default(),
                    "firedAt": r.try_get::<String, _>("fired_at").unwrap_or_default(),
                }))
                .collect();
            anomalies_list.push(serde_json::json!({
                "type": "repeated_alerts",
                "severity": "high",
                "title": format!("近 1 小时发现 {} 条重复告警", repeated.len()),
                "message": "同一告警在短时间内频繁触发，可能是未恢复的持续故障",
                "data": repeated,
            }));
        }
    }

    // ---- 处理 MTTR 退化 ----
    if let Ok(mttr_rows) = mttr_rows_res {
        let mttr_vals: Vec<f64> = mttr_rows
            .iter()
            .filter_map(|r| r.try_get::<Option<f64>, _>("avg_min").ok().flatten())
            .collect();

        if mttr_vals.len() >= 3 {
            let mean_mttr = mttr_vals.iter().sum::<f64>() / mttr_vals.len() as f64;
            let today_mttr = mttr_vals.last().copied().unwrap_or(0.0);
            if mean_mttr > 0.0 && today_mttr > mean_mttr * 3.0 {
                anomalies_list.push(serde_json::json!({
                    "type": "mttr_degradation",
                    "severity": "medium",
                    "title": "MTTR 显著退化",
                    "message": format!("今日平均解决时间 {:.0} 分钟，是近 7 天均值 {:.0} 分钟的 3 倍以上", today_mttr, mean_mttr),
                    "data": { "todayMttr": (today_mttr * 10.0).round() / 10.0, "avgMttr": (mean_mttr * 10.0).round() / 10.0 },
                }));
            }
        }
    }

    // ---- 处理告警风暴 ----
    if let Ok(row) = recent_active_res {
        let recent_active = row.try_get::<i64, _>("cnt").unwrap_or(0);
        if recent_active > 50 {
            anomalies_list.push(serde_json::json!({
                "type": "alert_storm",
                "severity": "critical",
                "title": "告警风暴",
                "message": format!("近 30 分钟内有 {} 条活跃告警，可能发生群体性故障", recent_active),
                "data": { "recentActive": recent_active },
            }));
        }
    }

    // ---- 趋势数据 ----
    let trend: Vec<serde_json::Value> = if let Ok(trend_rows) = trend_rows_res {
        trend_rows.iter()
            .map(|r| {
                let key = r.try_get::<String, _>("hr_key").unwrap_or_default();
                let month_day = key.get(5..10).unwrap_or("--").to_string();
                let hour = key.get(11..13).unwrap_or("00").to_string();
                serde_json::json!({
                    "hour": format!("{} {}:00", month_day, hour),
                    "count": r.try_get::<i64, _>("cnt").unwrap_or(0),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // ---- 统计卡片 ----
    let total_events = total_events_res.map(|r| r.try_get::<i64, _>("cnt").unwrap_or(0)).unwrap_or(0);
    let resolved_count = resolved_count_res.map(|r| r.try_get::<i64, _>("cnt").unwrap_or(0)).unwrap_or(0);

    let result = serde_json::json!({
        "anomalies": anomalies_list,
        "trend": trend,
        "stats": {
            "totalEvents": total_events,
            "resolvedCount": resolved_count,
            "anomalyCount": anomalies_list.len(),
        },
    });

    log_analysis(&state.db, "anomaly", None, &serde_json::json!({}), &result, &auth.0.sub).await;

    Ok(Json(serde_json::json!({"code": 0, "data": result})))
}

// ============ 第三层：根因分析 ============

/// GET /api/aiops/rca/:alertId — 根因分析
/// 取告警 ±30min 内活跃告警 → 关联 CMDB 拓扑 → 图遍历打分 → 推断根因
async fn root_cause_analysis(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(alert_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 1) 取目标告警
    let alert_row = sqlx::query(
        "SELECT id, fingerprint, source, severity, status, title, message, labels, \
         ci_id, ci_name_snapshot, fire_count, first_fired_at, fired_at \
         FROM alert_events WHERE id = ?",
    )
    .bind(&alert_id)
    .fetch_optional(&state.db)
    .await?;

    let alert = match alert_row {
        Some(r) => r,
        None => return Err(AppError::not_found("告警不存在")),
    };

    let target_ci_id = alert.try_get::<Option<String>, _>("ci_id").unwrap_or(None);
    let target_fired_at = alert.try_get::<String, _>("fired_at").unwrap_or_default();
    let target_title = alert.try_get::<String, _>("title").unwrap_or_default();
    let target_severity = alert.try_get::<String, _>("severity").unwrap_or_default();
    let target_ci_name = alert.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None);

    // 2) 取 ±30 分钟内告警（fired_at 是 VARCHAR RFC3339，用字符串比较）
    let (time_min, time_max) = match chrono::DateTime::parse_from_rfc3339(&target_fired_at) {
        Ok(dt) => {
            let min = (dt - chrono::Duration::minutes(30)).to_rfc3339();
            let max = (dt + chrono::Duration::minutes(30)).to_rfc3339();
            (min, max)
        }
        Err(_) => (target_fired_at.clone(), target_fired_at.clone()),
    };
    let related_rows = sqlx::query(
        "SELECT id, source, severity, status, title, message, ci_id, ci_name_snapshot, \
         fire_count, fired_at \
         FROM alert_events \
         WHERE fired_at >= ? AND fired_at <= ? \
         AND id != ? \
         ORDER BY severity DESC, fired_at DESC \
         LIMIT 20",
    )
    .bind(&time_min)
    .bind(&time_max)
    .bind(&alert_id)
    .fetch_all(&state.db)
    .await?;

    let related_alerts: Vec<serde_json::Value> = related_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "source": r.try_get::<String, _>("source").unwrap_or_default(),
                "severity": r.try_get::<String, _>("severity").unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "title": r.try_get::<String, _>("title").unwrap_or_default(),
                "message": r.try_get::<Option<String>, _>("message").unwrap_or(None),
                "ciId": r.try_get::<Option<String>, _>("ci_id").unwrap_or(None),
                "ciName": r.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None),
                "fireCount": r.try_get::<i64, _>("fire_count").unwrap_or(0),
                "firedAt": r.try_get::<String, _>("fired_at").unwrap_or_default(),
            })
        })
        .collect();

    // 3) 构建拓扑关联
    let mut topology_nodes: Vec<serde_json::Value> = Vec::new();
    let mut topology_links: Vec<serde_json::Value> = Vec::new();
    let mut root_cause_ci: Option<(String, String, f64)> = None; // (ci_id, ci_name, confidence)

    if let Some(ci_id) = &target_ci_id {
        // 查目标 CI 信息
        let ci_row = sqlx::query("SELECT id, name, model_id FROM ci_instances WHERE id = ?")
            .bind(ci_id)
            .fetch_optional(&state.db)
            .await?;

        if let Some(ci) = ci_row {
            let ci_name = ci.try_get::<String, _>("name").unwrap_or_default();
            topology_nodes.push(serde_json::json!({
                "id": ci_id, "name": ci_name,
                "type": "target", "level": "root",
            }));
        }

        // 查上下游 CI（1 跳）
        let rel_rows = sqlx::query(
            "SELECT cr.source_id, cr.target_id, cr.relation_type, \
                    cs.name AS source_name, ct.name AS target_name \
             FROM ci_relations cr \
             LEFT JOIN ci_instances cs ON cr.source_id = cs.id \
             LEFT JOIN ci_instances ct ON cr.target_id = ct.id \
             WHERE cr.source_id = ? OR cr.target_id = ?",
        )
        .bind(ci_id)
        .bind(ci_id)
        .fetch_all(&state.db)
        .await?;

        // 收集所有相关 CI
        let mut ci_alerts: std::collections::HashMap<String, (String, i64, f64)> = // (ci_name, alert_count, max_severity)
            std::collections::HashMap::new();

        // 目标 CI 本身有一条告警
        if let Some(name) = target_ci_name.as_ref() {
            ci_alerts.insert(ci_id.clone(), (name.clone(), 1, severity_to_score(&target_severity)));
        }

        for r in &rel_rows {
            let src_id = r.try_get::<String, _>("source_id").unwrap_or_default();
            let tgt_id = r.try_get::<String, _>("target_id").unwrap_or_default();
            let src_name = r.try_get::<String, _>("source_name").unwrap_or_default();
            let tgt_name = r.try_get::<String, _>("target_name").unwrap_or_default();
            let rel_type = r.try_get::<String, _>("relation_type").unwrap_or_default();

            // 添加到拓扑节点
            if src_id != *ci_id {
                topology_nodes.push(serde_json::json!({
                    "id": src_id, "name": src_name, "type": "upstream", "level": "cause",
                }));
                ci_alerts.entry(src_id.clone()).or_insert((src_name.clone(), 0, 0.0));
            }
            if tgt_id != *ci_id {
                topology_nodes.push(serde_json::json!({
                    "id": tgt_id, "name": tgt_name, "type": "downstream", "level": "direct",
                }));
                ci_alerts.entry(tgt_id.clone()).or_insert((tgt_name.clone(), 0, 0.0));
            }

            // 添加拓扑边
            topology_links.push(serde_json::json!({
                "source": src_id, "target": tgt_id, "relationType": rel_type,
            }));
        }

        // 统计各 CI 的告警数
        for ra in &related_alerts {
            if let Some(ra_ci_id) = ra["ciId"].as_str() {
                if let Some(entry) = ci_alerts.get_mut(ra_ci_id) {
                    entry.1 += 1;
                    let sev = ra["severity"].as_str().unwrap_or("0");
                    let score = severity_to_score(sev);
                    if score > entry.2 {
                        entry.2 = score;
                    }
                }
            }
        }

        // 打分：score = 0.4 * downstream_alerts + 0.3 * max_severity + 0.2 * (1/depth) + 0.1 * time_proximity
        // 简化版：上游 CI 的告警数越多 + 级别越高 → 越可能是根因
        let mut best_score = 0.0f64;
        let mut best_ci: Option<(String, String)> = None;

        for (ci_id_v, (ci_name_v, alert_count, max_sev)) in &ci_alerts {
            // 判断是上游还是目标节点
            let is_upstream = topology_nodes.iter().any(|n| {
                n["id"].as_str() == Some(ci_id_v.as_str()) && n["type"].as_str() == Some("upstream")
            });
            let depth = if is_upstream { 1.0 } else { 0.5 }; // 上游深度=1，目标深度=0.5

            let score = 0.4 * (*alert_count as f64)
                + 0.3 * (*max_sev / 5.0)
                + 0.2 * (1.0 / depth)
                + 0.1; // time_proximity 简化为常数

            if score > best_score && *alert_count > 0 {
                best_score = score;
                best_ci = Some((ci_id_v.clone(), ci_name_v.clone()));
            }
        }

        // 如果有最佳候选，设为根因
        if let Some((id, name)) = best_ci {
            let confidence = (best_score / 2.0 * 100.0).min(99.0).round() as u8;
            root_cause_ci = Some((id, name, confidence as f64));
        }
    }

    // 4) 知识库匹配：用根因 CI 名称或告警标题检索
    let search_keyword = root_cause_ci
        .as_ref()
        .map(|(_, name, _)| name.clone())
        .unwrap_or_else(|| target_title.clone());

    let segmented = segment_chinese(&search_keyword);
    let like_q = format!("%{}%", segmented.replace(' ', "%"));
    let ft_keyword = format!("\"{}\"", search_keyword.replace(['"', '+', '-', '*', '(', ')', '~', '<', '>', '\\', '/'], " "));

    let kb_rows = sqlx::query(
        "SELECT id, title, category, summary \
         FROM knowledge_items \
         WHERE status = 'published' \
         AND (MATCH(title) AGAINST(? IN BOOLEAN MODE) \
              OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) \
              OR title LIKE ? OR content_text LIKE ?) \
         ORDER BY updated_at DESC LIMIT 3",
    )
    .bind(&ft_keyword)
    .bind(&ft_keyword)
    .bind(format!("%{}%", &search_keyword))
    .bind(&like_q)
    .fetch_all(&state.db)
    .await?;

    let knowledge: Vec<serde_json::Value> = kb_rows
        .iter()
        .map(|r| serde_json::json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "title": r.try_get::<String, _>("title").unwrap_or_default(),
            "category": r.try_get::<String, _>("category").unwrap_or_default(),
            "summary": r.try_get::<Option<String>, _>("summary").unwrap_or(None),
        }))
        .collect();

    // 5) 构建诊断树
    let diagnosis_tree = if let Some((ci_id, ci_name, confidence)) = &root_cause_ci {
        let mut children: Vec<serde_json::Value> = Vec::new();
        // 相关告警作为子节点
        for ra in &related_alerts {
            if ra["ciId"].as_str() == Some(ci_id.as_str()) {
                children.push(serde_json::json!({
                    "id": ra["id"],
                    "label": format!("{} ({})", ra["title"], ra["severity"]),
                    "level": "direct",
                    "confidence": confidence * 0.8,
                }));
            }
        }
        // 如果没有子节点，用目标告警
        if children.is_empty() {
            children.push(serde_json::json!({
                "id": &alert_id,
                "label": format!("{} ({})", target_title, target_severity),
                "level": "direct",
                "confidence": confidence * 0.7,
            }));
        }

        vec![serde_json::json!({
            "id": "root",
            "label": format!("{} ({})", ci_name, target_severity),
            "level": "root",
            "confidence": confidence,
            "children": children,
        })]
    } else {
        // 没有拓扑数据，用目标告警作为根
        vec![serde_json::json!({
            "id": "root",
            "label": format!("{} ({})", target_title, target_severity),
            "level": "root",
            "confidence": 50,
            "children": related_alerts.iter().take(5).map(|ra| serde_json::json!({
                "id": ra["id"],
                "label": format!("{} ({})", ra["title"], ra["severity"]),
                "level": "cause",
                "confidence": 60,
            })).collect::<Vec<_>>(),
        })]
    };

    // 6) 处置建议
    let mut suggestions: Vec<serde_json::Value> = Vec::new();
    if let Some((_, ci_name, confidence)) = &root_cause_ci {
        suggestions.push(serde_json::json!({
            "time": "立即", "type": "danger",
            "title": format!("检查 {} 状态", ci_name),
            "desc": format!("拓扑分析推断根因为 {}，置信度 {}%，建议优先检查该节点状态", ci_name, confidence.round() as u8),
        }));
    }
    suggestions.push(serde_json::json!({
        "time": "5 分钟", "type": "warning",
        "title": "查看相关告警",
        "desc": format!("时间窗口内有 {} 条相关告警，关注是否同一故障引发", related_alerts.len()),
    }));
    if !knowledge.is_empty() {
        suggestions.push(serde_json::json!({
            "time": "10 分钟", "type": "primary",
            "title": "参考知识库案例",
            "desc": format!("找到 {} 条相似知识条目，查看历史解决方案", knowledge.len()),
        }));
    }

    let result = serde_json::json!({
        "rootAlert": {
            "id": alert_id,
            "title": target_title,
            "severity": target_severity,
            "source": alert.try_get::<String, _>("source").unwrap_or_default(),
            "ciId": target_ci_id,
            "ciName": target_ci_name,
            "firedAt": target_fired_at,
        },
        "relatedAlerts": related_alerts,
        "topology": {
            "nodes": topology_nodes,
            "links": topology_links,
        },
        "rootCause": root_cause_ci.as_ref().map(|(id, name, conf)| serde_json::json!({
            "ciId": id, "ciName": name, "confidence": conf,
        })),
        "diagnosisTree": diagnosis_tree,
        "knowledge": knowledge,
        "suggestions": suggestions,
    });

    log_analysis(&state.db, "rca", Some(&alert_id), &serde_json::json!({"alertId": alert_id}), &result, &auth.0.sub).await;

    Ok(Json(serde_json::json!({"code": 0, "data": result})))
}

// ============ 第四层：LLM 大模型诊断 ============

#[derive(Debug, Deserialize)]
struct LlmDiagnoseRequest {
    #[serde(rename = "alertId")]
    alert_id: Option<String>,
    question: Option<String>,
}

/// POST /api/aiops/llm-diagnose — LLM 大模型诊断（RAG 模式）
async fn llm_diagnose(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LlmDiagnoseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 读取 LLM 配置
    let llm_enabled = db::get_setting(&state.db, "aiops_llm_enabled")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    if !llm_enabled {
        return Err(AppError::bad("AIOps LLM 诊断未启用，请在系统设置中开启"));
    }

    let api_url = db::get_setting(&state.db, "aiops_llm_api_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let api_key = db::get_setting(&state.db, "aiops_llm_api_key")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let model = db::get_setting(&state.db, "aiops_llm_model")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "deepseek-chat".to_string());

    if api_url.is_empty() {
        return Err(AppError::bad("LLM API 地址未配置"));
    }

    // 收集上下文
    let mut context_parts: Vec<String> = Vec::new();
    let mut context_json = serde_json::json!({});

    if let Some(aid) = &req.alert_id {
        // 取告警详情
        let row = sqlx::query(
            "SELECT title, severity, status, message, source, ci_name_snapshot, fired_at \
             FROM alert_events WHERE id = ?",
        )
        .bind(aid)
        .fetch_optional(&state.db)
        .await?;

        if let Some(r) = row {
            let title = r.try_get::<String, _>("title").unwrap_or_default();
            let severity = r.try_get::<String, _>("severity").unwrap_or_default();
            let message = r.try_get::<Option<String>, _>("message").unwrap_or(None);
            let source = r.try_get::<String, _>("source").unwrap_or_default();
            let ci_name = r.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None);
            let fired_at = r.try_get::<String, _>("fired_at").unwrap_or_default();

            context_parts.push(format!(
                "## 告警信息\n- 标题: {}\n- 级别: {}\n- 来源: {}\n- 关联资产: {}\n- 触发时间: {}\n- 描述: {}",
                title, severity, source, ci_name.clone().unwrap_or_default(), fired_at, message.clone().unwrap_or_default()
            ));
            context_json["alert"] = serde_json::json!({
                "title": title, "severity": severity, "source": source,
                "ciName": ci_name, "firedAt": fired_at, "message": message,
            });

            // 知识库匹配
            let segmented = segment_chinese(&title);
            let like_q = format!("%{}%", segmented.replace(' ', "%"));
            let ft_title = format!("\"{}\"", title.replace(['"', '+', '-', '*', '(', ')', '~', '<', '>', '\\', '/'], " "));
            let kb_rows = sqlx::query(
                "SELECT title, summary FROM knowledge_items \
                 WHERE status = 'published' \
                 AND (MATCH(title) AGAINST(? IN BOOLEAN MODE) \
                      OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) \
                      OR title LIKE ? OR content_text LIKE ?) \
                 ORDER BY updated_at DESC LIMIT 3",
            )
            .bind(&ft_title)
            .bind(&ft_title)
            .bind(format!("%{}%", &title))
            .bind(&like_q)
            .fetch_all(&state.db)
            .await?;

            if !kb_rows.is_empty() {
                let kb_text: Vec<String> = kb_rows
                    .iter()
                    .map(|r| {
                        let t = r.try_get::<String, _>("title").unwrap_or_default();
                        let s = r.try_get::<Option<String>, _>("summary").unwrap_or(None);
                        format!("- {}: {}", t, s.unwrap_or_default())
                    })
                    .collect();
                context_parts.push(format!("## 知识库匹配\n{}", kb_text.join("\n")));
                context_json["knowledge"] = serde_json::json!(kb_text);
            }
        }
    }

    // 用户问题
    let user_question = req.question.clone().unwrap_or_else(|| {
        if req.alert_id.is_some() {
            "请分析此告警的可能根因，并给出处置建议。".to_string()
        } else {
            "请分析当前系统的运维状况。".to_string()
        }
    });

    context_parts.push(format!("## 用户问题\n{}", user_question));

    // 组装 Prompt
    let system_prompt = "你是 MeridianOps 智能运维助手，专注于 IT 运维场景的故障诊断和根因分析。\
请基于提供的运维数据（告警信息、CMDB 拓扑、知识库匹配）进行分析，给出：\n\
1. 可能的根因分析\n\
2. 处置建议和操作步骤\n\
3. 风险评估\n\n\
要求：回答简洁专业，如果数据不足以判断，明确说明需要补充什么信息。";

    let user_prompt = context_parts.join("\n\n");

    // 调用 LLM API
    let llm_request = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "temperature": 0.3,
        "max_tokens": 2000,
    });

    let client = &state.client;
    let full_url = if api_url.ends_with("/chat/completions") {
        api_url.clone()
    } else if api_url.ends_with('/') {
        format!("{}chat/completions", api_url)
    } else {
        format!("{}/chat/completions", api_url)
    };

    let llm_response = client
        .post(&full_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&llm_request)
        .send()
        .await
        .map_err(|e| AppError::internal(&format!("LLM API 调用失败: {}", e)))?;

    if !llm_response.status().is_success() {
        let status = llm_response.status();
        let body = llm_response.text().await.unwrap_or_default();
        return Err(AppError::internal(&format!("LLM API 返回错误 ({}): {}", status, body)));
    }

    let llm_data: serde_json::Value = llm_response
        .json()
        .await
        .map_err(|e| AppError::internal(&format!("LLM 响应解析失败: {}", e)))?;

    let answer = llm_data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("LLM 未返回有效内容")
        .to_string();

    let result = serde_json::json!({
        "answer": answer,
        "model": model,
        "context": context_json,
    });

    // 记录审计
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "aiops_llm_diagnose", "aiops",
        req.alert_id.as_deref().unwrap_or(""),
        Some(&result), &ip, "success",
    ).await;

    log_analysis(
        &state.db,
        "llm",
        req.alert_id.as_deref(),
        &serde_json::json!({"alertId": req.alert_id, "question": req.question}),
        &result,
        &auth.0.sub,
    )
    .await;

    Ok(Json(serde_json::json!({"code": 0, "data": result})))
}
