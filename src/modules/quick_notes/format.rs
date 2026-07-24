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

    Some(content[prefix_end..end].trim_end_matches('\n'))
}

pub fn replace_message(content: &str, msg_num: u32, new_text: &str) -> String {
    let bounds = message_bounds(content);
    let pos = bounds.iter().position(|b| b.num == msg_num)
        .expect("message not found");

    let start = bounds[pos].prefix_start;
    let end = bounds.get(pos + 1).map(|b| b.prefix_start).unwrap_or(content.len());

    let before = &content[..start];
    let after = &content[end..];

    format!("{}---{}---\n{}\n{}", before, msg_num, new_text, after)
}

pub fn append_message(content: &str, msg_num: u32, text: &str) -> String {
    let suffix = if content.ends_with("\n\n") { "" } else if content.ends_with('\n') { "\n" } else { "\n\n" };
    format!("{}{}---{}---\n\n{}\n", content, suffix, msg_num, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_message() {
        let content = "---\ncreated: test\n---\n\n---1---\nhello\n\n---2---\nworld\n";
        assert_eq!(find_message(content, 1), Some("hello"));
        assert_eq!(find_message(content, 2), Some("world"));
        assert_eq!(find_message(content, 3), None);
    }

    #[test]
    fn test_find_message_with_double_digit() {
        let content = "---\n---1---\na\n---9---\ni\n---10---\nj\n---11---\nk\n";
        assert_eq!(find_message(content, 1), Some("a"));
        assert_eq!(find_message(content, 9), Some("i"));
        assert_eq!(find_message(content, 10), Some("j"));
        assert_eq!(find_message(content, 11), Some("k"));
    }

    #[test]
    fn test_replace_message() {
        let content = "---\ncreated: test\n---\n\n---1---\nhello\n\n---2---\nworld\n";
        let result = replace_message(content, 1, "replaced");
        assert!(result.contains("---1---\nreplaced\n"));
        assert!(result.contains("---2---\nworld"));
    }

    #[test]
    fn test_append_message() {
        let content = "---\ncreated: test\n---\n\n---1---\nhello\n";
        let result = append_message(content, 2, "world");
        assert!(result.contains("---2---\n\nworld"));
    }
}
