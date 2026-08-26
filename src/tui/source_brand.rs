use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBrand {
    pub(crate) mark: &'static str,
    pub(crate) color: Color,
}

pub(crate) fn source_brand(source: &str) -> SourceBrand {
    match source {
        "claude-code" => brand("◆", 217, 119, 87),
        "codex" => brand("◎", 16, 163, 127),
        "opencode" => brand("◇", 245, 166, 35),
        "pi" => brand("π", 190, 120, 255),
        "oh-my-pi" => brand("⌥", 255, 139, 61),
        "antigravity-cli" => brand("△", 139, 92, 246),
        "gemini-cli" => brand("✦", 66, 133, 244),
        "grok" => brand("✕", 210, 210, 210),
        "kiro-cli" => brand("◆", 152, 101, 245),
        "copilot-cli" => brand("∞", 137, 87, 229),
        "cursor" => brand("▰", 190, 190, 190),
        "cline" => brand("◈", 238, 105, 80),
        "deepseek-harness" => brand("◫", 77, 107, 254),
        "kimi-code" => brand("☾", 105, 102, 255),
        _ => brand("•", 120, 200, 120),
    }
}

const fn brand(mark: &'static str, r: u8, g: u8, b: u8) -> SourceBrand {
    SourceBrand { mark, color: Color::Rgb(r, g, b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sources_have_distinct_brand_visuals() {
        assert_ne!(source_brand("claude-code"), source_brand("codex"));
        assert_ne!(source_brand("gemini-cli"), source_brand("deepseek-harness"));
    }
}
