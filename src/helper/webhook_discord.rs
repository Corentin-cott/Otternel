/// Sends a Discord embed via a webhook for a specific identity.
///
/// # Parameters
/// - webhook_identity: Which webhook configuration to use ("otternel", "mineotter", or "multiloutre").
/// - content: The message content to send to the Discord webhook.
/// - title: Embed title.
/// - title_hyperlink: Link in the embed title.
/// - supertext: Description/body of the embed.
/// - color_rgb: Color of the embed (0x000000..=0xFFFFFF), as a hex string like "#RRGGBB" or "RRGGBB".
/// - thumbnail_url: URL of the thumbnail image.
/// - image_url: URL of the main image for the embed (displayed full width below the description).
/// - footer_image_url: URL of the footer icon.
/// - footer_text: Footer text.
/// - timestamp_iso8601: Optional ISO 8601 datetime string to show as embed timestamp (e.g. RFC3339).
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
    title_hyperlink: &str,
    supertext: &str,
    color_rgb: Option<String>,
    thumbnail_url: &str,
    image_url: &str,
    footer_image_url: &str,
    footer_text: &str,
    timestamp_iso8601: Option<String>,
) -> Result<(), String> {
    // Get the webhook configuration
    let (_identity, activated, url) = get_webhook_config(webhook_identity)?;
    if !activated.eq_ignore_ascii_case("true") || url.is_empty() {
        return Ok(());
    }

    // Build the embed without color first
    let mut embed = serde_json::json!({
        "title": title,
        "description": supertext,
    });

    // Set a hyperlink on the title if provided (Discord uses "url" on the embed)
    if !title_hyperlink.trim().is_empty() {
        embed["url"] = serde_json::json!(title_hyperlink);
    }

    // Sanitize and set color if provided
    if let Some(c) = color_rgb
        .as_deref()
        .filter(|s| *s != "0" && !s.is_empty())
        .and_then(parse_discord_color)
    {
        embed["color"] = serde_json::json!(c as i64);
    }

    // Add the thumbnail if there is one
    if !thumbnail_url.trim().is_empty() {
        embed["thumbnail"] = serde_json::json!({ "url": thumbnail_url });
    }

    // Add a main image if provided
    if !image_url.trim().is_empty() {
        embed["image"] = serde_json::json!({ "url": image_url });
    }

    // Add footer if provided
    if !footer_text.trim().is_empty() || !footer_image_url.trim().is_empty() {
        let mut footer = serde_json::json!({ "text": footer_text });
        if !footer_image_url.trim().is_empty() {
            footer["icon_url"] = serde_json::json!(footer_image_url);
        }
        embed["footer"] = footer;
    }

    // Add timestamp if provided
    if let Some(ts) = timestamp_iso8601.as_ref().filter(|s| !s.trim().is_empty()) {
        embed["timestamp"] = serde_json::json!(ts);
    }

    // Build the payload
    let mut payload = serde_json::json!({
        "embeds": [embed],
        // "allowed_mentions": { "parse": [] } // If uncommented, mentions will be parsed
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
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(format!("webhook send error: status code {code}, body: {body}"))
        }
        Err(e) => Err(format!("webhook send error: {e}")),
    }
}

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

/// Parses a Discord color string to a u32 integer.
/// Accepts formats like:
/// - "#RRGGBB"
/// - "0xRRGGBB"
/// - "RRGGBB"
/// - decimal "16711680"
fn parse_discord_color(s: &str) -> Option<u32> {
    let t = s.trim();

    // If decimal
    if t.chars().all(|c| c.is_ascii_digit()) {
        return t.parse::<u32>().ok();
    }

    // Normalize hex-like strings
    let t = t.strip_prefix('#').unwrap_or(t);
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);

    u32::from_str_radix(t, 16).ok()
}

pub fn get_webhook_identity_by_server_id(game: String) -> &'static str {
    match game.to_lowercase().as_str() {
        "minecraft" => "mineotter",
        "palworld" => "multiloutre",
        _ => "otternel",
    }
}