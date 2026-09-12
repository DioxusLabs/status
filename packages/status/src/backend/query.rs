/// GitHub-like query tokens: `repo:x author:y label:z -label:w is:draft
/// is:green is:red is:approved is:conflict is:stale is:first-timer` + free text.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ParsedQuery {
    pub repos: Vec<String>,
    pub authors: Vec<String>,
    pub labels: Vec<String>,
    pub exclude_labels: Vec<String>,
    pub is_flags: Vec<String>,
    pub text: String,
}

pub fn parse_query(input: &str) -> ParsedQuery {
    let mut out = ParsedQuery::default();
    let mut text = Vec::new();
    for tok in input.split_whitespace() {
        if let Some((key, value)) = tok.split_once(':') {
            let (key, value, neg) = if let Some(v) = value.strip_prefix('-') {
                (key, v, true)
            } else {
                (key, value, false)
            };
            match key {
                "repo" => out.repos.push(value.to_string()),
                "author" => out.authors.push(value.to_string()),
                "label" => {
                    if neg {
                        out.exclude_labels.push(value.to_string());
                    } else {
                        out.labels.push(value.to_string());
                    }
                }
                "-label" => out.exclude_labels.push(value.to_string()),
                "is" => out.is_flags.push(value.to_string()),
                _ => text.push(tok),
            }
        } else {
            text.push(tok);
        }
    }
    out.text = text.join(" ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tokens_and_text() {
        let q = parse_query("repo:dioxus author:ealmloff is:green -label:blocked fix crash");
        assert_eq!(q.repos, ["dioxus"]);
        assert_eq!(q.authors, ["ealmloff"]);
        assert_eq!(q.is_flags, ["green"]);
        assert_eq!(q.exclude_labels, ["blocked"]);
        assert_eq!(q.text, "fix crash");
    }

    #[test]
    fn bare_text() {
        let q = parse_query("hello world");
        assert_eq!(q.text, "hello world");
        assert!(q.repos.is_empty());
    }
}
