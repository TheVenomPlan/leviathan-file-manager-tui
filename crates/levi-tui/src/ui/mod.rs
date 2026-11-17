//! Interfaz de Levi: paneles, barra de estado y overlays.

use levi_core::fs::{Entry, EntryKind, format_time_opt};
use levi_core::i18n::{self, Language};
use levi_core::keymap::SORTED_ACTIONS;
use levi_core::theme::{NamedColor, Theme, ThemeColor};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};

use crate::app::{App, TextPosition, TextSelection};
use crate::icons;

/// Dibuja la aplicación completa.
pub fn render(frame: &mut Frame, app: &mut App) {
    let theme = app.theme.clone();

    frame.render_widget(
        Block::default().style(Style::new().bg(color(theme.bg)).fg(color(theme.fg))),
        frame.area(),
    );

    let chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let (tabs_area, panels_area) = if app.tabs[app.active].search_active {
        // Barra de búsqueda en su propia fila, arriba de las pestañas.
        let top_chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(chunks[0]);
        render_search_bar(frame, app, &theme, top_chunks[0]);
        (top_chunks[1], top_chunks[2])
    } else {
        let top_chunks =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(chunks[0]);
        (top_chunks[0], top_chunks[1])
    };
    render_tabs(frame, app, &theme, tabs_area);

    let panels = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(40),
        Constraint::Percentage(35),
    ])
    .split(panels_area);

    render_parent(frame, app, &theme, panels[0]);
    render_current(frame, app, &theme, panels[1]);
    render_right(frame, app, &theme, panels[2]);
    render_status(frame, app, &theme, chunks[1]);
    render_hints(frame, app, &theme, chunks[2]);
    render_language_chip(frame, app, chunks[2]);

    if let Some(status) = &app.status {
        render_center_message(frame, &theme, status, frame.area());
    }

    if app.show_help {
        render_help(frame, app, &theme);
    }

    if app.show_prefs {
        render_prefs(frame, app, &theme);
    }

    if app.prompt.is_some() {
        render_prompt(frame, app, &theme);
    }

    if app.show_themes {
        render_themes(frame, app, &theme);
    }

    if app.show_languages {
        render_languages(frame, app, &theme);
    }

    render_text_selection(frame, app);
}

/// Pinta la selección de texto por encima de la interfaz.
fn render_text_selection(frame: &mut Frame, app: &App) {
    let Some(selection) = app.text_selection else {
        return;
    };
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    let (start, end) = selection.ordered();
    let style = Style::new()
        .fg(color(app.theme.selection_fg))
        .bg(color(app.theme.selection_bg));
    let left = area.left();
    let right = area.right().saturating_sub(1);
    let top = area.top();
    let bottom = area.bottom().saturating_sub(1);

    for y in start.y.max(top)..=end.y.min(bottom) {
        let x_start = if y == start.y {
            start.x.max(left)
        } else {
            left
        };
        let x_end = if y == end.y { end.x.min(right) } else { right };
        if x_start > x_end {
            continue;
        }
        let rect = Rect::new(x_start, y, x_end - x_start + 1, 1);
        frame.buffer_mut().set_style(rect, style);
    }
}

/// Extrae el texto de la selección a partir del buffer ya dibujado.
pub fn selection_text(buffer: &Buffer, selection: TextSelection) -> String {
    let area = buffer.area;
    let (start, end) = selection.ordered();
    let mut lines = Vec::new();
    let left = area.left();
    let right = area.right().saturating_sub(1);
    let top = area.top();
    let bottom = area.bottom().saturating_sub(1);

    for y in start.y.max(top)..=end.y.min(bottom) {
        let x_start = if y == start.y {
            start.x.max(left)
        } else {
            left
        };
        let x_end = if y == end.y { end.x.min(right) } else { right };
        if x_start > x_end {
            continue;
        }
        let mut line = String::new();
        for x in x_start..=x_end {
            let symbol = buffer[(x, y)].symbol();
            if !symbol.is_empty() {
                line.push_str(symbol);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

fn render_tabs(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let mut spans: Vec<Span> = vec![
        Span::styled(
            " Leví ",
            Style::new()
                .fg(color(theme.title))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::new().fg(color(theme.border))),
    ];
    for (i, tab) in app.tabs.iter().enumerate() {
        let name = tab
            .cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| tab.cwd.display().to_string());
        let label = format!(" {}:{} ", i + 1, name);
        let style = if i == app.active {
            Style::new()
                .fg(color(theme.selection_fg))
                .bg(color(theme.selection_bg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(color(theme.muted))
        };
        spans.push(Span::styled(label, style));
        if i + 1 < app.tabs.len() {
            spans.push(Span::styled("│", Style::new().fg(color(theme.border))));
        }
    }
    if app.tabs.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", app.strings().no_tabs),
            Style::new().fg(color(theme.muted)),
        ));
    }
    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::new().bg(color(theme.bg)));
    frame.render_widget(paragraph, area);
}

fn render_parent(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let items: Vec<ListItem> = app.tabs[app.active]
        .parent_entries
        .iter()
        .map(|entry| list_item(entry, app.config.icons, theme, false, false, false))
        .collect();

    let title = app.tabs[app.active]
        .cwd
        .parent()
        .map_or_else(|| "..".to_string(), |path| path.display().to_string());

    let list = List::new(items)
        .block(panel_block(theme, title))
        .style(Style::new().bg(color(theme.bg)))
        .highlight_symbol("▸ ")
        .highlight_style(selection_style(theme));

    frame.render_stateful_widget(list, area, &mut app.tabs[app.active].parent_state);
}

fn render_current(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let items: Vec<ListItem> = app.tabs[app.active]
        .entries
        .iter()
        .map(|entry| {
            let selected = app.tabs[app.active].selected.contains(&entry.path);
            let cut = app.clipboard_cut && app.clipboard.contains(&entry.path);
            let copied = !app.clipboard_cut && app.clipboard.contains(&entry.path);
            list_item(entry, app.config.icons, theme, selected, cut, copied)
        })
        .collect();

    let s = app.strings();
    let tab = &app.tabs[app.active];
    let title = if tab.search_global {
        i18n::fill(
            s.super_search_title,
            &[("count", &tab.entries.len().to_string())],
        )
    } else if tab.filter.trim().is_empty() {
        i18n::fill(
            s.entries_title,
            &[
                ("path", &tab.cwd.display().to_string()),
                ("count", &tab.entries.len().to_string()),
            ],
        )
    } else {
        i18n::fill(
            s.filtered_title,
            &[
                ("path", &tab.cwd.display().to_string()),
                ("shown", &tab.entries.len().to_string()),
                ("total", &tab.all_entries.len().to_string()),
                ("filter", &tab.filter),
            ],
        )
    };

    let list = List::new(items)
        .block(focused_panel_block(theme, title))
        .style(Style::new().bg(color(theme.bg)))
        .highlight_symbol("▸ ")
        .highlight_style(selection_style(theme));

    frame.render_stateful_widget(list, area, &mut app.tabs[app.active].list_state);
}

fn render_right(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let chunks =
        Layout::vertical([Constraint::Percentage(75), Constraint::Percentage(25)]).split(area);

    render_preview(frame, app, theme, chunks[0]);
    render_detail(frame, app, theme, chunks[1]);
}

/// Política de reescalado de las vistas previas con imagen.
///
/// `ratatui-image` ajusta la imagen al tamaño del panel y su filtro por defecto
/// es `Nearest` (vecino más próximo), que pixela el texto de los PDF al
/// reducir la página. Con `Lanczos3` el reducido conserva el detalle.
fn preview_resize() -> ratatui_image::Resize {
    ratatui_image::Resize::Fit(Some(ratatui_image::FilterType::Lanczos3))
}

fn render_preview(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let title = app
        .selected_entry()
        .map(|entry| i18n::fill(s.preview_of, &[("name", &entry.name)]))
        .unwrap_or_else(|| s.preview.to_string());

    if let Some(state) = app.tabs[app.active].preview_image.as_mut() {
        let block = panel_block(theme, title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let image = ratatui_image::StatefulImage::default().resize(preview_resize());
        frame.render_stateful_widget(image, inner, state);
        return;
    }

    let lines: Vec<Line> = app.tabs[app.active]
        .preview_lines
        .iter()
        .map(|l| Line::from(Span::raw(l.clone())))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(panel_block(theme, title))
        .style(Style::new().bg(color(theme.bg)).fg(color(theme.fg)))
        .wrap(Wrap { trim: false })
        .scroll((app.tabs[app.active].preview_scroll as u16, 0));

    frame.render_widget(paragraph, area);

    if app.tabs[app.active].preview_image.is_none()
        && !app.tabs[app.active].preview_lines.is_empty()
    {
        let mut scrollbar_state = ScrollbarState::new(
            app.tabs[app.active]
                .preview_lines
                .len()
                .saturating_sub(1)
                .max(1),
        )
        .position(app.tabs[app.active].preview_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

fn render_detail(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let s = app.strings();

    let selected = app.selected_entry().cloned();
    match selected {
        Some(entry) => {
            let size = app.detail_size_text(&entry);
            lines.push(field(s.field_name, &entry.name, theme));
            lines.push(field(
                s.field_type,
                entry.kind.label_in(app.language()),
                theme,
            ));
            lines.push(field(s.field_size, &size, theme));
            lines.push(field(
                s.field_modified,
                &format_time_opt(entry.modified),
                theme,
            ));
            lines.push(field(
                s.field_created,
                &format_time_opt(entry.created),
                theme,
            ));
            lines.push(field(s.field_permissions, &entry.permissions, theme));
            lines.push(Line::from(""));
            lines.push(field(
                s.field_path,
                &entry.path.display().to_string(),
                theme,
            ));
        }
        None => lines.push(Line::from(s.empty_dir).style(Style::new().fg(color(theme.muted)))),
    }

    let paragraph = Paragraph::new(lines)
        .block(panel_block(theme, s.detail.to_string()))
        .style(Style::new().bg(color(theme.bg)))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let tab = &app.tabs[app.active];
    let mut parts = vec![
        tab.cwd.display().to_string(),
        i18n::fill(
            s.status_entries,
            &[("count", &tab.entries.len().to_string())],
        ),
    ];
    if !tab.filter.is_empty() && !tab.search_active {
        parts.push(i18n::fill(s.status_filter, &[("filter", &tab.filter)]));
    }
    if !tab.selected.is_empty() {
        parts.push(i18n::fill(
            s.status_selected,
            &[("count", &tab.selected.len().to_string())],
        ));
    }
    if !app.clipboard.is_empty() {
        let kind = if app.clipboard_cut {
            s.clipboard_cut
        } else {
            s.clipboard_copied
        };
        parts.push(i18n::fill(
            s.status_clipboard,
            &[("count", &app.clipboard.len().to_string()), ("kind", kind)],
        ));
    }
    let hidden = if tab.show_hidden { s.yes } else { s.no };
    parts.push(i18n::fill(s.status_hidden, &[("value", hidden)]));
    parts.push(i18n::fill(s.status_theme, &[("name", &app.config.theme)]));
    parts.push(i18n::fill(s.status_icons, &[("name", icons_label(app))]));

    let text = format!(" {}", parts.join(" · "));
    let style = Style::new()
        .fg(color(theme.status_fg))
        .bg(color(theme.status_bg));
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_search_bar(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let global = app.tabs[app.active].search_global;
    let count = app.tabs[app.active].entries.len();
    let hint = if global {
        i18n::fill(
            s.search_global_hint,
            &[(
                "count",
                &app.tabs[app.active].global_results.len().to_string(),
            )],
        )
    } else {
        i18n::fill(
            s.search_hint,
            &[
                ("shown", &count.to_string()),
                ("total", &app.tabs[app.active].all_entries.len().to_string()),
            ],
        )
    };
    // Ambas barras de búsqueda comparten estilo; se distinguen por el texto.
    let (icon_style, text_style, hint_style, bar_style) = (
        Style::new()
            .fg(color(theme.title))
            .add_modifier(Modifier::BOLD),
        Style::new()
            .fg(color(theme.fg))
            .add_modifier(Modifier::BOLD),
        Style::new().fg(color(theme.muted)),
        Style::new()
            .fg(color(theme.status_fg))
            .bg(color(theme.status_bg)),
    );
    let line = Line::from(vec![
        Span::styled(" 🔎 ", icon_style),
        Span::styled(app.tabs[app.active].filter.clone(), text_style),
        Span::styled("  ", Style::new()),
        Span::styled(hint, hint_style),
    ]);
    let paragraph = Paragraph::new(line).style(bar_style);
    frame.render_widget(paragraph, area);

    // Muestra el cursor dentro del campo de búsqueda (ancho de visualización, no solo chars).
    let prefix_width = Span::raw(" 🔎 ").width() as u16;
    let before_cursor: String = app.tabs[app.active]
        .filter
        .chars()
        .take(app.tabs[app.active].filter_cursor)
        .collect();
    let cursor_width = Span::raw(before_cursor.as_str()).width() as u16;
    let x = (area.x + prefix_width + cursor_width).min(area.x + area.width.saturating_sub(1));
    frame.set_cursor_position((x, area.y));
}

fn render_center_message(
    frame: &mut Frame,
    theme: &Theme,
    status: &crate::app::Status,
    area: Rect,
) {
    let text = if status.error {
        format!(" ⚠ {} ", status.text)
    } else {
        format!(" {} ", status.text)
    };
    let width = (text.chars().count() as u16 + 4)
        .min(area.width.saturating_sub(4))
        .max(24);
    let height = 3u16;
    let popup = centered_rect_fixed(width, height, area);
    frame.render_widget(Clear, popup);
    let (fg, bg, border) = if status.error {
        (
            color(theme.error_fg),
            color(theme.error_bg),
            color(theme.error_fg),
        )
    } else {
        (
            color(theme.help_fg),
            color(theme.help_bg),
            color(theme.title),
        )
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(bg).fg(fg));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::new().fg(fg).bg(bg))
            .alignment(Alignment::Center),
        inner,
    );
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Etiqueta fija del botón de idioma: no se traduce en ningún idioma.
const LANGUAGE_CHIP_LABEL: &str = "Lenguaje";

/// Morado del botón de idioma.
const LANGUAGE_CHIP_BG: Color = Color::Rgb(0x6a, 0x1b, 0x9a);

/// Ancho total del selector de idioma.
const LANGUAGE_PICKER_WIDTH: u16 = 46;

/// Texto del botón «Lenguaje», con el idioma activo al lado.
fn language_chip_label(language: Language) -> String {
    format!(" {LANGUAGE_CHIP_LABEL}: {} ", language.display_name())
}

/// Ancho total del botón en celdas de la terminal.
fn language_chip_width(language: Language) -> u16 {
    Span::raw(language_chip_label(language)).width() as u16
}

/// Zona que ocupa el botón «Lenguaje»: última fila, esquina inferior derecha.
pub fn language_chip_area(area: Rect, language: Language) -> Rect {
    let width = language_chip_width(language).min(area.width);
    Rect {
        x: area.x + area.width.saturating_sub(width),
        y: area.y + area.height.saturating_sub(1),
        width,
        height: area.height.min(1),
    }
}

/// Dibuja el botón morado que abre el selector de idioma.
fn render_language_chip(frame: &mut Frame, app: &App, area: Rect) {
    let chip = language_chip_area(area, app.language());
    if chip.is_empty() {
        return;
    }
    let style = Style::new()
        .bg(LANGUAGE_CHIP_BG)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(language_chip_label(app.language()))
            .style(style)
            .alignment(Alignment::Center),
        chip,
    );
}

/// Zona que ocupa el selector de idioma (para dibujar y para los clics).
pub fn language_picker_area(area: Rect) -> Rect {
    let height = Language::ALL.len() as u16 + 4;
    centered_rect_fixed(LANGUAGE_PICKER_WIDTH, height, area)
}

/// Índice del idioma sobre el que cae la posición, si cae en una fila válida.
pub fn language_item_at(area: Rect, position: TextPosition) -> Option<usize> {
    let picker = language_picker_area(area);
    if picker.is_empty() {
        return None;
    }
    // Borde superior + línea de ayuda + línea en blanco antes de la lista.
    let first_row = picker.y.saturating_add(3);
    let last_row = picker.bottom().saturating_sub(1);
    if position.y < first_row || position.y >= last_row {
        return None;
    }
    if position.x <= picker.x || position.x >= picker.right().saturating_sub(1) {
        return None;
    }
    Some((position.y - first_row) as usize)
}

/// Dibuja el selector de idioma encima de la interfaz.
fn render_languages(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = language_picker_area(frame.area());
    let s = app.strings();
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        s.language_hint,
        Style::new().fg(color(theme.muted)),
    )));
    lines.push(Line::from(""));

    for (index, language) in Language::ALL.iter().enumerate() {
        let marker = if index == app.language_cursor {
            "> "
        } else {
            "  "
        };
        let current = if *language == app.config.language {
            " ●"
        } else {
            ""
        };
        let style = if index == app.language_cursor {
            Style::new()
                .fg(color(theme.selection_fg))
                .bg(color(theme.selection_bg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(color(theme.fg))
        };
        lines.push(Line::from(Span::styled(
            format!("{marker}{}{current}", language.display_name()),
            style,
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(Span::styled(
                    s.language,
                    Style::new()
                        .fg(color(theme.title))
                        .add_modifier(Modifier::BOLD),
                )))
                .border_style(Style::new().fg(color(theme.border))),
        )
        .style(Style::new().fg(color(theme.fg)).bg(color(theme.bg)));

    frame.render_widget(paragraph, area);
}

fn render_hints(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let line = if app.tabs[app.active].search_active {
        let enter_hint = if app.tabs[app.active].search_global {
            s.hint_copy
        } else {
            s.hint_accept
        };
        let hints = vec![
            (s.hint_search_active, s.hint_active),
            ("←/→", s.hint_cursor),
            ("↑/↓", s.hint_results),
            ("Intro", enter_hint),
            ("Esc", s.hint_close),
        ];
        hint_line(&hints, theme)
    } else {
        hint_line(
            &[
                ("j/k", s.hint_move),
                ("h/l", s.hint_parent_enter),
                ("space", s.hint_mark),
                ("ctrl+f", s.hint_search),
                ("/ f6", s.hint_super),
                ("o", s.hint_open),
                ("e", s.hint_edit),
                (".", s.hint_hidden),
                ("T", s.hint_themes),
                ("?", s.hint_help),
                (",", s.hint_prefs),
                ("q", s.hint_quit),
            ],
            theme,
        )
    };

    // Deja libre la esquina derecha para el botón de idioma; el fondo de la
    // barra llega hasta el borde del botón para que no quede un hueco.
    let text_area = Rect {
        width: area
            .width
            .saturating_sub(language_chip_width(app.language())),
        ..area
    };
    frame.render_widget(
        Paragraph::new(line).style(Style::new().bg(color(theme.status_bg))),
        text_area,
    );
}

/// Construye una línea de atajos con las teclas resaltadas.
fn hint_line(hints: &[(&str, &str)], theme: &Theme) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (index, (key, description)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", Style::new().fg(color(theme.border))));
        }
        spans.push(Span::styled(
            (*key).to_string(),
            Style::new()
                .fg(color(theme.title))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            (*description).to_string(),
            Style::new().fg(color(theme.muted)),
        ));
    }
    Line::from(spans)
}

fn render_help(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);
    let s = app.strings();

    let key = Style::new()
        .fg(color(theme.title))
        .add_modifier(Modifier::BOLD);

    let mut lines = vec![
        Line::from(Span::styled(" Leví File Manager TUI ", key)),
        Line::from(""),
    ];

    for action in SORTED_ACTIONS {
        if !action.implemented() {
            continue;
        }
        let keys = app.keymap.keys_for(action);
        if keys.is_empty() {
            continue;
        }
        let keys = keys
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" / ");
        lines.push(Line::from(vec![
            Span::styled(format!("  {keys:<18}"), key),
            Span::raw(action.label_in(app.language())),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        s.help_mouse,
        Style::new().fg(color(theme.muted)),
    )));
    lines.push(Line::from(Span::styled(
        s.help_config,
        Style::new().fg(color(theme.muted)),
    )));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(Span::styled(
                    s.help,
                    Style::new()
                        .fg(color(theme.title))
                        .add_modifier(Modifier::BOLD),
                )))
                .border_style(Style::new().fg(color(theme.border))),
        )
        .style(
            Style::new()
                .fg(color(theme.help_fg))
                .bg(color(theme.help_bg)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_prefs(frame: &mut Frame, app: &App, theme: &Theme) {
    // Ocupa toda la pantalla para que el fondo no distraiga.
    let area = frame.area();
    let s = app.strings();
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color(theme.title)))
        .style(Style::new().bg(color(theme.bg)).fg(color(theme.fg)))
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", s.prefs),
                Style::new()
                    .fg(color(theme.title))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(s.prefs_shortcuts, Style::new().fg(color(theme.muted))),
        ]));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);

    if app.prefs_capture {
        let msg = Paragraph::new(Line::from(Span::styled(
            s.prefs_capturing,
            Style::new()
                .fg(color(theme.error_fg))
                .bg(color(theme.error_bg)),
        )));
        frame.render_widget(msg, chunks[0]);
    } else {
        render_prefs_search_bar(frame, app, theme, chunks[0]);
    }

    render_prefs_list(frame, app, theme, chunks[1]);

    let footer_text = if app.prefs_search_active {
        s.prefs_footer_search
    } else {
        s.prefs_footer
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        footer_text,
        Style::new()
            .fg(color(theme.title))
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(footer, chunks[2]);
}

/// Barra de búsqueda de acciones de preferencias: filtra los atajos por
/// etiqueta, nombre de configuración o tecla asignada.
fn render_prefs_search_bar(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let visible = app.prefs_visible_actions().len();
    let total = SORTED_ACTIONS.len();
    let hint = if app.prefs_query.trim().is_empty() {
        i18n::fill(s.prefs_search_idle, &[("total", &total.to_string())])
    } else {
        let action = if app.prefs_search_active {
            s.prefs_action_cancel
        } else {
            s.prefs_action_clear
        };
        i18n::fill(
            s.prefs_search_filtered,
            &[
                ("visible", &visible.to_string()),
                ("total", &total.to_string()),
                ("action", action),
            ],
        )
    };
    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(
            " 🔎 ",
            Style::new()
                .fg(color(theme.title))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.prefs_query.clone(),
            Style::new()
                .fg(color(theme.fg))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::new()),
        Span::styled(hint, Style::new().fg(color(theme.muted))),
    ]))
    .style(
        Style::new()
            .fg(color(theme.status_fg))
            .bg(color(theme.status_bg)),
    );
    frame.render_widget(paragraph, area);

    // Cursor al final de la consulta mientras la barra tiene el foco.
    if app.prefs_search_active {
        let prefix_width = Span::raw(" 🔎 ").width() as u16;
        let cursor_width = Span::raw(app.prefs_query.as_str()).width() as u16;
        let x = (area.x + prefix_width + cursor_width).min(area.x + area.width.saturating_sub(1));
        frame.set_cursor_position((x, area.y));
    }
}

fn render_prefs_list(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let s = app.strings();
    let language = app.language();
    let heading = Style::new()
        .fg(color(theme.title))
        .add_modifier(Modifier::BOLD);
    let muted = Style::new().fg(color(theme.muted));

    // Muestra las acciones que coinciden con la búsqueda y sus teclas.
    let rows: Vec<(usize, _, String)> = SORTED_ACTIONS
        .iter()
        .enumerate()
        .filter(|(_, action)| app.prefs_action_matches(**action))
        .map(|(index, action)| {
            let keys = app
                .keymap
                .keys_for(*action)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let keys = if keys.is_empty() {
                s.prefs_no_key.to_string()
            } else {
                keys
            };
            (index, *action, keys)
        })
        .collect();

    if rows.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            i18n::fill(s.prefs_no_matches, &[("query", &app.prefs_query)]),
            Style::new().fg(color(theme.muted)),
        )))
        .style(
            Style::new()
                .bg(color(theme.help_bg))
                .fg(color(theme.help_fg)),
        );
        frame.render_widget(empty, area);
        return;
    }

    // Columnas dinámicas: así una etiqueta larga no empuja las teclas ni el estado.
    let label_width = rows
        .iter()
        .map(|(_, action, _)| action.label_in(language).chars().count())
        .max()
        .unwrap_or(0)
        .max(28)
        + 2;
    let keys_width = rows
        .iter()
        .map(|(_, _, keys)| keys.chars().count())
        .max()
        .unwrap_or(0)
        .max(22)
        + 2;

    let mut items: Vec<ListItem> = Vec::new();
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("  {:<label_width$}", s.prefs_header_action),
            heading,
        ),
        Span::styled(format!("{:<keys_width$}", s.prefs_header_keys), heading),
        Span::styled(s.prefs_header_status, heading),
    ])));
    items.push(ListItem::new(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::new().fg(color(theme.border)),
    ))));

    let selected = rows
        .iter()
        .position(|(index, _, _)| *index == app.prefs_key_cursor);

    for (index, action, keys) in &rows {
        let suffix = if action.implemented() {
            ""
        } else {
            s.prefs_next
        };
        let marker = if *index == app.prefs_key_cursor {
            "▸ "
        } else {
            "  "
        };
        let keys_style = if keys == s.prefs_no_key {
            muted
        } else {
            Style::new()
                .fg(color(theme.title))
                .add_modifier(Modifier::BOLD)
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{marker}{:<label_width$}", action.label_in(language)),
                Style::new().fg(color(theme.fg)),
            ),
            Span::styled(format!("{:<keys_width$}", keys), keys_style),
            Span::styled(suffix.to_string(), muted),
        ])));
    }

    let list = List::new(items)
        .style(
            Style::new()
                .fg(color(theme.help_fg))
                .bg(color(theme.help_bg)),
        )
        .highlight_symbol("")
        .highlight_style(selection_style(theme));

    let mut state = ratatui::widgets::ListState::default();
    state.select(selected.map(|index| index + 2));

    frame.render_stateful_widget(list, area, &mut state);
}

fn list_item(
    entry: &Entry,
    icons_mode: levi_core::config::IconsMode,
    theme: &Theme,
    selected: bool,
    cut: bool,
    copied: bool,
) -> ListItem<'static> {
    let mut style = entry_style(entry, theme);
    if cut {
        // Señal visual: los elementos cortados se atenúan.
        style = style.fg(color(theme.muted)).add_modifier(Modifier::DIM);
    }
    let icon = icons::icon(entry, icons_mode);
    let name = if entry.is_dir() {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    };

    let marker = if selected {
        Span::styled(
            "● ",
            Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };

    let mut spans = Vec::with_capacity(6);
    spans.push(marker);
    if !icon.is_empty() {
        spans.push(Span::styled(format!("{icon} "), style));
    }
    spans.push(Span::styled(name, style));
    if copied {
        spans.push(Span::styled("  ⎘", Style::new().fg(color(theme.title))));
    }
    if cut {
        spans.push(Span::styled("  ✂", Style::new().fg(color(theme.muted))));
    }

    ListItem::new(Line::from(spans))
}

fn entry_style(entry: &Entry, theme: &Theme) -> Style {
    if entry.hidden {
        return Style::new().fg(color(theme.hidden));
    }
    match entry.kind {
        EntryKind::Dir => Style::new()
            .fg(color(theme.dir))
            .add_modifier(Modifier::BOLD),
        EntryKind::Symlink => Style::new().fg(color(theme.symlink)),
        _ => Style::new().fg(color(theme.file)),
    }
}

fn selection_style(theme: &Theme) -> Style {
    Style::new()
        .fg(color(theme.selection_fg))
        .bg(color(theme.selection_bg))
        .add_modifier(Modifier::BOLD)
}

fn panel_block(theme: &Theme, title: String) -> Block<'static> {
    panel_block_with_border(theme, title, theme.border)
}

/// Panel con el borde resaltado (el panel activo).
fn focused_panel_block(theme: &Theme, title: String) -> Block<'static> {
    panel_block_with_border(theme, title, theme.title)
}

fn panel_block_with_border(theme: &Theme, title: String, border: ThemeColor) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color(border)))
        .style(Style::new().bg(color(theme.bg)).fg(color(theme.fg)))
        .title(Line::from(Span::styled(
            title,
            Style::new()
                .fg(color(theme.title))
                .add_modifier(Modifier::BOLD),
        )))
}

fn field(label: &'static str, value: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<11}"), Style::new().fg(color(theme.muted))),
        Span::styled(value.to_string(), Style::new().fg(color(theme.fg))),
    ])
}

fn icons_label(app: &App) -> &'static str {
    match app.config.icons {
        levi_core::config::IconsMode::Nerd => "nerd",
        levi_core::config::IconsMode::Emoji => "emoji",
        levi_core::config::IconsMode::Ascii => "ascii",
        levi_core::config::IconsMode::None => "none",
    }
}

fn color(value: ThemeColor) -> Color {
    match value {
        ThemeColor::Reset => Color::Reset,
        ThemeColor::Indexed(index) => Color::Indexed(index),
        ThemeColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        ThemeColor::Named(named) => match named {
            NamedColor::Black => Color::Black,
            NamedColor::Red => Color::Red,
            NamedColor::Green => Color::Green,
            NamedColor::Yellow => Color::Yellow,
            NamedColor::Blue => Color::Blue,
            NamedColor::Magenta => Color::Magenta,
            NamedColor::Cyan => Color::Cyan,
            NamedColor::Gray => Color::Gray,
            NamedColor::DarkGray => Color::DarkGray,
            NamedColor::White => Color::White,
            NamedColor::LightRed => Color::LightRed,
            NamedColor::LightGreen => Color::LightGreen,
            NamedColor::LightYellow => Color::LightYellow,
            NamedColor::LightBlue => Color::LightBlue,
            NamedColor::LightMagenta => Color::LightMagenta,
            NamedColor::LightCyan => Color::LightCyan,
        },
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

fn render_prompt(frame: &mut Frame, app: &App, theme: &Theme) {
    let prompt = app.prompt.as_ref().expect("prompt activo");
    let s = app.strings();
    let confirm = matches!(
        prompt.kind,
        crate::app::PromptKind::ConfirmTrash | crate::app::PromptKind::ConfirmDeletePermanent
    );

    // La barra Shell ocupa el ancho completo en la parte inferior.
    let area = if prompt.kind == crate::app::PromptKind::Shell {
        let full = frame.area();
        let height = full.height.min(3);
        Rect {
            x: full.x,
            y: full.y + full.height.saturating_sub(height),
            width: full.width,
            height,
        }
    } else if confirm {
        // Las confirmaciones necesitan espacio para listar los elementos.
        centered_rect(70, 60, frame.area())
    } else {
        centered_rect(50, 20, frame.area())
    };
    frame.render_widget(Clear, area);

    let count = prompt.items.len();
    let noun = if count == 1 {
        s.noun_item
    } else {
        s.noun_items
    };
    let title = match prompt.kind {
        crate::app::PromptKind::Create => s.prompt_create.to_string(),
        crate::app::PromptKind::Rename => s.prompt_rename.to_string(),
        crate::app::PromptKind::Shell => s.prompt_shell.to_string(),
        crate::app::PromptKind::ConfirmTrash => i18n::fill(
            s.prompt_confirm_trash,
            &[("count", &count.to_string()), ("noun", noun)],
        ),
        crate::app::PromptKind::ConfirmDeletePermanent => i18n::fill(
            s.prompt_confirm_delete,
            &[("count", &count.to_string()), ("noun", noun)],
        ),
    };

    let lines: Vec<Line> = if confirm {
        let mut lines = Vec::new();
        // Espacio interior menos una línea para el aviso final.
        let max_items = (area.height.saturating_sub(3) as usize).max(1);
        let shown = count.min(max_items);
        for name in prompt.items.iter().take(shown) {
            lines.push(Line::from(format!(" • {name}")));
        }
        if count > shown {
            lines.push(Line::from(Span::styled(
                i18n::fill(s.prompt_more, &[("count", &(count - shown).to_string())]),
                Style::new().fg(color(theme.muted)),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            s.prompt_confirm_hint,
            Style::new().fg(color(theme.muted)),
        )));
        lines
    } else {
        vec![Line::from(format!(" {}", prompt.input))]
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(Span::styled(
                    title,
                    Style::new()
                        .fg(color(theme.title))
                        .add_modifier(Modifier::BOLD),
                )))
                .border_style(Style::new().fg(color(theme.border))),
        )
        .style(Style::new().fg(color(theme.fg)).bg(color(theme.bg)));

    frame.render_widget(paragraph, area);

    // El texto se dibuja con un espacio inicial (" {input}"), así que el
    // cursor debe ir una celda más a la derecha para quedar tras la letra.
    if !confirm {
        let before: String = prompt.input.chars().take(prompt.cursor).collect();
        let cursor_w = Span::raw(before.as_str()).width() as u16;
        let cursor_x = area.x + 2 + cursor_w;
        let cursor_x = cursor_x.min(area.x + area.width.saturating_sub(2));
        frame.set_cursor_position((cursor_x, area.y + 1));
    }
}

fn render_themes(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 60, frame.area());
    let s = app.strings();
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        s.themes_hint,
        Style::new().fg(color(theme.muted)),
    )));
    lines.push(Line::from(""));

    for (i, name) in app.theme_list.iter().enumerate() {
        let marker = if i == app.theme_cursor { "> " } else { "  " };
        let style = if i == app.theme_cursor {
            Style::new()
                .fg(color(theme.selection_fg))
                .bg(color(theme.selection_bg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(color(theme.fg))
        };
        lines.push(Line::from(Span::styled(format!("{marker}{name}"), style)));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(Span::styled(
                    s.themes,
                    Style::new()
                        .fg(color(theme.title))
                        .add_modifier(Modifier::BOLD),
                )))
                .border_style(Style::new().fg(color(theme.border))),
        )
        .style(Style::new().fg(color(theme.fg)).bg(color(theme.bg)));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use levi_core::i18n::Language;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn buffer_text(app: &mut App) -> String {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal de prueba");
        terminal
            .draw(|frame| render(frame, app))
            .expect("dibujar la interfaz");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_three_panels_and_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("archivo.txt"), "hola").expect("archivo");
        std::fs::create_dir(dir.path().join("carpeta")).expect("carpeta");

        let mut app = App::new(dir.path().to_path_buf());
        let text = buffer_text(&mut app);

        assert!(text.contains("carpeta"), "debe mostrarse la carpeta");
        assert!(text.contains("archivo.txt"), "debe mostrarse el archivo");
        assert!(text.contains("Detalle"), "debe existir el panel de detalle");
    }

    #[test]
    fn help_overlay_renders() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = App::new(dir.path().to_path_buf());
        app.show_help = true;
        let text = buffer_text(&mut app);
        assert!(text.contains("Ayuda"), "debe mostrarse el overlay de ayuda");
    }

    #[test]
    fn preferences_overlay_lists_settings_and_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = App::new(dir.path().to_path_buf());
        app.show_prefs = true;
        app.prefs_scroll = 10;
        let text = buffer_text(&mut app);
        assert!(text.contains("Preferencias"), "debe mostrarse el overlay");
        assert!(
            text.contains("TECLAS"),
            "debe mostrarse la cabecera de teclas"
        );
        assert!(
            text.contains("ESTADO"),
            "debe mostrarse la cabecera de estado"
        );
        assert!(text.contains("copiar"), "debe aparecer la acción de copiar");
    }

    #[test]
    fn preferences_search_bar_filters_the_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());
        app.show_prefs = true;

        // Sin consulta la barra está arriba y la lista muestra todo.
        let text = buffer_text(&mut app);
        assert!(text.contains('🔎'), "la barra de búsqueda está arriba");
        assert!(
            text.contains("buscar acción o tecla"),
            "la barra invita a buscar con «/»"
        );
        assert!(
            text.contains("copiar"),
            "sin filtro se ven todas las acciones"
        );

        // Con consulta, la lista se reduce a las coincidencias.
        app.start_prefs_search();
        for character in "salir".chars() {
            app.push_prefs_query_char(character);
        }
        let text = buffer_text(&mut app);
        assert!(
            text.contains("salir"),
            "la consulta y su acción se muestran"
        );
        assert!(
            !text.contains("copiar"),
            "las acciones que no coinciden desaparecen"
        );

        // Sin coincidencias se avisa en la propia lista.
        app.prefs_query.clear();
        app.prefs_query.push_str("zzz");
        let text = buffer_text(&mut app);
        assert!(
            text.contains("Sin coincidencias"),
            "debe avisarse de que no hay coincidencias"
        );
    }

    #[test]
    fn mouse_selection_is_painted_over_the_interface() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = App::new(dir.path().to_path_buf());
        app.text_selection = Some(TextSelection {
            anchor: crate::app::TextPosition { x: 1, y: 1 },
            cursor: crate::app::TextPosition { x: 5, y: 1 },
        });

        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("dibujar la interfaz");

        let cell = &terminal.backend().buffer()[(3, 1)];
        assert_eq!(cell.bg, color(app.theme.selection_bg));
        assert_eq!(cell.fg, color(app.theme.selection_fg));
    }

    #[test]
    fn selection_text_extracts_multiline_and_reverse_selections() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 3));
        buffer.set_string(0, 0, "hola mundo", Style::default());
        buffer.set_string(0, 1, "segunda", Style::default());

        let forward = TextSelection {
            anchor: crate::app::TextPosition { x: 0, y: 0 },
            cursor: crate::app::TextPosition { x: 5, y: 1 },
        };
        assert_eq!(selection_text(&buffer, forward), "hola mundo\nsegund");

        let reverse = TextSelection {
            anchor: crate::app::TextPosition { x: 5, y: 1 },
            cursor: crate::app::TextPosition { x: 0, y: 0 },
        };
        assert_eq!(selection_text(&buffer, reverse), "hola mundo\nsegund");
    }

    #[test]
    fn search_prompt_and_selection_marker_are_rendered() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("mi_archivo.txt"), "hola").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        let path = app.tabs[app.active].entries[0].path.clone();
        app.tabs[app.active].selected.insert(path);
        app.start_search();
        app.push_search_char('m');

        let text = buffer_text(&mut app);
        assert!(text.contains("🔎"), "debe mostrarse la barra de búsqueda");
        assert!(
            text.contains('m') && text.contains("Buscar"),
            "debe mostrarse el filtro activo"
        );
        assert!(
            text.contains("Esc cerrar · Intro aceptar"),
            "debe indicarse cómo salir de la búsqueda"
        );
        assert!(
            text.contains("búsqueda activa"),
            "la barra inferior debe reflejar la búsqueda activa"
        );
        assert!(text.contains('●'), "debe marcarse la entrada seleccionada");
        assert!(
            text.contains("mi_archivo.txt"),
            "el filtro debe encontrar el archivo"
        );
    }

    #[test]
    fn language_chip_is_purple_and_keeps_its_name_in_english() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = App::new(dir.path().to_path_buf());
        app.config.language = Language::En;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("dibujar la interfaz");

        let area = Rect::new(0, 0, 100, 30);
        let chip = language_chip_area(area, Language::En);
        assert_eq!(chip.y, area.bottom() - 1, "el botón va en la última fila");
        assert_eq!(
            chip.x + chip.width,
            area.right(),
            "el botón está pegado a la derecha"
        );
        assert!(
            chip.width >= language_chip_width(Language::En),
            "el botón reserva sitio para el idioma activo"
        );
        let cell = &terminal.backend().buffer()[(chip.x + 1, chip.y)];
        assert_eq!(cell.bg, LANGUAGE_CHIP_BG, "el botón debe ser morado");

        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            text.contains("Lenguaje"),
            "el botón conserva su nombre en inglés"
        );
        assert!(
            text.contains("Lenguaje: English"),
            "el botón muestra el idioma activo"
        );
        assert!(
            text.contains("Details"),
            "los paneles deben estar en inglés"
        );
        assert!(
            text.contains("Preview"),
            "la vista previa debe estar en inglés"
        );
    }

    #[test]
    fn language_picker_lists_every_language_with_the_active_one_marked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = App::new(dir.path().to_path_buf());
        app.open_languages();
        app.language_cursor = 1;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("dibujar la interfaz");

        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("Idioma"), "el selector tiene título");
        assert!(
            text.contains("Español") && text.contains("English"),
            "el selector lista todos los idiomas"
        );
        assert!(text.contains("Español ●"), "el idioma activo queda marcado");

        let area = Rect::new(0, 0, 100, 30);
        let picker = language_picker_area(area);
        assert_eq!(
            language_item_at(area, TextPosition { x: 50, y: picker.y }),
            None
        );
        assert_eq!(
            language_item_at(
                area,
                TextPosition {
                    x: 50,
                    y: picker.y + 3
                }
            ),
            Some(0)
        );
        assert_eq!(
            language_item_at(
                area,
                TextPosition {
                    x: 50,
                    y: picker.y + 4
                }
            ),
            Some(1)
        );
        assert_eq!(
            language_item_at(
                area,
                TextPosition {
                    x: 50,
                    y: picker.y + 5
                }
            ),
            None,
            "la fila del borde inferior no es un idioma"
        );
    }

    #[test]
    fn preview_images_resample_with_lanczos() {
        // La vista previa de PDF pierde calidad si el reescalado al panel usa
        // el filtro por defecto (`Nearest`); debe usar remuestreo de alta
        // calidad para que el texto no se pixele.
        match preview_resize() {
            ratatui_image::Resize::Fit(Some(ratatui_image::FilterType::Lanczos3)) => {}
            otra => panic!("la vista previa debe reescalar con Lanczos3, no {otra:?}"),
        }
    }
}
