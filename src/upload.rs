//! Imgur anonymous image upload.

/// Default anonymous Imgur client ID. Override with the `HYDROSHOT_IMGUR_CLIENT_ID`
/// environment variable if you want to use your own.
const DEFAULT_IMGUR_CLIENT_ID: &str = "546c25a59c58ad7";

fn imgur_client_id() -> String {
    std::env::var("HYDROSHOT_IMGUR_CLIENT_ID").unwrap_or_else(|_| DEFAULT_IMGUR_CLIENT_ID.to_string())
}

pub fn upload_to_imgur(png_data: &[u8]) -> Result<String, String> {
    let base64_image = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(png_data)
    };

    let client_id = imgur_client_id();
    let response = ureq::post("https://api.imgur.com/3/image")
        .set("Authorization", &format!("Client-ID {}", client_id))
        .send_form(&[("image", &base64_image), ("type", "base64")])
        .map_err(|e| format!("Upload failed: {}", e))?;

    let body: String = response.into_string().map_err(|e| e.to_string())?;

    extract_link(&body).ok_or_else(|| "Failed to parse Imgur response".to_string())
}

fn extract_link(json: &str) -> Option<String> {
    // Find the "link" key robustly by scanning for the key with flexible whitespace
    let link_key = json.find("\"link\"")?;
    let after_key = &json[link_key + 6..]; // skip past "link"
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    // Value should start with a quote
    if !after_colon.starts_with('"') {
        return None;
    }
    let value_start = 1; // skip opening quote
    let value_content = &after_colon[value_start..];
    // Find closing quote, handling escaped quotes
    let mut end = 0;
    let bytes = value_content.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'\\' {
            end += 2; // skip escaped character
        } else if bytes[end] == b'"' {
            break;
        } else {
            end += 1;
        }
    }
    if end >= bytes.len() {
        return None;
    }
    let link = value_content[..end].replace("\\/", "/");
    Some(link)
}
