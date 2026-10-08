use std::sync::OnceLock;

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconStyle {
    Nerd,
    Plain,
    Herdr,
}

impl IconStyle {
    fn from_setting(setting: Option<&str>) -> Self {
        match setting {
            Some("herdr" | "herdr-agent-icons") => Self::Herdr,
            Some("plain" | "unicode" | "ascii" | "off" | "0") => Self::Plain,
            _ => Self::Nerd,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBrand {
    pub(crate) mark: &'static str,
    pub(crate) color: Color,
}

pub(crate) fn source_brand(source: &str) -> SourceBrand {
    static STYLE: OnceLock<IconStyle> = OnceLock::new();
    let style = *STYLE.get_or_init(|| {
        IconStyle::from_setting(std::env::var("RECALL_ICON_STYLE").ok().as_deref())
    });
    source_brand_with_style(source, style)
}

fn source_brand_with_style(source: &str, style: IconStyle) -> SourceBrand {
    let make = |nerd, fallback, r, g, b| brand(style, nerd, fallback, r, g, b);
    let mut selected = match source {
        // Keep Unicode fallbacks for brands whose Nerd Font glyph coverage is
        // inconsistent across commonly installed font versions.
        "claude-code" => make(None, "✳", 217, 119, 87),
        "codex" => make(None, "◎", 16, 163, 127),
        "opencode" => make(Some(""), "◇", 245, 166, 35),
        "pi" => make(None, "π", 190, 120, 255),
        "omp" => make(Some(""), "⌥", 255, 139, 61),
        "antigravity-cli" => make(Some(""), "△", 139, 92, 246),
        "gemini-cli" => make(Some(""), "✦", 66, 133, 244),
        "grok" => make(None, "𝕏", 210, 210, 210),
        "kiro-cli" => make(None, "◆", 152, 101, 245),
        "copilot-cli" => make(Some(""), "∞", 137, 87, 229),
        "copilot-chat" => make(Some(""), "◉", 94, 129, 172),
        "cursor" => make(None, "▰", 190, 190, 190),
        "cline" => make(Some(""), "◈", 238, 105, 80),
        "roo" => make(None, "◒", 62, 180, 137),
        "deepseek-harness" => make(None, "◫", 77, 107, 254),
        "kimi-code" => make(Some(""), "☾", 105, 102, 255),
        "qwen-code" => make(None, "Q", 99, 102, 241),
        "kilo-code" => make(None, "K", 0, 188, 212),
        "crush" => make(None, "C", 236, 72, 153),
        "amp" => make(None, "A", 180, 130, 240),
        "devin" => make(None, "D", 101, 169, 206),
        "qoder" => make(None, "Q", 124, 100, 238),
        "muse-code" => make(None, "M", 136, 178, 229),
        "mimo-code" => make(None, "M", 251, 146, 60),
        "zcode" => make(None, "Z", 72, 187, 120),
        "goose" => make(None, "G", 242, 184, 70),
        _ => make(None, "•", 120, 200, 120),
    };

    if style == IconStyle::Herdr {
        // Herdr icons require an explicitly configured terminal font. A font
        // installation alone does not guarantee that a terminal will select
        // its PUA glyphs instead of an unrelated CJK fallback.
        if let Some(mark) = herdr_logo(source) {
            selected.mark = mark;
        }
    }
    selected
}

fn brand(
    style: IconStyle,
    nerd_font_mark: Option<&'static str>,
    fallback_mark: &'static str,
    r: u8,
    g: u8,
    b: u8,
) -> SourceBrand {
    let mark = if style == IconStyle::Nerd {
        nerd_font_mark.unwrap_or(fallback_mark)
    } else {
        fallback_mark
    };
    SourceBrand { mark, color: Color::Rgb(r, g, b) }
}

// From hhdebb/herdr-radar lib/logos.js (HerdrAgentIconsMax-Regular.ttf).
// Only emit mapped vendor logos: a missing mark must use the plain Unicode
// fallback, never a PUA glyph that represents a different agent.
fn herdr_logo(source: &str) -> Option<&'static str> {
    Some(match source {
        "claude-code" => "\u{e1a0}",
        "codex" => "\u{e1a1}",
        "opencode" => "\u{e1a2}",
        "omp" => "\u{e1a3}",
        "cline" => "\u{e1a4}",
        "kimi-code" => "\u{e1a6}",
        "kilo-code" => "\u{e1a7}",
        "pi" => "\u{e1a9}",
        "cursor" => "\u{e1ab}",
        "copilot-cli" | "copilot-chat" => "\u{e1ac}",
        "deepseek-harness" => "\u{e1ad}",
        "gemini-cli" => "\u{e1ae}",
        "qwen-code" => "\u{e1b0}",
        "grok" => "\u{e1b1}",
        "antigravity-cli" => "\u{e1b2}",
        "kiro-cli" => "\u{e1b3}",
        "amp" => "\u{e1b4}",
        "devin" => "\u{e1b5}",
        "qoder" => "\u{e1b6}",
        "muse-code" => "\u{e1b9}",
        "crush" => "\u{e1ba}",
        _ => return None,
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
        "amp",
        "devin",
        "qoder",
        "muse-code",
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

    #[test]
    fn icon_style_preserves_existing_defaults_and_plain_aliases() {
        assert_eq!(IconStyle::from_setting(None), IconStyle::Nerd);
        assert_eq!(IconStyle::from_setting(Some("nerd")), IconStyle::Nerd);
        for alias in ["plain", "unicode", "ascii", "off", "0"] {
            assert_eq!(IconStyle::from_setting(Some(alias)), IconStyle::Plain);
        }
        for alias in ["herdr", "herdr-agent-icons"] {
            assert_eq!(IconStyle::from_setting(Some(alias)), IconStyle::Herdr);
        }
        assert_eq!(source_brand_with_style("opencode", IconStyle::Nerd).mark, "");
        assert_eq!(source_brand_with_style("opencode", IconStyle::Plain).mark, "◇");
    }

    #[test]
    fn herdr_agent_icons_match_the_upstream_private_use_mapping() {
        // Expected codepoints from hhdebb/herdr-radar lib/logos.js PUA table.
        let logos = [
            ("claude-code", 0xe1a0),
            ("codex", 0xe1a1),
            ("opencode", 0xe1a2),
            ("omp", 0xe1a3),
            ("cline", 0xe1a4),
            ("kimi-code", 0xe1a6),
            ("kilo-code", 0xe1a7),
            ("pi", 0xe1a9),
            ("cursor", 0xe1ab),
            ("copilot-cli", 0xe1ac),
            ("copilot-chat", 0xe1ac),
            ("deepseek-harness", 0xe1ad),
            ("gemini-cli", 0xe1ae),
            ("qwen-code", 0xe1b0),
            ("grok", 0xe1b1),
            ("antigravity-cli", 0xe1b2),
            ("kiro-cli", 0xe1b3),
            ("amp", 0xe1b4),
            ("devin", 0xe1b5),
            ("qoder", 0xe1b6),
            ("muse-code", 0xe1b9),
            ("crush", 0xe1ba),
        ];
        for (source, codepoint) in logos {
            let glyph = source_brand_with_style(source, IconStyle::Herdr);
            assert_eq!(glyph.mark.chars().count(), 1, "multiple characters for {source}");
            assert_eq!(
                glyph.mark.chars().next().unwrap() as u32,
                codepoint,
                "wrong logo for {source}"
            );
            assert_eq!(
                unicode_width::UnicodeWidthStr::width(glyph.mark),
                1,
                "wide glyph for {source}"
            );
            assert_eq!(
                glyph.color,
                source_brand_with_style(source, IconStyle::Plain).color,
                "brand color changed for {source}"
            );
        }
    }

    #[test]
    fn herdr_mode_keeps_unicode_fallbacks_for_unmapped_agents() {
        for source in ["roo", "mimo-code", "goose", "codebuddy", "warp", "unknown"] {
            let with_font = source_brand_with_style(source, IconStyle::Herdr);
            let without_font = source_brand_with_style(source, IconStyle::Plain);
            assert_eq!(with_font, without_font, "missing fallback for {source}");
        }
        assert_ne!(
            source_brand_with_style("claude-code", IconStyle::Herdr).mark,
            source_brand_with_style("claude-code", IconStyle::Plain).mark
        );
    }
}
