use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::session_delete::DeleteMode;
use crate::tui::app::App;
use crate::tui::search_state::{FilterFocus, PanelFocus, ProjectPickerRow, SourcePickerRow};
use crate::tui::share_state::PendingCommandAction;
use crate::tui::theme::THEME;

use super::truncate_start;

/// Truncate `s` to at most `max` terminal columns (CJK-aware), appending `…`.
fn truncate_to_width(s: &str, max: usize) -> String {
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

pub(super) fn render_subagents_picker(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = area.width.saturating_sub(4).clamp(48, 96).min(area.width);
    let count = app.viewing.children.len();
    let desired_height = count as u16 + 5;
    let height = desired_height.clamp(8, area.height.saturating_sub(2).max(8));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect::new(x, y, width, height);

    let block = Block::default()
        .title(format!(" Subagents ({count}) "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.accent))
        .style(Style::default().bg(THEME.popup_bg));

    // Inner width available for one row, minus the leading " > " marker.
    let inner = (width as usize).saturating_sub(2);
    let marker_w = 3;

    // Scroll the row window so the selected subagent stays visible. Rows share
    // the popup with the border (2), a top blank (1), and a blank + help (2).
    let visible_rows = (height as usize).saturating_sub(5).max(1);
    let start = if app.viewing.child_selected < visible_rows {
        0
    } else {
        app.viewing.child_selected + 1 - visible_rows
    };
    let end = (start + visible_rows).min(count);

    let mut lines = vec![Line::from("")];
    for (offset, child) in app.viewing.children[start..end].iter().enumerate() {
        let index = start + offset;
        let selected = index == app.viewing.child_selected;
        let marker = if selected { ">" } else { " " };
        let style = if selected {
            Style::default()
                .fg(THEME.selected_fg)
                .bg(THEME.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(THEME.text)
        };
        let meta = format!(
            "{} · {} · {} msgs",
            app.source_label_for(&child.source),
            crate::utils::format_age(child.started_at),
            child.message_count
        );
        let meta_w = UnicodeWidthStr::width(meta.as_str());
        let title_budget = inner.saturating_sub(marker_w + meta_w + 2);
        let title = truncate_to_width(&child.title, title_budget);
        let gap =
            inner.saturating_sub(marker_w + UnicodeWidthStr::width(title.as_str()) + meta_w).max(1);
        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} "), style),
            Span::styled(title, style),
            Span::styled(" ".repeat(gap), style),
            Span::styled(
                meta,
                if selected { style } else { Style::default().fg(THEME.text_muted) },
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" [Enter] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
        Span::styled("open  ", Style::default().fg(THEME.text)),
        Span::styled("[Esc] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
        Span::styled("close", Style::default().fg(THEME.text)),
    ]));

    let widget = Paragraph::new(lines).block(block);
    f.render_widget(Clear, rect);
    f.render_widget(widget, rect);
}

pub(super) fn render_handoff_target_picker(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = area.width.clamp(36, 56).min(area.width);
    let count = app.handoff_targets.len();
    let desired_height = count as u16 + 5;
    let max_height = area.height.max(1);
    let height = if max_height < 8 { max_height } else { desired_height.clamp(8, max_height) };
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect::new(x, y, width, height);

    let block = Block::default()
        .title(" Handoff target ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.accent))
        .style(Style::default().bg(THEME.popup_bg));

    let visible_rows = (height as usize).saturating_sub(5).max(1);
    let selected = app.handoff_target_selected.min(count.saturating_sub(1));
    let start = if count == 0 || selected < visible_rows { 0 } else { selected + 1 - visible_rows };
    let end = (start + visible_rows).min(count);

    let mut lines = vec![Line::from("")];
    for (offset, target) in app.handoff_targets[start..end].iter().enumerate() {
        let index = start + offset;
        let selected = index == app.handoff_target_selected;
        let marker = if selected { ">" } else { " " };
        let style = if selected {
            Style::default()
                .fg(THEME.selected_fg)
                .bg(THEME.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(THEME.text)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} "), style),
            Span::styled(target.label.as_str(), style),
            Span::styled(format!(" ({})", target.id), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" [Enter] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
        Span::styled("select  ", Style::default().fg(THEME.text)),
        Span::styled("[Esc] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
        Span::styled("cancel", Style::default().fg(THEME.text)),
    ]));

    let widget = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(Clear, rect);
    f.render_widget(widget, rect);
}

pub(super) fn render_filter_picker(f: &mut Frame, app: &App) {
    let project = app.filters.editing == Some(FilterFocus::Project);
    let picker = app.filters.picker();
    let area = f.area();
    let (width, title, hint, empty) = if project {
        (area.width.min(92), " Filters > Project ", "type to filter paths", " No matching projects")
    } else {
        (area.width.min(76), " Filters > Source ", "press / to filter", " No sources")
    };
    let rows: Vec<Option<String>> = if project {
        app.project_picker_rows()
            .iter()
            .map(|row| match *row {
                ProjectPickerRow::All => {
                    let marker = if app.filters.project.is_none() { "(*)" } else { "( )" };
                    Some(format!(" {marker} All projects"))
                }
                ProjectPickerRow::Project(index) => {
                    app.project_directories.get(index).map(|project| {
                        let marker =
                            if app.filters.project.as_deref() == Some(project.directory.as_str()) {
                                "(*)"
                            } else {
                                "( )"
                            };
                        let path =
                            truncate_start(&project.directory, width.saturating_sub(30) as usize);
                        format!(" {marker} {path}  {}", project.sessions)
                    })
                }
            })
            .collect()
    } else {
        app.source_picker_rows()
            .iter()
            .map(|row| match *row {
                SourcePickerRow::All => {
                    let marker = if app.filters.sources.is_empty() { "(*)" } else { "( )" };
                    Some(format!(" {marker} All enabled sources"))
                }
                SourcePickerRow::Source(index) => {
                    app.all_sources.get(index).map(|(source_id, label)| {
                        let marker =
                            if app.source_is_selected_in_picker(source_id) { "[x]" } else { "[ ]" };
                        format!(" {marker} {label} ({source_id})")
                    })
                }
            })
            .collect()
    };
    let desired_height = rows.len() as u16 + 7;
    let height = desired_height.clamp(8, area.height.saturating_sub(2).max(8));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_focus));
    let selected_style = Style::default().bg(THEME.selected_bg).fg(THEME.selected_fg);
    let normal_style = Style::default().fg(THEME.text);
    let muted_style = Style::default().fg(THEME.text_muted);
    let visible_rows = height.saturating_sub(7) as usize;
    let start = if visible_rows == 0 || picker.selected < visible_rows {
        0
    } else {
        picker.selected + 1 - visible_rows
    };
    let end = (start + visible_rows).min(rows.len());
    let filter_value = if picker.query.is_empty() && !picker.typing {
        Span::styled(hint, muted_style)
    } else {
        Span::styled(picker.query.clone(), normal_style)
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                " Filter: ",
                Style::default().fg(THEME.highlight).add_modifier(Modifier::BOLD),
            ),
            filter_value,
        ]),
        Line::from(""),
    ];
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(empty, muted_style)));
    } else {
        for (offset, row) in rows[start..end].iter().enumerate() {
            if let Some(text) = row {
                let style =
                    if start + offset == picker.selected { selected_style } else { normal_style };
                lines.push(Line::from(Span::styled(text.clone(), style)));
            }
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Space", Style::default().fg(THEME.accent)),
        Span::styled(" select/clear  ", muted_style),
        Span::styled("/", Style::default().fg(THEME.accent)),
        Span::styled(" filter  ", muted_style),
        Span::styled("Enter", Style::default().fg(THEME.accent)),
        Span::styled(" apply  ", muted_style),
        Span::styled("Esc", Style::default().fg(THEME.accent)),
        Span::styled(" back", muted_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+A", Style::default().fg(THEME.accent)),
        Span::styled(" all  ", muted_style),
        Span::styled("Ctrl+U", Style::default().fg(THEME.accent)),
        Span::styled(" clear input", muted_style),
    ]));
    f.render_widget(Clear, popup);
    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: false }), popup);
    if picker.typing {
        let cursor_x = popup.x + 9 + UnicodeWidthStr::width(&picker.query[..picker.cursor]) as u16;
        f.set_cursor_position((cursor_x.min(popup.right().saturating_sub(2)), popup.y + 1));
    }
}

pub(super) fn render_export_input(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_height = 3u16;
    let y = area.height.saturating_sub(popup_height + 1);
    let popup_area = Rect::new(area.x, y, area.width, popup_height);

    let block = Block::default()
        .title(" Export to (Enter confirm, Esc cancel) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.accent))
        .style(Style::default().bg(THEME.popup_bg));

    let input = Paragraph::new(app.export_path.as_str())
        .style(Style::default().fg(THEME.text).bg(THEME.popup_bg))
        .block(block);

    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);

    let cursor_x =
        popup_area.x + 1 + UnicodeWidthStr::width(&app.export_path[..app.export_cursor]) as u16;
    f.set_cursor_position((cursor_x.min(popup_area.right() - 2), y + 1));
}

pub(super) fn render_settings(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let block = Block::default()
        .title(" Settings (Enter/Space toggle, PgUp/PgDn shortcuts, Esc close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.accent))
        .style(Style::default().bg(THEME.popup_bg));
    let inner = block.inner(popup);
    f.render_widget(Clear, popup);
    f.render_widget(block, popup);
    if inner.width < 8 || inner.height < 3 {
        return;
    }
    let left_width = (inner.width / 2).saturating_sub(1);
    let left = Rect::new(
        inner.x + 1,
        inner.y + 1,
        left_width.saturating_sub(1),
        inner.height.saturating_sub(2),
    );
    let right = Rect::new(
        inner.x + left_width + 1,
        inner.y + 1,
        inner.width.saturating_sub(left_width + 2),
        inner.height.saturating_sub(2),
    );
    let header = Style::default().fg(THEME.highlight).add_modifier(Modifier::BOLD);
    let selected = Style::default().bg(THEME.selected_bg).fg(THEME.selected_fg);
    let normal = Style::default().fg(THEME.text);
    let mut settings = vec![
        Line::from(Span::styled("Time scope", header)),
        Line::from(Span::styled(
            format!("  {}", app.config.sync_window.label()),
            if app.settings_selected == 0 { selected } else { normal },
        )),
        Line::from(""),
        Line::from(Span::styled("Agents", header)),
    ];
    let longest = app.all_sources.iter().map(|(id, _)| id.len() + 8).max().unwrap_or(0);
    let columns = if usize::from(left.width) >= longest * 2 { 2 } else { 1 };
    let rows = app.all_sources.len().div_ceil(columns);
    let width = usize::from(left.width) / columns;
    for row in 0..rows {
        let mut spans = Vec::new();
        for col in 0..columns {
            let index = row + col * rows;
            if let Some((id, _)) = app.all_sources.get(index) {
                let brand = crate::tui::source_brand::source_brand(id);
                let name = id
                    .split('-')
                    .map(|word| match word {
                        "cli" => "CLI".to_string(),
                        _ => {
                            let mut chars = word.chars();
                            chars
                                .next()
                                .map(|c| c.to_uppercase().to_string() + chars.as_str())
                                .unwrap_or_default()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let prefix = if app.config.is_source_enabled(id) { "[x]" } else { "[ ]" };
                let chosen = app.settings_selected == index + 1;
                let style = if chosen { selected } else { normal };
                let text = format!(" {prefix} ");
                let name = truncate_to_width(&name, width.saturating_sub(8));
                let used = UnicodeWidthStr::width(text.as_str())
                    + UnicodeWidthStr::width(brand.mark)
                    + 1
                    + UnicodeWidthStr::width(name.as_str());
                spans.push(Span::styled(text, style));
                spans.push(Span::styled(brand.mark, style.fg(brand.color)));
                spans.push(Span::styled(
                    format!(" {name}{}", " ".repeat(width.saturating_sub(used))),
                    style,
                ));
            }
        }
        settings.push(Line::from(spans));
    }
    let visible = usize::from(left.height.saturating_sub(4));
    let selected_row = app.settings_selected.saturating_sub(1) % rows.max(1);
    let offset = selected_row.saturating_add(1).saturating_sub(visible);
    if offset > 0 {
        settings.drain(4..4 + offset.min(rows));
    }
    f.render_widget(Paragraph::new(settings), left);
    if right.width > 0 {
        let viewport = usize::from(right.height.saturating_sub(1));
        let content_len = shortcut_reference_lines(0).len();
        let has_scrollbar = viewport > 0 && content_len > viewport && right.width > 1;
        let content_width = right.width.saturating_sub(u16::from(has_scrollbar));
        let shortcuts = shortcut_reference_lines(usize::from(content_width));
        let max_scroll = shortcuts.len().saturating_sub(viewport);
        let offset = usize::from(app.settings_shortcut_scroll).min(max_scroll);
        let title = format!("Shortcuts  ↓ PgUp/PgDn ({})", offset + 1);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(title, header))),
            Rect::new(right.x, right.y, right.width, 1),
        );
        f.render_widget(
            Paragraph::new(shortcuts).scroll((offset as u16, 0)),
            Rect::new(right.x, right.y + 1, content_width, right.height.saturating_sub(1)),
        );
        if has_scrollbar {
            let scrollbar_area =
                Rect::new(right.x, right.y, right.width, right.height.saturating_add(1));
            super::render_vertical_scrollbar(f, scrollbar_area, content_len, viewport, offset);
        }
    }
}

const SHORTCUTS: &[(&str, &str, &str)] = &[
    ("Global", "Ctrl+C", "quit (except while deleting)"),
    ("Search", "↑/↓", "move"),
    ("Search", "PgUp/PgDn", "page"),
    ("Search", "←/→", "cursor/preview"),
    ("Search", "Backspace", "edit"),
    ("Search", "Enter", "detail"),
    ("Search", "Tab/Ctrl+F", "filters"),
    ("Search", "Alt+←/→", "cycle source tabs"),
    ("Search", "Ctrl+R", "resume"),
    ("Search", "Ctrl+O", "app"),
    ("Search", "Ctrl+S", "sync"),
    ("Search", "Ctrl+P", "settings"),
    ("Search", "Ctrl+E", "expand/collapse children"),
    ("Search", "Ins/Ctrl+Space", "select"),
    ("Search", "Space", "select (empty query)"),
    ("Search", "Del", "trash"),
    ("Search", "Ctrl+D", "purge"),
    ("Search", "Esc", "clear/back/quit"),
    ("Search", "q", "quit(empty query)"),
    ("Preview", "↑/↓/PgUp/PgDn", "scroll"),
    ("Preview", "←", "sessions"),
    ("Preview", "Enter", "detail"),
    ("Preview", "Esc", "back"),
    ("Viewer", "↑/↓/j/k", "scroll"),
    ("Viewer", "PgUp/PgDn", "scroll"),
    ("Viewer", "Home/g", "first"),
    ("Viewer", "End/G", "last"),
    ("Viewer", "a", "subagents"),
    ("Viewer", "c", "copy"),
    ("Viewer", "e", "export"),
    ("Viewer", "s", "share"),
    ("Viewer", "v", "preview"),
    ("Viewer", "h", "handoff"),
    ("Viewer", "/", "find"),
    ("Viewer", "n/N", "match"),
    ("Viewer", "Ctrl+R", "resume"),
    ("Viewer", "Ctrl+O", "app"),
    ("Viewer", "Del", "trash"),
    ("Viewer", "Ctrl+D", "purge"),
    ("Viewer", "Esc/q", "back"),
    ("Usage", "m", "mode"),
    ("Usage", "t", "time"),
    ("Usage", "s", "source"),
    ("Usage", "r", "reset"),
    ("Usage", "Enter", "open"),
    ("Usage", "j/k", "move"),
    ("Usage", "Esc/q", "back"),
    ("Filters", "↑/↓", "move"),
    ("Filters", "←/→", "change"),
    ("Filters", "Enter/Space", "edit"),
    ("Filters", "c", "clear"),
    ("Filters", "d/w/m/l", "time"),
    ("Filters", "r/n", "sort"),
    ("Pickers", "/", "filter"),
    ("Pickers", "Space", "toggle"),
    ("Pickers", "Enter", "apply"),
    ("Pickers", "Ctrl+A", "all"),
    ("Pickers", "Ctrl+U", "clear"),
    ("Pickers", "↑/↓", "move"),
    ("Pickers", "Backspace/←/→", "edit"),
    ("Pickers", "Home/End", "edit"),
    ("Pickers", "Esc", "back"),
    ("Settings", "↑/↓", "move"),
    ("Settings", "←/→/Enter/Space", "change"),
    ("Settings", "Esc/q", "close"),
    ("Subagents & handoff", "↑/↓/j/k", "move"),
    ("Subagents & handoff", "Enter/Space", "open/choose"),
    ("Subagents & handoff", "Esc/q", "close"),
    ("Confirm & share", "Y/Enter", "confirm"),
    ("Confirm & share", "N/Esc", "cancel"),
    ("Confirm & share", "O", "open"),
    ("Confirm & share", "C", "copy"),
    ("Confirm & share", "q", "close"),
    ("Export & find", "type", "enter text"),
    ("Export & find", "Backspace", "edit"),
    ("Export & find", "←/→", "move cursor"),
    ("Export & find", "Home/End", "find"),
    ("Export & find", "Enter", "apply/save"),
    ("Export & find", "Esc", "back"),
];

fn shortcut_reference_lines(width: usize) -> Vec<Line<'static>> {
    let key_width = SHORTCUTS.iter().map(|(_, key, _)| key.width()).max().unwrap_or(0).min(16);
    let mut lines = Vec::new();
    let mut previous = "";
    for &(section, key, description) in SHORTCUTS {
        if section != previous {
            if !previous.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                truncate_to_width(section, width),
                Style::default().fg(THEME.highlight).add_modifier(Modifier::BOLD),
            )));
            previous = section;
        }
        let indent = width.min(2);
        let available_key_width = key_width.min(width.saturating_sub(indent));
        let key = truncate_to_width(key, available_key_width);
        let remaining = width.saturating_sub(indent + available_key_width);
        let separator = usize::from(remaining > 0);
        let padding = available_key_width.saturating_sub(key.width()) + separator;
        let description_width = remaining.saturating_sub(separator);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(indent)),
            Span::styled(key, Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(padding)),
            Span::styled(
                truncate_to_width(description, description_width),
                Style::default().fg(THEME.text_muted),
            ),
        ]));
    }
    lines
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn shortcut_rows_preserve_entries_and_truncate_without_wrapping() {
        let lines = shortcut_reference_lines(80);
        for &(_, key, description) in SHORTCUTS {
            assert!(
                lines.iter().any(|line| {
                    line.spans.len() == 4
                        && line.spans[1].content == key
                        && line.spans[3].content == description
                }),
                "missing {key} {description}"
            );
        }
        let screenshot = shortcut_reference_lines(36);
        assert!(screenshot.iter().any(|line| line.spans.len() == 4
            && line.spans[1].content == "Ctrl+R"
            && line.spans[3].content == "resume"));
        assert!(screenshot.iter().any(|line| line.spans.len() == 4
            && line.spans[1].content == "Alt+←/→"
            && line.spans[3].content == "cycle source tabs"));
        let narrow = shortcut_reference_lines(25);
        assert!(
            narrow
                .iter()
                .any(|line| line.spans.last().is_some_and(|span| span.content.ends_with('…')))
        );
        for width in 0..=25 {
            assert!(shortcut_reference_lines(width).iter().all(|line| line.width() <= width));
        }
    }
}

pub(super) fn render_share_result(f: &mut Frame, app: &App) {
    let Some(popup) = app.share_popup.as_ref() else {
        return;
    };

    let area = f.area();
    let width = area.width.clamp(46, 88);
    let height: u16 = if popup.url.is_some() { 9 } else { 8 };
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect::new(x, y, width, height);

    let border = if popup.is_error { THEME.error } else { THEME.success };
    let title = if popup.is_error { " Share failed " } else { " Session shared " };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(THEME.popup_bg));

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            popup.message.clone(),
            if popup.is_error {
                Style::default().fg(THEME.error)
            } else {
                Style::default().fg(THEME.success).add_modifier(Modifier::BOLD)
            },
        )),
    ];
    if let Some(url) = popup.url.as_ref() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(url.clone(), Style::default().fg(THEME.highlight))));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" [O] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
            Span::styled("open   ", Style::default().fg(THEME.text)),
            Span::styled(" [C] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
            Span::styled("copy URL   ", Style::default().fg(THEME.text)),
            Span::styled(
                "[Enter/Esc] ",
                Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("close", Style::default().fg(THEME.text)),
        ]));
    } else if popup.is_error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                " [Enter/Esc] ",
                Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("close", Style::default().fg(THEME.text)),
        ]));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Publishing with Wrangler...",
            Style::default().fg(THEME.text_muted),
        )));
    }

    let widget = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(Clear, rect);
    f.render_widget(widget, rect);
}

pub(super) fn render_confirm_resume(f: &mut Frame, app: &App) {
    let Some(pending) = app.pending_resume.as_ref() else {
        return;
    };

    let area = f.area();
    let width = area.width.clamp(40, 76);
    let height: u16 = 9;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let block_title = match pending.action {
        PendingCommandAction::Resume => " Resume session ",
        PendingCommandAction::OpenApp => " Open in app ",
        PendingCommandAction::Handoff => " Handoff session ",
    };
    let confirm_label = match pending.action {
        PendingCommandAction::Resume => "confirm & exec     ",
        PendingCommandAction::OpenApp => "confirm & open     ",
        PendingCommandAction::Handoff => "confirm & handoff  ",
    };

    let block = Block::default()
        .title(block_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.accent))
        .style(Style::default().bg(THEME.popup_bg));

    let title: String = pending.session_title.chars().take(width as usize - 10).collect();
    let command_text: String =
        pending.command.display().chars().take(width as usize - 14).collect();
    let cwd_text: String = pending
        .cwd
        .as_deref()
        .unwrap_or("-")
        .chars()
        .rev()
        .take(width as usize - 10)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                match pending.action {
                    PendingCommandAction::Handoff => " Target:  ",
                    _ => " Source:  ",
                },
                Style::default().fg(THEME.text_muted),
            ),
            Span::styled(
                pending.source_label.clone(),
                Style::default().fg(THEME.source).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(title, Style::default().fg(THEME.text)),
        ]),
        Line::from(vec![
            Span::styled(" Cwd:     ", Style::default().fg(THEME.text_muted)),
            Span::styled(cwd_text, Style::default().fg(THEME.text)),
        ]),
        Line::from(vec![
            Span::styled(" Command: ", Style::default().fg(THEME.text_muted)),
            Span::styled(
                command_text,
                Style::default().fg(THEME.highlight).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Y] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
            Span::styled(confirm_label, Style::default().fg(THEME.text)),
            Span::styled("[N] ", Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD)),
            Span::styled("cancel", Style::default().fg(THEME.text)),
        ]),
    ];

    let widget = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(Clear, popup);
    f.render_widget(widget, popup);
}

pub(super) fn render_confirm_delete(f: &mut Frame, app: &App) {
    let Some(pending) = app.pending_delete.as_ref() else {
        return;
    };

    let area = f.area();
    let width = area.width.clamp(52, 88);
    let mut source_counts = std::collections::BTreeMap::<String, usize>::new();
    for session in &pending.sessions {
        *source_counts.entry(session.source.clone()).or_default() += 1;
    }
    let source_rows = source_counts.len().min(4) as u16;
    let height = (10 + source_rows).min(area.height.saturating_sub(2).max(10));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let (mode_label, mode_note, mode_color) = match pending.mode {
        DeleteMode::Trash => (
            "Move to trash",
            "Native data is moved/backed up when supported; unsupported sources require explicit index-only deletion.",
            THEME.success,
        ),
        DeleteMode::Permanent => (
            "Permanent delete",
            "Native session data is deleted without a recovery copy when supported.",
            THEME.error,
        ),
        DeleteMode::IndexOnly => {
            ("Recall only", "Only Recall index entries are removed.", THEME.info)
        }
    };
    let mode_note = if pending.mode != DeleteMode::IndexOnly && source_counts.contains_key("droid")
    {
        "Droid files are removed locally only; Factory cloud copies may remain."
    } else {
        mode_note
    };
    let deleting = app.delete_rx.is_some();
    let block = Block::default()
        .title(if pending.sessions.len() == 1 {
            " Confirm delete session "
        } else {
            " Confirm bulk delete "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mode_color))
        .style(Style::default().bg(THEME.popup_bg));

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Sessions: ", Style::default().fg(THEME.text_muted)),
            Span::styled(
                pending.sessions.len().to_string(),
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Action:   ", Style::default().fg(THEME.text_muted)),
            Span::styled(mode_label, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("           "),
            Span::styled(
                truncate_to_width(mode_note, width.saturating_sub(14) as usize),
                Style::default().fg(THEME.text_muted),
            ),
        ]),
        Line::from(""),
    ];

    for (source, count) in source_counts.iter().take(4) {
        let brand = crate::tui::source_brand::source_brand(source);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} {}", brand.mark, app.source_label_for(source)),
                Style::default().fg(brand.color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  ×{count}"), Style::default().fg(THEME.text_muted)),
        ]));
    }
    if source_counts.len() > 4 {
        lines.push(Line::from(Span::styled(
            format!("  +{} more sources", source_counts.len() - 4),
            Style::default().fg(THEME.text_muted),
        )));
    }
    lines.push(Line::from(""));

    if deleting {
        lines.push(Line::from(Span::styled(
            "  Deleting...",
            Style::default().fg(THEME.highlight).add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "  [Y/Enter] ",
                Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("confirm   ", Style::default().fg(THEME.text)),
            Span::styled(
                "[N/Esc] ",
                Style::default().fg(THEME.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("cancel", Style::default().fg(THEME.text)),
        ]));
    }

    let widget = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(Clear, popup);
    f.render_widget(widget, popup);
}

pub(super) fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let semantic_span = if app.semantic_progress.total_sessions > 0 {
        let mut text = format!(
            " [semantic {}/{}]",
            app.semantic_progress.done_sessions, app.semantic_progress.total_sessions
        );
        if app.semantic_progress.failed_sessions > 0 {
            text = format!(
                " [semantic {}/{}, {} failed]",
                app.semantic_progress.done_sessions,
                app.semantic_progress.total_sessions,
                app.semantic_progress.failed_sessions
            );
        }
        Some(Span::styled(text, Style::default().fg(THEME.info)))
    } else {
        None
    };

    let stats_span = Span::styled(
        format!(" [{} sessions, {} messages]", app.total_sessions, app.total_messages),
        Style::default().fg(THEME.text_muted),
    );

    let line = if app.sync_requested || app.sync_in_flight {
        let mut spans = vec![Span::styled(" Syncing...", Style::default().fg(THEME.info))];
        if let Some(span) = semantic_span.clone() {
            spans.push(span);
        }
        spans.push(stats_span);
        Line::from(spans)
    } else if let Some(ref msg) = app.status_message {
        let mut spans = vec![Span::styled(format!(" {msg}"), Style::default().fg(THEME.success))];
        if let Some(span) = semantic_span.clone() {
            spans.push(span);
        }
        spans.push(stats_span);
        Line::from(spans)
    } else {
        match app.panel_focus {
            PanelFocus::SessionList => {
                let mut spans = vec![
                    Span::styled(" ↑/↓", Style::default().fg(THEME.accent)),
                    Span::styled(" sessions  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("→", Style::default().fg(THEME.accent)),
                    Span::styled(" preview  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Enter", Style::default().fg(THEME.accent)),
                    Span::styled(" detail  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Ctrl+F", Style::default().fg(THEME.accent)),
                    Span::styled(" filter  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Ctrl+S", Style::default().fg(THEME.accent)),
                    Span::styled(" sync  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Ctrl+R", Style::default().fg(THEME.accent)),
                    Span::styled(" resume  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Ctrl+O", Style::default().fg(THEME.accent)),
                    Span::styled(" app  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Ctrl+P", Style::default().fg(THEME.accent)),
                    Span::styled(" settings  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Esc", Style::default().fg(THEME.accent)),
                    Span::styled(" clear  ", Style::default().fg(THEME.text_muted)),
                ];
                if app.query.is_empty() {
                    spans.push(Span::styled("q", Style::default().fg(THEME.accent)));
                    spans.push(Span::styled(" quit", Style::default().fg(THEME.text_muted)));
                }
                if let Some(span) = semantic_span.clone() {
                    spans.push(span);
                }
                spans.push(stats_span);
                Line::from(spans)
            }
            PanelFocus::Preview => {
                let mut spans = vec![
                    Span::styled(" ↑/↓", Style::default().fg(THEME.accent)),
                    Span::styled(" messages  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("←", Style::default().fg(THEME.accent)),
                    Span::styled(" sessions  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Enter", Style::default().fg(THEME.accent)),
                    Span::styled(" detail  ", Style::default().fg(THEME.text_muted)),
                    Span::styled("Esc", Style::default().fg(THEME.accent)),
                    Span::styled(" back", Style::default().fg(THEME.text_muted)),
                ];
                if let Some(span) = semantic_span {
                    spans.push(span);
                }
                spans.push(stats_span);
                Line::from(spans)
            }
        }
    };
    f.render_widget(Paragraph::new(line), area);
}
