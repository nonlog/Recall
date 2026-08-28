use std::sync::OnceLock;

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBrand {
    pub(crate) mark: &'static str,
    pub(crate) color: Color,
}

pub(crate) fn source_brand(source: &str) -> SourceBrand {
    match source {
        // Claude/OpenAI do not currently have dependable Nerd Font brand
        // glyphs across common font versions, so keep recognizable Unicode
        // marks instead of rendering a missing-glyph box.
        "claude-code" => brand(None, "✳", 217, 119, 87),
        "codex" => brand(None, "◎", 16, 163, 127),
        "opencode" => brand(Some(""), "◇", 245, 166, 35),
        "pi" => brand(None, "π", 190, 120, 255),
        "oh-my-pi" => brand(Some(""), "⌥", 255, 139, 61),
        "antigravity-cli" => brand(Some(""), "△", 139, 92, 246),
        "gemini-cli" => brand(Some(""), "✦", 66, 133, 244),
        "grok" => brand(None, "𝕏", 210, 210, 210),
        "kiro-cli" => brand(None, "◆", 152, 101, 245),
        "copilot-cli" => brand(Some(""), "∞", 137, 87, 229),
        "cursor" => brand(None, "▰", 190, 190, 190),
        "cline" => brand(Some(""), "◈", 238, 105, 80),
        "deepseek-harness" => brand(None, "◫", 77, 107, 254),
        "kimi-code" => brand(Some(""), "☾", 105, 102, 255),
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

    #[test]
    fn known_sources_have_distinct_brand_visuals() {
        assert_ne!(source_brand("claude-code"), source_brand("codex"));
        assert_ne!(source_brand("gemini-cli"), source_brand("deepseek-harness"));
    }

    #[test]
    fn every_supported_brand_has_a_non_empty_mark() {
        for source in [
            "claude-code",
            "codex",
            "opencode",
            "pi",
            "oh-my-pi",
            "antigravity-cli",
            "gemini-cli",
            "grok",
            "kiro-cli",
            "copilot-cli",
            "cursor",
            "cline",
            "deepseek-harness",
            "kimi-code",
        ] {
            assert!(!source_brand(source).mark.is_empty(), "missing mark for {source}");
        }
    }
}
