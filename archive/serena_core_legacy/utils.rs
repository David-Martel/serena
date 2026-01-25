//! Utility functions for MCP tools

/// Truncate output to a maximum number of characters.
///
/// # Parameters
/// - `content`: The content to truncate
/// - `max_chars`: Maximum characters to return. -1 for unlimited.
///
/// # Returns
/// Truncated content with a message if truncation occurred.
pub fn truncate_output(content: String, max_chars: i32) -> String {
    if max_chars < 0 {
        // No limit
        content
    } else {
        let max = max_chars as usize;
        if content.len() > max {
            let truncated = content.chars().take(max).collect::<String>();
            format!(
                "{}...\n[Output truncated at {} chars. {} chars total.]",
                truncated,
                max,
                content.len()
            )
        } else {
            content
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_unlimited() {
        let content = "Hello, world!".to_string();
        assert_eq!(truncate_output(content.clone(), -1), content);
    }

    #[test]
    fn test_truncate_no_limit_needed() {
        let content = "Short".to_string();
        assert_eq!(truncate_output(content.clone(), 100), content);
    }

    #[test]
    fn test_truncate_applies() {
        let content = "This is a long string that should be truncated".to_string();
        let result = truncate_output(content.clone(), 10);
        assert!(result.starts_with("This is a "));
        assert!(result.contains("[Output truncated"));
        assert!(result.contains("10 chars"));
        assert!(result.contains(&format!("{} chars total", content.len())));
    }

    #[test]
    fn test_truncate_exact_limit() {
        let content = "12345".to_string();
        assert_eq!(truncate_output(content.clone(), 5), content);
    }

    #[test]
    fn test_truncate_unicode() {
        let content = "Hello 👋 World 🌍".to_string();
        let result = truncate_output(content, 10);
        assert!(result.len() >= 10);
        assert!(result.contains("[Output truncated"));
    }
}
