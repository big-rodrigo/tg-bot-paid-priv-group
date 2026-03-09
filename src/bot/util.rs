/// Escapes HTML special characters for safe embedding in Telegram HTML messages.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
