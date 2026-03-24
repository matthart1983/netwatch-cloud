use crate::config::ServerConfig;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn send_alert(
    db: &sqlx::PgPool,
    config: &ServerConfig,
    account_id: Uuid,
    severity: &str,
    message: &str,
    hostname: &str,
) {
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
                    "from": "NetWatch <alerts@netwatch.dev>",
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
