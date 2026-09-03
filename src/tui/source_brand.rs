use std::sync::OnceLock;

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBrand {
    pub(crate) mark: &'static str,
    pub(crate) color: Color,
}

pub(crate) fn source_brand(source: &str) -> SourceBrand {
    match source {
        // Keep Unicode fallbacks for brands whose Nerd Font glyph coverage is
        // inconsistent across commonly installed font versions.
        "claude-code" => brand(None, "✳", 217, 119, 87),
        "codex" => brand(None, "◎", 16, 163, 127),
        "opencode" => brand(Some(""), "◇", 245, 166, 35),
        "pi" => brand(None, "π", 190, 120, 255),
        "omp" => brand(Some(""), "⌥", 255, 139, 61),
        "antigravity-cli" => brand(Some(""), "△", 139, 92, 246),
        "gemini-cli" => brand(Some(""), "✦", 66, 133, 244),
        "grok" => brand(None, "𝕏", 210, 210, 210),
        "kiro-cli" => brand(None, "◆", 152, 101, 245),
        "copilot-cli" => brand(Some(""), "∞", 137, 87, 229),
        "copilot-chat" => brand(Some(""), "◉", 94, 129, 172),
        "cursor" => brand(None, "▰", 190, 190, 190),
        "cline" => brand(Some(""), "◈", 238, 105, 80),
        "roo" => brand(None, "◒", 62, 180, 137),
        "deepseek-harness" => brand(None, "◫", 77, 107, 254),
        "kimi-code" => brand(Some(""), "☾", 105, 102, 255),
        "qwen-code" => brand(None, "Q", 99, 102, 241),
        "kilo-code" => brand(None, "K", 0, 188, 212),
        "crush" => brand(None, "C", 236, 72, 153),
        "mimo-code" => brand(None, "M", 251, 146, 60),
        "zcode" => brand(None, "Z", 72, 187, 120),
        "goose" => brand(None, "G", 242, 184, 70),
        _ => brand(None, "•", 120, 200, 120),
    }
}

fn brand(
    nerd_font_mark: Option<&'static str>,
    fallback_mark: &'static str,
    r: u8,
    g: u8,
    b: u8,
) -> SourceBrand {
    let mark =
        if nerd_icons_enabled() { nerd_font_mark.unwrap_or(fallback_mark) } else { fallback_mark };
    SourceBrand { mark, color: Color::Rgb(r, g, b) }
}

fn nerd_icons_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        !matches!(
            std::env::var("RECALL_ICON_STYLE").ok().as_deref(),
            Some("plain" | "unicode" | "ascii" | "off" | "0")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTED_SOURCES: &[&str] = &[
        "claude-code",
        "opencode",
        "codex",
        "pi",
        "omp",
        "antigravity-cli",
        "gemini-cli",
        "grok",
        "kiro-cli",
        "copilot-cli",
        "copilot-chat",
        "cursor",
        "cline",
        "roo",
        "deepseek-harness",
        "kimi-code",
        "qwen-code",
        "kilo-code",
        "crush",
        "mimo-code",
        "zcode",
        "goose",
    ];

    #[test]
    fn known_sources_have_distinct_brand_visuals() {
        assert_ne!(source_brand("claude-code"), source_brand("codex"));
        assert_ne!(source_brand("gemini-cli"), source_brand("deepseek-harness"));
        assert_ne!(source_brand("omp"), source_brand("pi"));
    }

    #[test]
    fn every_supported_brand_has_a_non_empty_mark() {
        for source in SUPPORTED_SOURCES {
            assert!(!source_brand(source).mark.is_empty(), "missing mark for {source}");
        }
    }
}
