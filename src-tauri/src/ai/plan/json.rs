pub fn extract_json(raw: &str) -> Option<&str> {
    let s = if let Some(fence) = raw.find("```") {
        let after = &raw[fence..];
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(after.len());
        let body = &after[body_start..];
        if let Some(end) = body.find("```") {
            &body[..end]
        } else {
            body
        }
    } else {
        raw
    };
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut end = start;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in s[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    Some(&s[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenced_json_loses_its_fence() {
        let raw = "```json\n{\"edits\":[]}\n```";
        assert_eq!(extract_json(raw), Some("{\"edits\":[]}"));
    }

    #[test]
    fn brace_inside_a_string_value_does_not_truncate_the_object() {
        let raw = r#"{"note":"skipping the idle stretch} at the start","edits":[]}"#;
        assert_eq!(extract_json(raw), Some(raw));
    }

    #[test]
    fn unmatched_brace_inside_a_string_does_not_imbalance_depth() {
        let raw = r#"{"note":"a { without a match","edits":[]}"#;
        assert_eq!(extract_json(raw), Some(raw));
    }

    #[test]
    fn escaped_quote_inside_a_string_does_not_end_the_string_early() {
        let raw = r#"{"note":"a \" quote } inside","edits":[]}"#;
        assert_eq!(extract_json(raw), Some(raw));
    }

    #[test]
    fn prose_with_no_object_in_it_is_nothing_to_parse() {
        assert_eq!(extract_json("I could not find anything to edit."), None);
        assert_eq!(extract_json(""), None);
        assert_eq!(
            extract_json("{\"edits\":["),
            None,
            "an unclosed object is not a plan"
        );
    }
}
