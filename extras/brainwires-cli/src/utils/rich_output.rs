use console::style;

/// Rich terminal output utilities
pub struct RichOutput;

impl RichOutput {
    /// Create a boxed message
    pub fn boxed<S: AsRef<str>>(message: S, title: Option<&str>, color: &str) -> String {
        let message = message.as_ref();
        let lines: Vec<&str> = message.lines().collect();

        // Calculate box width
        let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let title_width = title.map(|t| t.len() + 2).unwrap_or(0);
        let width = max_width.max(title_width).max(40);

        let mut output = String::new();

        // Top border
        output.push_str(&format!("┌{}┐\n", "─".repeat(width + 2)));

        // Title (if provided)
        if let Some(title_text) = title {
            let padding = (width - title_text.len()) / 2;
            output.push_str(&format!(
                "│ {}{}{} │\n",
                " ".repeat(padding),
                Self::colorize(title_text, color),
                " ".repeat(width - padding - title_text.len())
            ));
            output.push_str(&format!("├{}┤\n", "─".repeat(width + 2)));
        }

        // Content lines
        for line in lines {
            let padding = width - line.len();
            output.push_str(&format!("│ {}{} │\n", line, " ".repeat(padding)));
        }

        // Bottom border
        output.push_str(&format!("└{}┘", "─".repeat(width + 2)));

        output
    }

    /// Colorize text
    fn colorize(text: &str, color: &str) -> String {
        match color {
            "red" => style(text).red().to_string(),
            "green" => style(text).green().to_string(),
            "blue" => style(text).blue().to_string(),
            "yellow" => style(text).yellow().to_string(),
            "cyan" => style(text).cyan().to_string(),
            "magenta" => style(text).magenta().to_string(),
            "white" => style(text).white().to_string(),
            "gray" => style(text).dim().to_string(),
            "bold" => style(text).bold().to_string(),
            _ => text.to_string(),
        }
    }

    /// Create a horizontal separator
    pub fn separator(width: usize) -> String {
        "─".repeat(width)
    }

    /// Create a section header
    pub fn header<S: AsRef<str>>(text: S, color: &str) -> String {
        let text = text.as_ref();
        format!("\n{}\n{}\n", Self::colorize(text, color), Self::separator(text.len()))
    }

    /// Create a table row
    pub fn table_row(columns: &[String], widths: &[usize]) -> String {
        let mut row = String::from("│");
        for (_i, (col, width)) in columns.iter().zip(widths.iter()).enumerate() {
            let padding = width.saturating_sub(col.len());
            row.push_str(&format!(" {}{} │", col, " ".repeat(padding)));
        }
        row
    }

    /// Create a table header
    pub fn table_header(headers: &[String], widths: &[usize]) -> String {
        let mut output = String::new();

        // Top border
        output.push('┌');
        for (i, width) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < widths.len() - 1 {
                output.push('┬');
            }
        }
        output.push_str("┐\n");

        // Header row
        output.push_str(&Self::table_row(headers, widths));
        output.push('\n');

        // Separator
        output.push('├');
        for (i, width) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < widths.len() - 1 {
                output.push('┼');
            }
        }
        output.push('┤');

        output
    }

    /// Create a table footer
    pub fn table_footer(widths: &[usize]) -> String {
        let mut output = String::from("└");
        for (i, width) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(width + 2));
            if i < widths.len() - 1 {
                output.push('┴');
            }
        }
        output.push('┘');
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxed() {
        let output = RichOutput::boxed("Hello, World!", Some("Greeting"), "green");
        assert!(output.contains("Hello, World!"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
    }

    #[test]
    fn test_boxed_without_title() {
        let output = RichOutput::boxed("Content only", None, "blue");
        assert!(output.contains("Content only"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
        assert!(!output.contains("├")); // No title separator
    }

    #[test]
    fn test_boxed_multiline() {
        let output = RichOutput::boxed("Line 1\nLine 2\nLine 3", Some("Multi"), "red");
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2"));
        assert!(output.contains("Line 3"));
        assert!(output.contains("Multi"));
    }

    #[test]
    fn test_colorize_red() {
        let output = RichOutput::colorize("test", "red");
        // Should contain the text (exact styling depends on console crate)
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_green() {
        let output = RichOutput::colorize("test", "green");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_blue() {
        let output = RichOutput::colorize("test", "blue");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_yellow() {
        let output = RichOutput::colorize("test", "yellow");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_cyan() {
        let output = RichOutput::colorize("test", "cyan");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_magenta() {
        let output = RichOutput::colorize("test", "magenta");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_white() {
        let output = RichOutput::colorize("test", "white");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_gray() {
        let output = RichOutput::colorize("test", "gray");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_bold() {
        let output = RichOutput::colorize("test", "bold");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_colorize_unknown() {
        let output = RichOutput::colorize("test", "unknown");
        assert_eq!(output, "test"); // Unknown colors return plain text
    }

    #[test]
    fn test_separator() {
        let sep = RichOutput::separator(10);
        assert_eq!(sep.chars().count(), 10); // Count chars, not bytes
        assert_eq!(sep, "──────────");
    }

    #[test]
    fn test_separator_empty() {
        let sep = RichOutput::separator(0);
        assert_eq!(sep.len(), 0);
        assert_eq!(sep, "");
    }

    #[test]
    fn test_header() {
        let output = RichOutput::header("Test Header", "blue");
        assert!(output.contains("Test Header"));
        assert!(output.contains("─"));
    }

    #[test]
    fn test_table_row() {
        let columns = vec!["Col1".to_string(), "Col2".to_string()];
        let widths = vec![10, 15];
        let row = RichOutput::table_row(&columns, &widths);
        assert!(row.contains("Col1"));
        assert!(row.contains("Col2"));
        assert!(row.starts_with("│"));
    }

    #[test]
    fn test_table_row_empty() {
        let columns = vec![];
        let widths = vec![];
        let row = RichOutput::table_row(&columns, &widths);
        assert_eq!(row, "│");
    }

    #[test]
    fn test_table_header() {
        let headers = vec!["Name".to_string(), "Value".to_string()];
        let widths = vec![10, 15];
        let header = RichOutput::table_header(&headers, &widths);
        assert!(header.contains("Name"));
        assert!(header.contains("Value"));
        assert!(header.contains("┌"));
        assert!(header.contains("┬"));
        assert!(header.contains("├"));
        assert!(header.contains("┼"));
    }

    #[test]
    fn test_table_header_single_column() {
        let headers = vec!["Only".to_string()];
        let widths = vec![20];
        let header = RichOutput::table_header(&headers, &widths);
        assert!(header.contains("Only"));
        assert!(header.contains("┌"));
        assert!(!header.contains("┬")); // No separator for single column
    }

    #[test]
    fn test_table_footer() {
        let widths = vec![10, 15, 20];
        let footer = RichOutput::table_footer(&widths);
        assert!(footer.contains("└"));
        assert!(footer.contains("┴"));
        assert!(footer.contains("┘"));
    }

    #[test]
    fn test_table_footer_single_column() {
        let widths = vec![20];
        let footer = RichOutput::table_footer(&widths);
        assert!(footer.contains("└"));
        assert!(footer.contains("┘"));
        assert!(!footer.contains("┴")); // No separator for single column
    }

    #[test]
    fn test_table_footer_empty() {
        let widths = vec![];
        let footer = RichOutput::table_footer(&widths);
        assert_eq!(footer, "└┘");
    }
}
