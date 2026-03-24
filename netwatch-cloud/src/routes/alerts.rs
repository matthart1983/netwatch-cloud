use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::AppState;

// --- Alert Rules ---

#[derive(Serialize)]
pub struct AlertRule {
    pub id: Uuid,
    pub host_id: Option<Uuid>,
    pub name: String,
    pub metric: String,
    pub condition: String,
    pub threshold: Option<f64>,
    pub threshold_str: Option<String>,
    pub duration_secs: i32,
    pub severity: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn list_rules(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AlertRule>>, StatusCode> {
    let rows = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, String, String, Option<f64>, Option<String>, i32, String, bool, DateTime<Utc>)>(
        "SELECT id, host_id, name, metric, condition, threshold, threshold_str, duration_secs, severity, enabled, created_at FROM alert_rules WHERE account_id = $1 ORDER BY created_at",
    )
    .bind(user.account_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rules: Vec<AlertRule> = rows
        .into_iter()
        .map(
            |(id, host_id, name, metric, condition, threshold, threshold_str, duration_secs, severity, enabled, created_at)| {
                AlertRule {
                    id, host_id, name, metric, condition, threshold, threshold_str,
                    duration_secs, severity, enabled, created_at,
                }
            },
        )
        .collect();

    Ok(Json(rules))
}

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    pub host_id: Option<Uuid>,
    pub name: String,
    pub metric: String,
    pub condition: String,
    pub threshold: Option<f64>,
    pub threshold_str: Option<String>,
    pub duration_secs: Option<i32>,
    pub severity: Option<String>,
}

pub async fn create_rule(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<AlertRule>, StatusCode> {
    let id = Uuid::new_v4();
    let duration = req.duration_secs.unwrap_or(60);
    let severity = req.severity.unwrap_or_else(|| "warning".to_string());
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO alert_rules (id, account_id, host_id, name, metric, condition, threshold, threshold_str, duration_secs, severity) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(user.account_id)
    .bind(req.host_id)
    .bind(&req.name)
    .bind(&req.metric)
    .bind(&req.condition)
    .bind(req.threshold)
    .bind(&req.threshold_str)
    .bind(duration)
    .bind(&severity)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AlertRule {
        id,
        host_id: req.host_id,
        name: req.name,
        metric: req.metric,
        condition: req.condition,
        threshold: req.threshold,
        threshold_str: req.threshold_str,
        duration_secs: duration,
        severity,
        enabled: true,
        created_at: now,
    }))
}

#[derive(Deserialize)]
pub struct UpdateRuleRequest {
    pub enabled: Option<bool>,
    pub threshold: Option<f64>,
    pub duration_secs: Option<i32>,
    pub severity: Option<String>,
}

pub async fn update_rule(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRuleRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut query = String::from("UPDATE alert_rules SET ");
    let mut parts: Vec<String> = Vec::new();
    let mut param_idx = 3u32; // $1=id, $2=account_id

    if req.enabled.is_some() {
        parts.push(format!("enabled = ${}", param_idx));
        param_idx += 1;
    }
    if req.threshold.is_some() {
        parts.push(format!("threshold = ${}", param_idx));
        param_idx += 1;
    }
    if req.duration_secs.is_some() {
        parts.push(format!("duration_secs = ${}", param_idx));
        param_idx += 1;
    }
    if req.severity.is_some() {
        parts.push(format!("severity = ${}", param_idx));
    }

    if parts.is_empty() {
        return Ok(StatusCode::OK);
    }

    query.push_str(&parts.join(", "));
    query.push_str(" WHERE id = $1 AND account_id = $2");

    let mut q = sqlx::query(&query).bind(id).bind(user.account_id);
    if let Some(v) = req.enabled {
        q = q.bind(v);
    }
    if let Some(v) = req.threshold {
        q = q.bind(v);
    }
    if let Some(v) = req.duration_secs {
        q = q.bind(v);
    }
    if let Some(v) = &req.severity {
        q = q.bind(v);
    }

    let result = q
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::OK)
}

pub async fn delete_rule(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM alert_rules WHERE id = $1 AND account_id = $2")
        .bind(id)
        .bind(user.account_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

// --- Alert History ---

#[derive(Serialize)]
pub struct AlertEvent {
    pub id: i64,
    pub rule_id: Uuid,
    pub host_id: Uuid,
    pub state: String,
    pub metric_value: Option<f64>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub host_id: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn alert_history(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<AlertEvent>>, StatusCode> {
    let limit = query.limit.unwrap_or(50).min(200);

    let rows = if let Some(host_id) = query.host_id {
        sqlx::query_as::<_, (i64, Uuid, Uuid, String, Option<f64>, String, DateTime<Utc>)>(
            "SELECT ae.id, ae.rule_id, ae.host_id, ae.state, ae.metric_value, ae.message, ae.created_at FROM alert_events ae JOIN alert_rules ar ON ae.rule_id = ar.id WHERE ar.account_id = $1 AND ae.host_id = $2 ORDER BY ae.created_at DESC LIMIT $3",
        )
        .bind(user.account_id)
        .bind(host_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, (i64, Uuid, Uuid, String, Option<f64>, String, DateTime<Utc>)>(
            "SELECT ae.id, ae.rule_id, ae.host_id, ae.state, ae.metric_value, ae.message, ae.created_at FROM alert_events ae JOIN alert_rules ar ON ae.rule_id = ar.id WHERE ar.account_id = $1 ORDER BY ae.created_at DESC LIMIT $2",
        )
        .bind(user.account_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let events: Vec<AlertEvent> = rows
        .into_iter()
        .map(
            |(id, rule_id, host_id, alert_state, metric_value, message, created_at)| AlertEvent {
                id,
                rule_id,
                host_id,
                state: alert_state,
                metric_value,
                message,
                created_at,
            },
        )
        .collect();

    Ok(Json(events))
}
