use crate::config::Snippet;

fn normalized_trigger(value: &str) -> String {
    let trimmed = value
        .trim()
        .trim_matches(|c: char| matches!(c, '.' | ',' | '!' | '?' | ':' | ';' | '"' | '\''));
    // Lowercase before stripping the spoken "snippet" prefix so STT capitalization
    // ("Snippet my signature.") still expands the macro.
    let lowered = trimmed.to_lowercase();
    lowered
        .strip_prefix("snippet ")
        .unwrap_or(lowered.as_str())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Expand only a complete utterance. This keeps ordinary prose from accidentally
/// triggering a macro when it happens to contain a configured phrase.
pub fn expand_snippet<'a>(transcript: &'a str, snippets: &'a [Snippet]) -> Option<&'a str> {
    let spoken = normalized_trigger(transcript);
    snippets
        .iter()
        .filter(|snippet| snippet.enabled && !snippet.trigger.trim().is_empty())
        .find(|snippet| normalized_trigger(&snippet.trigger) == spoken)
        .map(|snippet| snippet.expansion.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippets() -> Vec<Snippet> {
        vec![Snippet {
            id: "signature".into(),
            trigger: "my signature".into(),
            expansion: "Best,\nAveek".into(),
            enabled: true,
        }]
    }

    #[test]
    fn expands_exact_trigger_with_punctuation() {
        assert_eq!(
            expand_snippet("My signature.", &snippets()),
            Some("Best,\nAveek")
        );
        assert_eq!(
            expand_snippet("snippet my signature", &snippets()),
            Some("Best,\nAveek")
        );
        assert_eq!(
            expand_snippet("Snippet my signature.", &snippets()),
            Some("Best,\nAveek")
        );
    }

    #[test]
    fn does_not_expand_trigger_inside_sentence() {
        assert_eq!(
            expand_snippet("Please include my signature", &snippets()),
            None
        );
    }
}
