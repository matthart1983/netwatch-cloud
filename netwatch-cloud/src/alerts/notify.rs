use crate::config::ServerConfig;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use tracing::{info, warn};
use uuid::Uuid;

// Global throttle state: (rule_id, host_id) -> last notification time
lazy_static::lazy_static! {
    static ref NOTIFICATION_THROTTLE: Mutex<HashMap<(Uuid, Uuid), Instant>> = Mutex::new(HashMap::new());
}

const THROTTLE_DURATION: Duration = Duration::from_secs(15 * 60); // 15 minutes

pub async fn send_alert(
    db: &sqlx::PgPool,
    config: &ServerConfig,
    account_id: Uuid,
    rule_id: Uuid,
    host_id: Uuid,
    severity: &str,
    message: &str,
    hostname: &str,
) {
    // Check throttle: allow if (1) resolved state or (2) within burst window and first notification
    let should_notify = {
        let mut throttle = NOTIFICATION_THROTTLE.lock().unwrap();
        let key = (rule_id, host_id);
        let last_notified = throttle.get(&key).copied();
        
        if severity == "resolved" {
            // Always notify on resolution
            throttle.insert(key, Instant::now());
            true
        } else if let Some(last_time) = last_notified {
            // Check if 15 minutes have passed
            if last_time.elapsed() >= THROTTLE_DURATION {
                throttle.insert(key, Instant::now());
                true
            } else {
                false
            }
        } else {
            // First notification for this rule+host
            throttle.insert(key, Instant::now());
            true
        }
    };

    if !should_notify {
        info!("notification throttled for rule {} host {}", rule_id, host_id);
        return;
    }

    // Get account notification settings
    let account = sqlx::query_as::<_, (bool, Option<String>, String)>(
        "SELECT notify_email, slack_webhook, email FROM accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(db)
    .await;

    let Ok(Some((notify_email, slack_webhook, email))) = account else {
        return;
    };

    let emoji = match severity {
        "critical" => "🔴",
        "warning" => "🟡",
        "resolved" => "✅",
        _ => "ℹ️",
    };

    // Slack notification
    if let Some(ref webhook_url) = slack_webhook {
        let slack_text = format!("{} *{}* — {}", emoji, severity.to_uppercase(), message);
        let payload = serde_json::json!({ "text": slack_text });

        match ureq::post(webhook_url)
            .set("Content-Type", "application/json")
            .send_json(payload)
        {
            Ok(_) => info!("slack notification sent for {}", hostname),
            Err(e) => warn!("slack notification failed: {}", e),
        }
    }

    // Email notification via Resend
    if notify_email {
        if let Some(ref api_key) = config.resend_api_key {
            if !api_key.is_empty() {
                let subject = format!("{} [{}] {}", emoji, severity.to_uppercase(), message);
                let body = serde_json::json!({
                    "from": "NetWatch <onboarding@resend.dev>",
                    "to": [email],
                    "subject": subject,
                    "text": format!("NetWatch Alert\n\nHost: {}\nStatus: {}\n\n{}", hostname, severity.to_uppercase(), message),
                });

                match ureq::post("https://api.resend.com/emails")
                    .set("Authorization", &format!("Bearer {}", api_key))
                    .set("Content-Type", "application/json")
                    .send_json(body)
                {
                    Ok(_) => info!("email notification sent to {}", email),
                    Err(e) => warn!("email notification failed: {}", e),
                }
            }
        }
    }
}
