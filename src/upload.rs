//! Imgur anonymous image upload.

fn imgur_client_id(config_id: &str) -> Result<String, String> {
    // Environment variable overrides config
    if let Ok(env_id) = std::env::var("HYDROSHOT_IMGUR_CLIENT_ID") {
        if !env_id.is_empty() {
            return Ok(env_id);
        }
    }
    // Fall back to config setting
    if !config_id.is_empty() {
        return Ok(config_id.to_string());
    }
    Err(
        "No Imgur client ID configured. Set one in Settings > General, \
         or via the HYDROSHOT_IMGUR_CLIENT_ID environment variable."
            .to_string(),
    )
}

pub fn upload_to_imgur(png_data: &[u8], config_client_id: &str) -> Result<String, String> {
    let base64_image = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(png_data)
    };

    let client_id = imgur_client_id(config_client_id)?;
    let response = ureq::post("https://api.imgur.com/3/image")
        .set("Authorization", &format!("Client-ID {}", client_id))
        .timeout(std::time::Duration::from_secs(30))
        .send_form(&[("image", &base64_image), ("type", "base64")])
        .map_err(|e| format!("Upload failed: {}", e))?;

    let body: String = response.into_string().map_err(|e| e.to_string())?;

    extract_link(&body).ok_or_else(|| "Failed to parse Imgur response".to_string())
}

fn extract_link(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v["data"]["link"].as_str().map(|s| s.to_string())
}
