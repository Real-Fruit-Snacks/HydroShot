//! Imgur anonymous image upload.

const IMGUR_CLIENT_ID: &str = "546c25a59c58ad7";

pub fn upload_to_imgur(png_data: &[u8]) -> Result<String, String> {
    let base64_image = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(png_data)
    };

    let response = ureq::post("https://api.imgur.com/3/image")
        .set("Authorization", &format!("Client-ID {}", IMGUR_CLIENT_ID))
        .send_form(&[("image", &base64_image), ("type", "base64")])
        .map_err(|e| format!("Upload failed: {}", e))?;

    let body: String = response.into_string().map_err(|e| e.to_string())?;

    extract_link(&body).ok_or_else(|| "Failed to parse Imgur response".to_string())
}

fn extract_link(json: &str) -> Option<String> {
    let marker = "\"link\":\"";
    let start = json.find(marker)? + marker.len();
    let end = json[start..].find('"')? + start;
    let link = json[start..end].replace("\\/", "/");
    Some(link)
}
