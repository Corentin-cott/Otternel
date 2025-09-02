/// Sends a simple message via Discord Webhook for a specific identity.
///
/// # Parameters
/// - webhook_identity: Which webhook configuration to use ("otternel", "mineotter", or "multiloutre").
/// - content: The message content to send to the Discord webhook.
///
/// # Returns
/// Ok(()) if the webhook is sent or disabled; Err(String) if an error occurs while sending.
///
/// # Errors
/// Returns an error if configuration fails to load, identity is unknown, or the HTTP request fails.
pub fn send_discord_content(webhook_identity: &str, content: &str) -> Result<(), String> {
    let cfg = crate::config::Config::from_env()
        .map_err(|e| format!("config error: {e}"))?;

    let (activated, url) = match webhook_identity.trim().to_ascii_lowercase().as_str() {
        "otternel" => (
            cfg.otternel_webhook_activated.trim(),
            cfg.otternel_webhook_url.trim(),
        ),
        "mineotter" => (
            cfg.mineotter_bot_webhook_activated.trim(),
            cfg.mineotter_bot_webhook_url.trim(),
        ),
        "multiloutre" => (
            cfg.multiloutre_bot_webhook_activated.trim(),
            cfg.multiloutre_bot_webhook_url.trim(),
        ),
        other => {
            return Err(format!("unknown webhook identity: {other}"));
        }
    };

    if !activated.eq_ignore_ascii_case("true") || url.is_empty() {
        // Treat disabled as a no-op success to avoid spamming callers with errors.
        return Ok(());
    }

    let body = serde_json::json!({ "content": content });

    let resp = ureq::post(url)
        .set("Content-Type", "application/json")
        .send_json(body);

    match resp {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("webhook send error: {e}")),
    }
}
