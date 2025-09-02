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
///
pub fn send_discord_content(webhook_identity: &str, content: &str) -> Result<(), String> {
    // Get the webhook configuration
    let (_identity, activated, url) = get_webhook_config(webhook_identity)?;
    if !activated.eq_ignore_ascii_case("true") || url.is_empty() {
        return Ok(());
    }

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

/// Sends a Discord embed via a webhook for a specific identity.
///
/// # Parameters
/// - webhook_identity: Which webhook configuration to use ("otternel", "mineotter", or "multiloutre").
/// - content: The message content to send to the Discord webhook.
/// - title: Embed title.
/// - supertext: Link in the embed title.
/// - color_rgb: Color of the embed (0x000000..=0xFFFFFF).
/// - thumbnail_url: URL of the thumbnail image.
///
/// # Returns
/// Ok(()) if the webhook is sent or disabled; Err(String) if an error occurs while sending.
///
/// # Errors
/// Returns an error if configuration fails to load, identity is unknown, or the HTTP request fails.
///
pub fn send_discord_embed(
    webhook_identity: &str,
    content: &str,
    title: &str,
    supertext: &str,
    color_rgb: u32,
    thumbnail_url: &str,
) -> Result<(), String> {
    // Get the webhook configuration
    let (_identity, activated, url) = get_webhook_config(webhook_identity)?;
    if !activated.eq_ignore_ascii_case("true") || url.is_empty() {
        return Ok(());
    }

    // Format the embed color
    let color_sanitized = color_rgb & 0x00FF_FFFF;

    // Build the embed
    let mut embed = serde_json::json!({
        "title": title,
        "description": supertext,
        "color": color_sanitized as i64, // Discord attend un entier
    });

    // Add the thumbnail if there is one
    if !thumbnail_url.trim().is_empty() {
        embed["thumbnail"] = serde_json::json!({ "url": thumbnail_url });
    }

    // Build the payload
    let mut payload = serde_json::json!({
        "embeds": [embed],
        // "allowed_mentions": { "parse": [] } // If uncomment, mentions will be parsed
    });

    if !content.trim().is_empty() {
        payload["content"] = serde_json::Value::String(content.to_string());
    }

    // Sending
    let resp = ureq::post(url)
        .set("Content-Type", "application/json")
        .send_json(payload);

    match resp {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("webhook send error: {e}")),
    }
}

/// Retrieves the webhook configuration for a given webhook identity.
///
/// # Parameters
/// * `webhook_identity` - A string slice that identifies the webhook. Supported values are:
///     - `"otternel"`
///     - `"mineotter"`
///     - `"multiloutre"`
///
/// # Returns
/// If successful, returns a `Result` containing a tuple with three string slices:
/// - The name of the webhook identity.
/// - A static reference to whether the webhook is activated (`1` for activated, `0` for deactivated).
/// - A static reference to the webhook URL.
///
/// In case of an error, returns a `Result` with a `String` describing the issue:
/// - If the configuration cannot be loaded from the environment, an error of the form
///   `"config error: {error_message}"` is returned.
/// - If an unsupported or unknown `webhook_identity` is provided, an error of the form
///   `"unknown webhook identity: {webhook_identity}"` is returned.
///
/// # Errors
/// - Returns `"config error: {error_message}"` if loading configuration from the environment fails.
/// - Returns `"unknown webhook identity: {webhook_identity}"` for invalid webhook identities.
///
/// # Examples
/// ```rust
/// match get_webhook_config("otternel") {
///     Ok((identity, activated, url)) => {
///         println!("Webhook identity: {}", identity);
///         println!("Activated: {}", activated);
///         println!("URL: {}", url);
///     }
///     Err(err) => println!("Error: {}", err),
/// }
/// ```
///
/// # Notes
/// This function depends on the `Config` struct defined in the `crate::config` module to retrieve
/// environment-based configurations. The `from_env` method should be implemented for `Config` to
/// construct the configuration from environment variables.
///
fn get_webhook_config(webhook_identity: &str) -> Result<(&'static str, &'static str, &'static str), String> {
    let cfg = crate::config::Config::from_env()
        .map_err(|e| format!("config error: {e}"))?;

    match webhook_identity.trim().to_ascii_lowercase().as_str() {
        "otternel" => {
            let activated: &'static str = Box::leak(cfg.otternel_webhook_activated.into_boxed_str());
            let url: &'static str = Box::leak(cfg.otternel_webhook_url.into_boxed_str());
            Ok(("otternel", activated, url))
        }
        "mineotter" => {
            let activated: &'static str = Box::leak(cfg.mineotter_bot_webhook_activated.into_boxed_str());
            let url: &'static str = Box::leak(cfg.mineotter_bot_webhook_url.into_boxed_str());
            Ok(("mineotter", activated, url))
        }
        "multiloutre" => {
            let activated: &'static str = Box::leak(cfg.multiloutre_bot_webhook_activated.into_boxed_str());
            let url: &'static str = Box::leak(cfg.multiloutre_bot_webhook_url.into_boxed_str());
            Ok(("multiloutre", activated, url))
        }
        other => Err(format!("unknown webhook identity: {other}")),
    }
}
