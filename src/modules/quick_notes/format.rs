use regex::Regex;

struct MessageBounds {
    num: u32,
    prefix_start: usize,
}

fn message_bounds(content: &str) -> Vec<MessageBounds> {
    let re = Regex::new(r"(?m)^---(\d+)---\n?").unwrap();
    let matches: Vec<(u32, usize)> = re
        .captures_iter(content)
        .map(|c| {
            let num = c[1].parse().unwrap();
            let full = c.get(0).unwrap();
            (num, full.start())
        })
        .collect();

    matches
        .iter()
        .map(|&(num, prefix_start)| {
            MessageBounds { num, prefix_start }
        })
        .collect()
}

#[allow(dead_code)]
pub fn find_message(content: &str, msg_num: u32) -> Option<&str> {
    let bounds = message_bounds(content);
    let pos = bounds.iter().position(|b| b.num == msg_num)?;

    let prefix_end = bounds[pos].prefix_start + format!("---{}---", msg_num).len() + 1;
    let end = bounds.get(pos + 1).map(|b| b.prefix_start).unwrap_or(content.len());

    Some(content[prefix_end..end].trim_start_matches('\n').trim_end_matches('\n'))
}

pub fn replace_message(content: &str, msg_num: u32, new_text: &str) -> String {
    let bounds = message_bounds(content);
    let pos = bounds.iter().position(|b| b.num == msg_num)
        .expect("message not found");

    let start = bounds[pos].prefix_start;
    let end = bounds.get(pos + 1).map(|b| b.prefix_start).unwrap_or(content.len());

    let before = &content[..start];
    let after = &content[end..];

    format!("{}---{}---\n\n{}\n\n{}", before, msg_num, new_text, after)
}

pub fn append_message(content: &str, msg_num: u32, text: &str) -> String {
    let suffix = if content.ends_with("\n\n") { "" } else if content.ends_with('\n') { "\n" } else { "\n\n" };
    format!("{}{}---{}---\n\n{}\n", content, suffix, msg_num, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "---\ncreated: test\n---\n\n---1---\n\nhello\n\n---2---\n\nworld\n\n---3---\n\nthird\n";

    #[test]
    fn test_find_message() {
        assert_eq!(find_message(FILE, 1), Some("hello"));
        assert_eq!(find_message(FILE, 2), Some("world"));
        assert_eq!(find_message(FILE, 3), Some("third"));
        assert_eq!(find_message(FILE, 4), None);
    }

    #[test]
    fn test_find_message_single() {
        let content = "---\n---1---\n\nonly one\n";
        assert_eq!(find_message(content, 1), Some("only one"));
    }

    #[test]
    fn test_find_message_double_digit() {
        let content = "---\n---1---\n\na\n---9---\n\ni\n---10---\n\nj\n---11---\n\nk\n";
        assert_eq!(find_message(content, 1), Some("a"));
        assert_eq!(find_message(content, 9), Some("i"));
        assert_eq!(find_message(content, 10), Some("j"));
        assert_eq!(find_message(content, 11), Some("k"));
    }

    #[test]
    fn test_replace_first() {
        let result = replace_message(FILE, 1, "REPLACED");
        assert!(result.contains("---1---\n\nREPLACED\n\n---2---\n\nworld"));
        assert!(result.contains("---3---\n\nthird"));
    }

    #[test]
    fn test_replace_middle() {
        let result = replace_message(FILE, 2, "MIDDLE");
        assert!(result.contains("---1---\n\nhello\n\n---2---\n\nMIDDLE\n\n---3---"));
    }

    #[test]
    fn test_replace_last() {
        let result = replace_message(FILE, 3, "LAST");
        assert!(result.contains("---2---\n\nworld\n\n---3---\n\nLAST"));
    }

    #[test]
    fn test_replace_preserves_separators() {
        let result = replace_message(FILE, 1, "x");
        assert!(result.contains("---1---"));
        assert!(result.contains("---2---"));
        assert!(result.contains("---3---"));
    }

    #[test]
    fn test_append_to_single() {
        let content = "---\n---1---\n\nfirst\n";
        let result = append_message(content, 2, "second");
        assert!(result.contains("---2---\n\nsecond"));
        assert!(result.contains("---1---\n\nfirst"));
    }

    #[test]
    fn test_append_to_multiple() {
        let result = append_message(FILE, 4, "fourth");
        assert!(result.contains("---3---\n\nthird\n\n---4---\n\nfourth"));
    }

    #[test]
    fn test_append_multiline() {
        let content = "---\n---1---\n\nfirst\n";
        let result = append_message(content, 2, "line1\nline2\nline3");
        assert!(result.contains("---2---\n\nline1\nline2\nline3"));
    }

    #[test]
    fn test_full_cycle() {
        let mut content = "---\ncreated: test\n---\n\n---1---\n\nfirst\n".to_string();
        content = append_message(&content, 2, "second").to_string();
        content = append_message(&content, 3, "third").to_string();
        content = replace_message(&content, 2, "SECOND").to_string();

        assert_eq!(find_message(&content, 1), Some("first"));
        assert_eq!(find_message(&content, 2), Some("SECOND"));
        assert_eq!(find_message(&content, 3), Some("third"));
        assert!(content.contains("---1---\n\nfirst\n\n---2---\n\nSECOND\n\n---3---\n\nthird"));
    }
}
