use crate::error::Result;
use crate::parser::uri::parse_uri;
use crate::protocols::OutboundConfig;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

pub fn parse_subscription(content: &str) -> Result<Vec<(String, OutboundConfig)>> {
    let trimmed = content.trim();

    // Check if the entire payload is base64 encoded
    let decoded_str = if !trimmed.contains('\n') && !trimmed.contains("://") {
        let cleaned = trimmed.replace(['\r', '\n', ' '], "");
        if let Ok(bytes) = STANDARD
            .decode(&cleaned)
            .or_else(|_| URL_SAFE_NO_PAD.decode(&cleaned))
        {
            String::from_utf8_lossy(&bytes).to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    };

    let mut nodes = Vec::new();

    for line in decoded_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        match parse_uri(line) {
            Ok(node) => nodes.push(node),
            Err(e) => {
                tracing::debug!("Skipping unparseable line '{}': {}", line, e);
            }
        }
    }

    Ok(nodes)
}
