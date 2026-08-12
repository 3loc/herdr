pub(crate) fn stripped_terminal_title(
    title: &str,
    agent: Option<crate::detect::Agent>,
) -> Option<String> {
    let title = crate::platform::terminal_title_for_presentation(title).trim();
    if title.is_empty() {
        return None;
    }

    let mut chars = title.char_indices();
    let (_, first) = chars.next()?;
    let after_first = &title[first.len_utf8()..];
    let recognized = matches!(first, '\u{2800}'..='\u{28ff}')
        || agent.is_some_and(|agent| {
            crate::detect::manifest::terminal_title_activity_matches(
                agent,
                &title[..first.len_utf8()],
            )
        });
    let stripped = if recognized
        && (after_first.is_empty() || after_first.chars().next().is_some_and(char::is_whitespace))
    {
        after_first.trim()
    } else {
        title
    };

    (!stripped.is_empty()).then(|| stripped.to_string())
}

#[cfg(test)]
mod tests {
    use super::stripped_terminal_title;
    use crate::detect::Agent;

    #[test]
    fn manifest_activity_glyph_is_scoped_to_the_effective_agent() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        assert_eq!(
            stripped_terminal_title("◐ task", Some(Agent::Claude)).as_deref(),
            Some("task")
        );
        assert_eq!(
            stripped_terminal_title("◐ task", Some(Agent::Codex)).as_deref(),
            Some("◐ task")
        );
        assert_eq!(
            stripped_terminal_title("◐ task", None).as_deref(),
            Some("◐ task")
        );
    }

    #[test]
    fn strips_one_recognized_leading_activity_glyph() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        for title in [
            "⠋ task",
            "✳ task",
            "  ⠙   task  ",
            "✢ task",
            "✻ task",
            "◐ task",
            "◓ task",
            "◑ task",
            "◒ task",
        ] {
            assert_eq!(
                stripped_terminal_title(title, Some(Agent::Claude)).as_deref(),
                Some("task")
            );
        }
        assert_eq!(
            stripped_terminal_title("⠋ ⠙ task", None).as_deref(),
            Some("⠙ task")
        );
    }

    #[test]
    fn preserves_unrecognized_or_unbounded_symbols() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        for (title, expected) in [
            ("★task", "★task"),
            ("◐task", "◐task"),
            ("★ production", "★ production"),
            ("✨ task", "✨ task"),
            ("☼ status", "☼ status"),
            ("@ task", "@ task"),
            ("task ⠋ detail", "task ⠋ detail"),
            ("[prod] task", "[prod] task"),
        ] {
            assert_eq!(
                stripped_terminal_title(title, Some(Agent::Claude)).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn preserves_unicode_text_and_elides_empty_results() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        assert_eq!(
            stripped_terminal_title(" ⠋ 修复🙂标题 ", None).as_deref(),
            Some("修复🙂标题")
        );
        assert_eq!(stripped_terminal_title("  ", None), None);
        assert_eq!(stripped_terminal_title("⠋   ", None), None);
    }

    #[cfg(windows)]
    #[test]
    fn strips_one_windows_elevation_decoration_before_activity_glyph() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        assert_eq!(
            stripped_terminal_title("Administrator:   ⠋ task", None).as_deref(),
            Some("task")
        );
        assert_eq!(
            stripped_terminal_title("Administrator: Administrator: task", None).as_deref(),
            Some("Administrator: task")
        );
        assert_eq!(stripped_terminal_title("Administrator: ", None), None);
    }
}
