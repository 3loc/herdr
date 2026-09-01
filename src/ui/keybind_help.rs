use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::AppState;

pub(super) type HelpEntry = (String, Cow<'static, str>);
pub(super) type HelpGroup = (&'static str, Vec<HelpEntry>);

fn help_entry(key: impl Into<String>, label: &'static str) -> HelpEntry {
    (key.into(), Cow::Borrowed(label))
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_string())
}

fn indexed_label(bindings: &[crate::config::IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return "unset".to_string();
    }

    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }

    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[crate::config::IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

pub(super) fn keybind_help_groups(app: &AppState) -> Vec<HelpGroup> {
    let kb = &app.keybinds;
    let mut groups = Vec::new();

    groups.push((
        "global",
        vec![
            help_entry(
                crate::config::format_key_combo((app.prefix_code, app.prefix_mods)),
                "prefix mode",
            ),
            help_entry(keybind_label(&kb.help), "keybinds"),
            help_entry(keybind_label(&kb.settings), "settings"),
            help_entry(keybind_label(&kb.detach), "detach"),
            help_entry(keybind_label(&kb.reload_config), "reload config"),
            help_entry(
                keybind_label(&kb.open_notification_target),
                "open notification target",
            ),
        ],
    ));

    groups.push((
        "navigation",
        vec![
            help_entry("esc", "back"),
            help_entry(
                format!("{} / left", keybind_label(&kb.navigate.pane_left)),
                "enter sidebar",
            ),
            help_entry(
                format!(
                    "{} / {} / {} / {}",
                    keybind_label(&kb.navigate.pane_down),
                    keybind_label(&kb.navigate.pane_up),
                    keybind_label(&kb.navigate.workspace_down),
                    keybind_label(&kb.navigate.workspace_up)
                ),
                "select space or agent",
            ),
            help_entry(
                format!("{} / right / enter", keybind_label(&kb.navigate.pane_right)),
                "activate sidebar selection",
            ),
            help_entry("tab / shift+tab", "cycle pane"),
            help_entry("1..9", "switch workspace"),
        ],
    ));

    let workspace_tab = vec![
        help_entry(keybind_label(&kb.workspace_picker), "workspace navigation"),
        help_entry(keybind_label(&kb.goto), "session navigator"),
        help_entry(keybind_label(&kb.new_workspace), "new workspace"),
        help_entry(keybind_label(&kb.new_worktree), "new worktree"),
        help_entry(keybind_label(&kb.open_worktree), "open worktree"),
        help_entry(
            keybind_label(&kb.remove_worktree),
            "delete worktree checkout",
        ),
        help_entry(keybind_label(&kb.rename_workspace), "rename workspace"),
        help_entry(keybind_label(&kb.close_workspace), "close workspace"),
        help_entry(keybind_label(&kb.previous_workspace), "previous workspace"),
        help_entry(keybind_label(&kb.next_workspace), "next workspace"),
        help_entry(indexed_label(&kb.switch_workspace), "switch workspace 1-9"),
        help_entry(keybind_label(&kb.previous_agent), "previous agent"),
        help_entry(keybind_label(&kb.next_agent), "next agent"),
        help_entry(indexed_label(&kb.focus_agent), "focus agent 1-9"),
        help_entry(keybind_label(&kb.new_tab), "new tab"),
        help_entry(keybind_label(&kb.rename_tab), "rename tab"),
        help_entry(keybind_label(&kb.previous_tab), "previous tab"),
        help_entry(keybind_label(&kb.next_tab), "next tab"),
        help_entry(keybind_label(&kb.move_tab_previous), "move tab left"),
        help_entry(keybind_label(&kb.move_tab_next), "move tab right"),
        help_entry(indexed_label(&kb.switch_tab), "switch tab 1-9"),
        help_entry(keybind_label(&kb.close_tab), "close tab"),
    ];
    groups.push(("workspaces / tabs", workspace_tab));

    let panes = vec![
        help_entry(keybind_label(&kb.split_vertical), "split vertical"),
        help_entry(keybind_label(&kb.split_horizontal), "split horizontal"),
        help_entry(keybind_label(&kb.close_pane), "close pane"),
        help_entry(keybind_label(&kb.rename_pane), "rename pane"),
        help_entry(keybind_label(&kb.note), "note this pane"),
        help_entry(keybind_label(&kb.notes), "open pane notes"),
        help_entry(keybind_label(&kb.edit_scrollback), "edit scrollback"),
        help_entry(keybind_label(&kb.copy_mode), "copy mode"),
        help_entry(keybind_label(&kb.zoom), "zoom pane"),
        help_entry(keybind_label(&kb.resize_mode), "resize mode"),
        help_entry(keybind_label(&kb.resize_pane_left), "resize pane left"),
        help_entry(keybind_label(&kb.resize_pane_down), "resize pane down"),
        help_entry(keybind_label(&kb.resize_pane_up), "resize pane up"),
        help_entry(keybind_label(&kb.resize_pane_right), "resize pane right"),
        help_entry(keybind_label(&kb.toggle_sidebar), "toggle sidebar"),
        help_entry(keybind_label(&kb.focus_pane_left), "focus pane left"),
        help_entry(keybind_label(&kb.focus_pane_down), "focus pane down"),
        help_entry(keybind_label(&kb.focus_pane_up), "focus pane up"),
        help_entry(keybind_label(&kb.focus_pane_right), "focus pane right"),
        help_entry(keybind_label(&kb.cycle_pane_next), "cycle pane next"),
        help_entry(
            keybind_label(&kb.cycle_pane_previous),
            "cycle pane previous",
        ),
        help_entry(keybind_label(&kb.last_pane), "last pane"),
    ];
    groups.push(("panes", panes));

    if !kb.custom_commands.is_empty() {
        groups.push((
            "custom",
            kb.custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or(Cow::Borrowed("custom command")),
                    )
                })
                .collect(),
        ));
    }

    groups
}

fn filter_keybind_help_groups(groups: Vec<HelpGroup>, query: &str) -> Vec<HelpGroup> {
    if query.is_empty() {
        return groups;
    }

    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|(key, label)| {
                    key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

/// Width needed for the longest default binding and its description.
const MIN_COLUMN_WIDTH: u16 = 38;
const COLUMN_GAP: u16 = 3;
const MAX_COLUMNS: usize = 4;

pub(crate) fn keybind_help_column_count(width: u16) -> usize {
    let mut columns = 1usize;
    while columns < MAX_COLUMNS {
        let next = columns + 1;
        let needed = next as u16 * MIN_COLUMN_WIDTH + (next as u16 - 1) * COLUMN_GAP;
        if needed > width {
            break;
        }
        columns = next;
    }
    columns
}

fn group_block(
    app: &AppState,
    group: &str,
    entries: &[HelpEntry],
    key_width: usize,
) -> Vec<Line<'static>> {
    let heading_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(app.palette.text);

    let mut lines = vec![Line::from(Span::styled(format!(" {group}"), heading_style))];
    for (key, label) in entries {
        lines.push(Line::from(vec![
            Span::styled(format!(" {key:<key_width$} "), key_style),
            Span::styled(label.clone().into_owned(), label_style),
        ]));
    }
    lines.push(Line::raw(""));
    lines
}

pub(crate) fn keybind_help_columns(app: &AppState, width: u16) -> Vec<Vec<Line<'static>>> {
    let groups = filter_keybind_help_groups(keybind_help_groups(app), &app.keybind_help.query);
    if groups.is_empty() {
        return vec![vec![Line::from(Span::styled(
            " no matching keybinds",
            Style::default().fg(app.palette.overlay1),
        ))]];
    }

    let column_count = keybind_help_column_count(width).min(groups.len());
    let heights: Vec<usize> = groups.iter().map(|(_, e)| e.len() + 2).collect();
    let total: usize = heights.iter().sum();
    let target = total.div_ceil(column_count);

    let mut assigned: Vec<Vec<usize>> = vec![Vec::new(); column_count];
    let mut column = 0usize;
    let mut used = 0usize;
    for (index, height) in heights.iter().enumerate() {
        let remaining_columns = column_count - column;
        let remaining_groups = groups.len() - index;
        // Keep every column populated while balancing whole groups.
        if column + 1 < column_count
            && !assigned[column].is_empty()
            && (used + height > target || remaining_groups < remaining_columns)
        {
            column += 1;
            used = 0;
        }
        assigned[column].push(index);
        used += height;
    }

    assigned
        .into_iter()
        .map(|indices| {
            let key_width = indices
                .iter()
                .flat_map(|index| groups[*index].1.iter().map(|(key, _)| key.chars().count()))
                .max()
                .unwrap_or(8);
            indices
                .into_iter()
                .flat_map(|index| {
                    let (group, entries) = &groups[index];
                    group_block(app, group, entries, key_width)
                })
                .collect()
        })
        .collect()
}

pub(super) fn render_keybind_help_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell(frame, frame.area(), u16::MAX, u16::MAX, &app.palette)
    else {
        return;
    };
    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(frame, header_rows[0], "keybinds", &app.palette);
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        if app.keybind_help.search_focused {
            "back"
        } else {
            "close"
        },
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    let search_line = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.keybind_help.query.as_str(),
                Style::default()
                    .fg(app.palette.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " press / to filter by command or shortcut",
            Style::default().fg(app.palette.overlay0),
        ))
    };
    frame.render_widget(Paragraph::new(search_line), header_rows[1]);

    let body_area = stack.content;
    // Reserving the scrollbar column prevents filtering from reflowing columns.
    let content_width = body_area.width.saturating_sub(1);
    let columns = keybind_help_columns(app, content_width);
    let tallest = columns.iter().map(Vec::len).max().unwrap_or(0);

    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .keybind_help_max_scroll()
            .saturating_sub(app.keybind_help.scroll) as usize,
        max_offset_from_bottom: tallest.saturating_sub(body_area.height.max(1) as usize),
        viewport_rows: body_area.height.max(1) as usize,
    };

    let column_count = columns.len().max(1) as u32;
    let constraints = (0..column_count)
        .map(|_| Constraint::Ratio(1, column_count))
        .collect::<Vec<_>>();
    let column_areas = Layout::horizontal(constraints)
        .spacing(COLUMN_GAP)
        .split(Rect::new(
            body_area.x,
            body_area.y,
            content_width,
            body_area.height,
        ));

    for (lines, area) in columns.into_iter().zip(column_areas.iter()) {
        frame.render_widget(
            Paragraph::new(lines).scroll((app.keybind_help.scroll, 0)),
            *area,
        );
    }

    if let Some(track) = release_notes_scrollbar_rect(body_area, metrics) {
        render_scrollbar(
            frame,
            metrics,
            track,
            app.palette.overlay0,
            app.palette.overlay1,
            "▐",
        );
    }

    let footer = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(" filter ", Style::default().fg(app.palette.overlay0)),
            Span::styled("type/backspace", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("clear ", Style::default().fg(app.palette.overlay0)),
            Span::styled("ctrl+u", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("scroll ", Style::default().fg(app.palette.overlay0)),
            Span::styled("↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("back ", Style::default().fg(app.palette.overlay0)),
            Span::styled("esc", Style::default().fg(app.palette.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" search ", Style::default().fg(app.palette.overlay0)),
            Span::styled("/", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("scroll ", Style::default().fg(app.palette.overlay0)),
            Span::styled("j/k/↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled("close ", Style::default().fg(app.palette.overlay0)),
            Span::styled("esc/enter", Style::default().fg(app.palette.text)),
        ])
    };
    frame.render_widget(Paragraph::new(footer), stack.footer.unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<HelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![
                    help_entry("w", "workspace navigation"),
                    help_entry("c", "new tab"),
                ],
            ),
            (
                "panes",
                vec![
                    help_entry("v", "split vertical"),
                    help_entry("x", "close pane"),
                ],
            ),
        ]
    }

    #[test]
    fn column_count_grows_with_width_and_stops_at_four() {
        assert_eq!(keybind_help_column_count(0), 1);
        assert_eq!(keybind_help_column_count(MIN_COLUMN_WIDTH), 1);
        assert_eq!(
            keybind_help_column_count(2 * MIN_COLUMN_WIDTH + COLUMN_GAP - 1),
            1
        );
        assert_eq!(
            keybind_help_column_count(2 * MIN_COLUMN_WIDTH + COLUMN_GAP),
            2
        );
        assert_eq!(keybind_help_column_count(u16::MAX), MAX_COLUMNS);
    }

    #[test]
    fn columns_keep_every_entry_and_never_leave_one_empty() {
        let app = crate::app::state::AppState::test_new();

        for width in [40u16, 80, 120, 200, 400] {
            let columns = keybind_help_columns(&app, width);
            let expected = keybind_help_column_count(width).min(keybind_help_groups(&app).len());
            assert_eq!(columns.len(), expected, "column count at width {width}");
            assert!(
                columns.iter().all(|column| !column.is_empty()),
                "empty column at width {width}"
            );

            let rendered = columns
                .iter()
                .flatten()
                .flat_map(|line| line.spans.iter())
                .map(|span| span.content.as_ref())
                .collect::<Vec<_>>()
                .join("");
            for (group, _) in keybind_help_groups(&app) {
                assert!(
                    rendered.contains(group),
                    "group {group} missing at width {width}"
                );
            }
        }
    }

    #[test]
    fn filtered_to_nothing_still_renders_one_column() {
        let mut app = crate::app::state::AppState::test_new();
        app.keybind_help.query = "zzzz-no-such-binding".into();

        let columns = keybind_help_columns(&app, 200);
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].len(), 1);
    }

    #[test]
    fn keybind_help_filter_matches_labels_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "workspaces / tabs");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");
    }

    #[test]
    fn keybind_help_filter_matches_shortcuts_without_matching_group_headings() {
        let filtered = filter_keybind_help_groups(groups(), "x");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "panes");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");

        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }
}
