use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub struct Attachment {
    pub filename: String,
    pub content_base64: String,
    pub mime_type: String,
}

fn base64url(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

/// Encodes a header value as RFC 2047 when it contains non-ASCII characters.
fn encode_header_value(value: &str) -> String {
    if value.is_ascii() {
        value.replace(['\r', '\n'], " ")
    } else {
        format!(
            "=?UTF-8?B?{}?=",
            BASE64.encode(value.replace(['\r', '\n'], " "))
        )
    }
}

/// Builds an RFC 2822 message. Plain text when no attachment, multipart/mixed otherwise.
pub fn build_mime(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    attachment: Option<&Attachment>,
) -> String {
    let date = chrono::Utc::now().to_rfc2822();
    let mut out = String::new();
    out.push_str(&format!("From: {from}\r\n"));
    out.push_str(&format!("To: {to}\r\n"));
    out.push_str(&format!("Subject: {}\r\n", encode_header_value(subject)));
    out.push_str(&format!("Date: {date}\r\n"));

    match attachment {
        None => {
            out.push_str("MIME-Version: 1.0\r\n");
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: 8bit\r\n");
            out.push_str("\r\n");
            out.push_str(body);
        }
        Some(att) => {
            let boundary = format!("copilot-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
            out.push_str("MIME-Version: 1.0\r\n");
            out.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n"
            ));
            out.push_str("\r\n");
            out.push_str(&format!("--{boundary}\r\n"));
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: 8bit\r\n\r\n");
            out.push_str(body);
            out.push_str("\r\n");
            out.push_str(&format!("--{boundary}\r\n"));
            out.push_str(&format!("Content-Type: {};\r\n", att.mime_type));
            out.push_str(&format!(
                "Content-Disposition: attachment; filename=\"{}\"\r\n",
                att.filename.replace('"', "'")
            ));
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&chunk_base64(&att.content_base64));
            out.push_str(&format!("\r\n--{boundary}--\r\n"));
        }
    }
    out
}

/// Gmail expects base64url-encoded raw messages; this wraps the full MIME text.
pub fn to_gmail_raw(mime: &str) -> String {
    base64url(mime.as_bytes())
}

/// Wraps base64 content at 76 chars per line per MIME spec.
fn chunk_base64(data: &str) -> String {
    let mut out = String::with_capacity(data.len() + data.len() / 38);
    let bytes = data.as_bytes();
    for (i, chunk) in bytes.chunks(76).enumerate() {
        if i > 0 {
            out.push_str("\r\n");
        }
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
    }
    out
}
