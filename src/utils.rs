use std::io::Write;
use std::io::stdout;
use std::time::Instant;

pub fn print_safe<'a>(name: &'a str, max_len: usize) -> String {
    let mut s = String::new();
    s.push('"');
    if name.len() > max_len {
        s += &escape_string_chars(name.chars().take(max_len));
        s += "...";
    } else {
        s.push_str(&escape_string(name));
    }
    s.push('"');

    s
}

/// Escapes special characters in strings for display
pub fn escape_string(s: &str) -> String {
    escape_string_chars(s.chars())
}

pub fn escape_string_chars(s: impl IntoIterator<Item = char>) -> String {
    s.into_iter()
        .flat_map(|c| match c {
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            c if c < ' ' || c > '~' => vec!['?'],
            c => vec![c],
        })
        .collect()
}

/// Formats bytes into human-readable format
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}k", bytes as f64 / KB as f64)
    } else {
        format!("{}b", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("tab\there"), "tab\\there");
        assert_eq!(escape_string("quote\"test"), "quote\\\"test");
        assert_eq!(escape_string("backslash\\test"), "backslash\\\\test");
        assert_eq!(escape_string("normal text"), "normal text");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500b");
        assert_eq!(format_bytes(1024), "1.00k");
        assert_eq!(format_bytes(1536), "1.50k");
        assert_eq!(format_bytes(1048576), "1.00M");
        assert_eq!(format_bytes(1073741824), "1.00G");
    }
}

pub struct Timer {
    name: String,
    start: Instant,
}

pub fn start_timer(name: String) -> Timer {
    eprint!("{}... ", name);
    let _ = stdout().flush();
    Timer {
        name,
        start: Instant::now(),
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = Instant::now() - self.start;
        eprintln!("Done ({:?})", duration);
    }
}
