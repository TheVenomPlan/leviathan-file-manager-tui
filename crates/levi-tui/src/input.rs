//! Traducción de eventos de teclado a acciones de la aplicación.
//!
//! Las teclas no están fijas: se consultan en el [`Keymap`] cargado desde
//! `config.toml`, así que cualquier acción se puede remapear.

use crossterm::event::{
    KeyCode as TermCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use levi_core::i18n::Language;
use levi_core::keymap::{Action, Key, KeyCode};
use ratatui::layout::Rect;

use crate::app::{App, Request, TextPosition, TextSelection};

/// Procesa una pulsación de tecla (ya filtrada por `KeyEventKind::Press`/`Repeat`).
pub fn handle_key(app: &mut App, event: KeyEvent) {
    // Los mensajes transitorios se descartan con la siguiente tecla.
    if app.status.as_ref().is_some_and(|s| !s.error)
        && app.prompt.is_none()
        && !app.tabs[app.active].search_active
        && !app.show_themes
        && !app.show_languages
        && !(app.show_prefs && (app.prefs_capture || app.prefs_search_active))
    {
        app.status = None;
    }

    // Prompt de texto (crear/renombrar).
    if app.prompt.is_some() {
        handle_prompt_key(app, event);
        return;
    }

    // Selector de idioma: se dibuja encima de los demás overlays.
    if app.show_languages {
        handle_languages_key(app, event);
        return;
    }

    // Overlay de temas.
    if app.show_themes {
        handle_themes_key(app, event);
        return;
    }

    // Preferencias: la captura de teclas y los atajos propios (`s`, `r`, `t`, Intro)
    // tienen prioridad sobre el keymap, para que funcionen aunque la tecla no
    // esté asignada a ninguna acción.
    if app.show_prefs {
        if app.prefs_capture {
            if event.code == TermCode::Esc {
                app.prefs_capture = false;
                app.set_message(app.strings().capture_cancelled.to_string());
                return;
            }
            if let Some(key) = to_key(event) {
                // Esc no se puede asignar como atajo.
                if matches!(key.code, levi_core::keymap::KeyCode::Esc) {
                    app.prefs_capture = false;
                    app.set_message(app.strings().esc_not_assignable.to_string());
                    return;
                }
                let actions = levi_core::keymap::SORTED_ACTIONS;
                if let Some(action) = actions.get(app.prefs_key_cursor).copied() {
                    app.keymap.rebind(action, key);
                    app.prefs_capture = false;
                    let text = levi_core::i18n::fill(
                        app.strings().shortcut_rebound,
                        &[
                            ("action", action.label_in(app.language())),
                            ("key", &key.to_string()),
                        ],
                    );
                    app.set_message(text);
                }
            }
            return;
        }

        // La barra de búsqueda de acciones captura todo lo que se escribe.
        if app.prefs_search_active {
            handle_prefs_search_key(app, event);
            return;
        }

        match event.code {
            TermCode::Char('s') | TermCode::Char('S') => {
                app.save_config();
                return;
            }
            TermCode::Char('r') | TermCode::Char('R') => {
                app.reset_keymap_to_defaults();
                return;
            }
            TermCode::Char('t') | TermCode::Char('T') => {
                app.show_prefs = false;
                app.open_themes();
                return;
            }
            // «/» abre la barra de búsqueda de acciones, como en el panel.
            TermCode::Char('/') => {
                app.start_prefs_search();
                return;
            }
            // Con un filtro aplicado, Esc lo limpia antes de cerrar el overlay.
            TermCode::Esc if !app.prefs_query.is_empty() => {
                app.cancel_prefs_search();
                return;
            }
            TermCode::Enter => {
                if app.prefs_visible_actions().is_empty() {
                    let text = levi_core::i18n::fill(
                        app.strings().no_matching_actions,
                        &[("query", &app.prefs_query)],
                    );
                    app.set_message(text);
                    return;
                }
                normalize_prefs_cursor(app);
                app.prefs_capture = true;
                app.set_message(app.strings().press_new_key.to_string());
                return;
            }
            _ => {}
        }
    }

    // Acceso directo a temas (mayúscula) fuera de preferencias.
    if !app.show_prefs
        && !app.tabs[app.active].search_active
        && !event.modifiers.contains(KeyModifiers::CONTROL)
        && !event.modifiers.contains(KeyModifiers::ALT)
        && let TermCode::Char('T') = event.code
    {
        app.open_themes();
        return;
    }

    // Fallback directo para superbúsqueda (Ctrl+Shift+F / F6) en distintos terminales.
    // Se acepta Ctrl+'F' mayúscula aunque el modificador Shift no se reporte de forma fiable.
    if !app.show_prefs && !app.show_help && !app.show_themes && !app.tabs[app.active].search_active
    {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        let is_f = matches!(event.code, TermCode::Char('f') | TermCode::Char('F'));
        let upper_f = matches!(event.code, TermCode::Char('F'));
        if event.code == TermCode::F(6) || (ctrl && is_f && (shift || upper_f)) {
            app.start_global_search();
            return;
        }
    }

    // Mientras se busca, las teclas escriben en el filtro en lugar de navegar.
    if app.tabs[app.active].search_active {
        handle_search_key(app, event);
        return;
    }

    let Some(key) = to_key(event) else {
        return;
    };

    // Desplazamiento de la vista previa / páginas PDF: Ctrl+flechas y PageUp/PageDown.
    // En preferencias no aplica: esas teclas pertenecen al overlay.
    if !app.show_prefs && event.modifiers.contains(KeyModifiers::CONTROL) {
        match event.code {
            TermCode::Up => {
                app.preview_pdf_prev_page();
                return;
            }
            TermCode::Down => {
                app.preview_pdf_next_page();
                return;
            }
            TermCode::PageUp => {
                if app.is_pdf_preview() {
                    app.preview_pdf_prev_page();
                } else {
                    app.preview_scroll_up(10);
                }
                return;
            }
            TermCode::PageDown => {
                if app.is_pdf_preview() {
                    app.preview_pdf_next_page();
                } else {
                    app.preview_scroll_down(10);
                }
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::PageUp if !app.show_prefs => {
            app.preview_scroll_up(5);
            return;
        }
        KeyCode::PageDown if !app.show_prefs => {
            app.preview_scroll_down(5);
            return;
        }
        _ => {}
    }

    let Some(action) = app.keymap.action(&key) else {
        return;
    };

    // Los overlays atrapan la entrada: primero se cierran y luego navegan.
    if app.show_help {
        if matches!(
            action,
            Action::Help | Action::Quit | Action::ClearStatus | Action::Preferences
        ) {
            app.show_help = false;
        } else if action == Action::ToggleLanguage {
            app.open_languages();
        }
        return;
    }

    if app.show_prefs {
        match action {
            Action::Preferences | Action::Help | Action::ClearStatus | Action::Quit => {
                app.show_prefs = false;
            }
            Action::ToggleLanguage => app.open_languages(),
            Action::CursorDown => move_prefs_cursor(app, true),
            Action::CursorUp => move_prefs_cursor(app, false),
            Action::GoTop => set_prefs_cursor(app, true),
            Action::GoBottom => set_prefs_cursor(app, false),
            Action::Edit if !app.config_path.as_os_str().is_empty() => {
                app.show_prefs = false;
                app.request = Some(Request::Edit(app.config_path.clone()));
            }
            _ => {}
        }
        return;
    }

    match action {
        Action::Quit => app.should_quit = true,
        Action::Help => {
            app.show_help = true;
            app.show_prefs = false;
        }
        Action::Preferences => {
            app.show_prefs = true;
            app.show_help = false;
            app.reset_prefs_search();
            normalize_prefs_cursor(app);
        }
        Action::CursorDown => app.move_cursor(1),
        Action::CursorUp => app.move_cursor(-1),
        Action::GoTop => app.go_top(),
        Action::GoBottom => app.go_bottom(),
        Action::GoParent => app.go_parent(),
        Action::Enter => app.enter(),
        Action::Open => app.open_selected(),
        Action::Reveal => app.reveal_selected(),
        Action::Edit => app.edit_selected(),
        Action::ToggleHidden => app.toggle_hidden(),
        Action::Refresh => app.refresh(),
        Action::ClearStatus => app.status = None,
        Action::Filter => app.start_search(),
        Action::GlobalSearch => app.start_global_search(),
        Action::Shell => app.start_shell(),
        Action::Locations => app.show_locations(),
        Action::GoTrash => app.go_trash(),
        Action::ToggleSelect => app.toggle_select(),
        Action::SelectAll => app.select_all(),
        Action::ClearSelection => app.clear_selection(),
        Action::Copy => app.copy_selected(),
        Action::Cut => app.cut_selected(),
        Action::Paste => app.paste(),
        Action::Trash => app.trash_selected(),
        Action::DeletePermanent => app.delete_permanently_selected(),
        Action::NextTab => app.next_tab(),
        Action::PrevTab => app.prev_tab(),
        Action::NewTab => app.new_tab(),
        Action::CloseTab => app.close_tab(),
        Action::Create => app.start_create(),
        Action::Rename => app.start_rename(),
        Action::ToggleLanguage => app.open_languages(),
        other => {
            let text = levi_core::i18n::fill(
                app.strings().not_available,
                &[("action", other.label_in(app.language()))],
            );
            app.set_message(text);
        }
    }
}

/// Mueve el cursor de preferencias.
fn move_prefs_cursor(app: &mut App, down: bool) {
    let visible = app.prefs_visible_actions();
    let Some(current) = visible
        .iter()
        .position(|index| *index == app.prefs_key_cursor)
    else {
        if let Some(first) = visible.first() {
            app.prefs_key_cursor = *first;
            app.prefs_scroll = app.prefs_key_cursor;
        }
        return;
    };
    let next = if down {
        (current + 1).min(visible.len().saturating_sub(1))
    } else {
        current.saturating_sub(1)
    };
    app.prefs_key_cursor = visible[next];
    app.prefs_scroll = app.prefs_key_cursor;
}

/// Coloca el cursor en la primera o última acción visible.
fn set_prefs_cursor(app: &mut App, top: bool) {
    let visible = app.prefs_visible_actions();
    let index = if top { visible.first() } else { visible.last() };
    if let Some(index) = index {
        app.prefs_key_cursor = *index;
        app.prefs_scroll = app.prefs_key_cursor;
    }
}

/// Si el cursor quedó fuera de rango, lo ajusta.
fn normalize_prefs_cursor(app: &mut App) {
    let visible = app.prefs_visible_actions();
    if !visible.contains(&app.prefs_key_cursor)
        && let Some(first) = visible.first()
    {
        app.prefs_key_cursor = *first;
    }
    app.prefs_scroll = app.prefs_key_cursor;
}

/// Procesa eventos del ratón para seleccionar texto y copiarlo al soltar.
pub fn handle_mouse(app: &mut App, event: MouseEvent, area: Rect) {
    if area.is_empty() {
        return;
    }
    let position = TextPosition {
        x: event
            .column
            .clamp(area.left(), area.right().saturating_sub(1)),
        y: event.row.clamp(area.top(), area.bottom().saturating_sub(1)),
    };

    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // El selector de idioma captura el clic: una fila aplica el idioma
            // y un clic fuera lo cierra.
            if app.show_languages {
                let picker = crate::ui::language_picker_area(area);
                if rect_contains(picker, position) {
                    if let Some(index) = crate::ui::language_item_at(area, position)
                        && let Some(language) = Language::ALL.get(index).copied()
                    {
                        app.apply_language(language);
                    }
                } else {
                    app.close_languages();
                }
                return;
            }

            // El botón «Lenguaje» ocupa la esquina inferior derecha: abre el
            // selector en vez de iniciar una selección de texto.
            if in_language_chip(app, area, position) {
                if app.prompt.is_none() {
                    app.text_selection = None;
                    app.copy_selection = false;
                    app.open_languages();
                }
                return;
            }
            app.text_selection = Some(TextSelection {
                anchor: position,
                cursor: position,
            });
            app.copy_selection = false;
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.text_selection.is_none() && in_language_chip(app, area, position) {
                return;
            }
            if let Some(selection) = app.text_selection.as_mut() {
                selection.cursor = position;
            } else {
                app.text_selection = Some(TextSelection {
                    anchor: position,
                    cursor: position,
                });
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some(selection) = app.text_selection {
                if selection.is_empty() {
                    app.text_selection = None;
                } else {
                    app.copy_selection = true;
                }
            }
        }
        MouseEventKind::ScrollDown => app.move_cursor(1),
        MouseEventKind::ScrollUp => app.move_cursor(-1),
        _ => {}
    }
}

/// Indica si la posición está dentro del botón «Lenguaje».
fn in_language_chip(app: &App, area: Rect, position: TextPosition) -> bool {
    let chip = crate::ui::language_chip_area(area, app.language());
    chip.width > 0
        && chip.height > 0
        && position.x >= chip.x
        && position.x < chip.x.saturating_add(chip.width)
        && position.y >= chip.y
        && position.y < chip.y.saturating_add(chip.height)
}

/// Indica si la posición cae dentro del rectángulo.
fn rect_contains(rect: Rect, position: TextPosition) -> bool {
    rect.width > 0
        && rect.height > 0
        && position.x >= rect.x
        && position.x < rect.x.saturating_add(rect.width)
        && position.y >= rect.y
        && position.y < rect.y.saturating_add(rect.height)
}

fn handle_search_key(app: &mut App, event: KeyEvent) {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let alt = event.modifiers.contains(KeyModifiers::ALT);

    match event.code {
        // Las barras de búsqueda se cierran con Esc (cancelar) o con Intro
        // (aceptar; en la super búsqueda copia la ruta seleccionada).
        TermCode::Esc => app.cancel_search(),
        TermCode::Enter if app.tabs[app.active].search_global => {
            // Sin consulta no hay nada que copiar: cerrar sin ruido.
            if app.tabs[app.active].filter.trim().is_empty() {
                app.cancel_search();
            } else {
                app.copy_selected();
                app.cancel_search();
            }
        }
        TermCode::Enter => app.accept_search(),
        TermCode::Backspace => app.pop_search_char(),
        TermCode::Delete => app.delete_search_char(),
        TermCode::Left => app.move_search_cursor_left(),
        TermCode::Right => app.move_search_cursor_right(),
        TermCode::Home => app.move_search_cursor_home(),
        TermCode::End => app.move_search_cursor_end(),
        // Navegación por los resultados sin salir de la búsqueda.
        TermCode::Up => app.move_cursor(-1),
        TermCode::Down => app.move_cursor(1),
        TermCode::PageUp => app.move_cursor(-10),
        TermCode::PageDown => app.move_cursor(10),
        // La tecla que abre la barra («/», F6, Ctrl+F…) no cierra: mientras
        // está abierta no hace nada. «/» tampoco se escribe, porque ningún
        // nombre de archivo puede contenerla.
        TermCode::Char('/') => {}
        TermCode::Char(character) if !ctrl && !alt => app.push_search_char(character),
        _ => {}
    }
}

/// Teclas mientras la barra de búsqueda de acciones de preferencias tiene
/// el foco: filtra la lista de atajos sin salir del overlay.
fn handle_prefs_search_key(app: &mut App, event: KeyEvent) {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let alt = event.modifiers.contains(KeyModifiers::ALT);

    match event.code {
        // Esc cancela el filtro; Intro lo deja aplicado y devuelve el foco
        // a la lista para editar la tecla de la acción seleccionada.
        TermCode::Esc => app.cancel_prefs_search(),
        TermCode::Enter => app.confirm_prefs_search(),
        TermCode::Backspace => app.pop_prefs_query_char(),
        // Navegación por las coincidencias sin salir de la búsqueda.
        TermCode::Up => move_prefs_cursor(app, false),
        TermCode::Down => move_prefs_cursor(app, true),
        TermCode::PageUp => set_prefs_cursor(app, true),
        TermCode::PageDown => set_prefs_cursor(app, false),
        // La tecla que abre la barra no hace nada con ella abierta, y «/»
        // tampoco se escribe (igual que las barras del panel).
        TermCode::Char('/') => {}
        TermCode::Char(character) if !ctrl && !alt => app.push_prefs_query_char(character),
        _ => {}
    }
}

/// Convierte un evento de `crossterm` en una tecla del núcleo.
fn to_key(event: KeyEvent) -> Option<Key> {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let alt = event.modifiers.contains(KeyModifiers::ALT);
    let shift = event.modifiers.contains(KeyModifiers::SHIFT);

    let code = match event.code {
        TermCode::Char(character) => {
            // Normaliza letras con Ctrl para que coincidan con el keymap
            // (p. ej. Ctrl+Shift+F llega como 'f'/'F' según terminal).
            let character = if ctrl && character.is_ascii_alphabetic() {
                if shift || character.is_ascii_uppercase() {
                    character.to_ascii_uppercase()
                } else {
                    character.to_ascii_lowercase()
                }
            } else {
                character
            };
            KeyCode::Char(character)
        }
        // Algunos terminales envían Ctrl+Espacio como NUL (0x00).
        TermCode::Null if ctrl => KeyCode::Char(' '),
        TermCode::Enter => KeyCode::Enter,
        TermCode::Esc => KeyCode::Esc,
        TermCode::Tab => KeyCode::Tab,
        TermCode::BackTab => KeyCode::Tab,
        TermCode::Backspace => KeyCode::Backspace,
        TermCode::Delete => KeyCode::Delete,
        TermCode::Insert => KeyCode::Insert,
        TermCode::Up => KeyCode::Up,
        TermCode::Down => KeyCode::Down,
        TermCode::Left => KeyCode::Left,
        TermCode::Right => KeyCode::Right,
        TermCode::Home => KeyCode::Home,
        TermCode::End => KeyCode::End,
        TermCode::PageUp => KeyCode::PageUp,
        TermCode::PageDown => KeyCode::PageDown,
        TermCode::F(number) => KeyCode::F(number),
        _ => return None,
    };

    let mut shift = if matches!(event.code, TermCode::BackTab) {
        true
    } else {
        shift
    };
    // Para caracteres alfabéticos, el caso ya representa Shift.
    if let KeyCode::Char(c) = code
        && c.is_ascii_alphabetic()
    {
        shift = false;
    }
    Some(Key::with_all_mods(code, ctrl, alt, shift))
}

/// Gestiona las teclas mientras hay un prompt de texto activo.
fn handle_prompt_key(app: &mut App, event: KeyEvent) {
    let confirm = app.prompt.as_ref().is_some_and(|p| {
        matches!(
            p.kind,
            crate::app::PromptKind::ConfirmTrash | crate::app::PromptKind::ConfirmDeletePermanent
        )
    });

    match event.code {
        TermCode::Esc => app.cancel_prompt(),
        TermCode::Enter => app.confirm_prompt(),
        TermCode::Backspace => app.pop_prompt_char(),
        TermCode::Delete => app.delete_prompt_char(),
        TermCode::Left => app.move_prompt_cursor_left(),
        TermCode::Right => app.move_prompt_cursor_right(),
        TermCode::Home => app.move_prompt_cursor_home(),
        TermCode::End => app.move_prompt_cursor_end(),
        TermCode::Char(c)
            if !confirm
                && !event.modifiers.contains(KeyModifiers::CONTROL)
                && !event.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.push_prompt_char(c)
        }
        _ => {}
    }
}

/// Gestiona las teclas del overlay de temas.
fn handle_themes_key(app: &mut App, event: KeyEvent) {
    match event.code {
        TermCode::Esc | TermCode::Char('q') => app.close_themes(),
        TermCode::Up | TermCode::Char('k') => {
            if app.theme_cursor > 0 {
                app.theme_cursor -= 1;
            }
        }
        TermCode::Down | TermCode::Char('j') => {
            if app.theme_cursor + 1 < app.theme_list.len() {
                app.theme_cursor += 1;
            }
        }
        TermCode::Enter => {
            if let Some(name) = app.theme_list.get(app.theme_cursor).cloned() {
                app.apply_theme(&name);
            }
        }
        TermCode::Char('s') => app.save_config(),
        _ => {}
    }
}

/// Gestiona las teclas del selector de idioma.
fn handle_languages_key(app: &mut App, event: KeyEvent) {
    match event.code {
        TermCode::Esc | TermCode::Char('q') | TermCode::Char('l') | TermCode::Char('L') => {
            app.close_languages();
        }
        TermCode::Up | TermCode::Char('k') => {
            app.language_cursor = app.language_cursor.saturating_sub(1);
        }
        TermCode::Down | TermCode::Char('j') => {
            if app.language_cursor + 1 < Language::ALL.len() {
                app.language_cursor += 1;
            }
        }
        TermCode::Home | TermCode::Char('g') => app.language_cursor = 0,
        TermCode::End | TermCode::Char('G') => {
            app.language_cursor = Language::ALL.len().saturating_sub(1);
        }
        TermCode::Enter => {
            if let Some(language) = Language::ALL.get(app.language_cursor).copied() {
                app.apply_language(language);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;
    use levi_core::config::Config;
    use levi_core::i18n::Language;
    use levi_core::keymap::Keymap;
    use levi_core::theme::Theme;
    use std::path::PathBuf;

    fn app() -> App {
        App::with_config(
            PathBuf::from("."),
            Config::default(),
            Theme::default(),
            Keymap::defaults(),
        )
    }

    #[test]
    fn mouse_drag_marks_selection_for_copy() {
        let mut app = app();
        let area = Rect::new(0, 0, 80, 24);
        let event = |kind, column, row| MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };

        handle_mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), 2, 3),
            area,
        );
        handle_mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), 10, 3),
            area,
        );
        assert!(app.text_selection.is_some());
        assert!(!app.copy_selection);

        handle_mouse(
            &mut app,
            event(MouseEventKind::Up(MouseButton::Left), 10, 3),
            area,
        );
        assert!(app.copy_selection);
        assert_eq!(app.text_selection.expect("selección").cursor.x, 10);
    }

    #[test]
    fn mouse_coordinates_are_clamped_to_the_terminal() {
        let mut app = app();
        let area = Rect::new(0, 0, 20, 10);
        let event = |kind, column, row| MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };
        // Empieza lejos del botón de idioma para no alternarlo al pulsar.
        handle_mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), 1, 1),
            area,
        );
        handle_mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), 999, 999),
            area,
        );
        let selection = app.text_selection.expect("selección");
        assert_eq!(selection.cursor.x, 19);
        assert_eq!(selection.cursor.y, 9);
    }

    #[test]
    fn language_chip_click_opens_the_picker_and_chooses_a_language() {
        let mut app = app();
        let area = Rect::new(0, 0, 80, 24);
        let chip = crate::ui::language_chip_area(area, app.language());
        let click = |column, row| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };

        handle_mouse(&mut app, click(chip.x + 1, chip.y), area);
        assert!(app.show_languages, "el clic abre el selector de idioma");
        assert_eq!(app.language_cursor, 0, "el cursor parte del idioma activo");
        assert!(
            app.text_selection.is_none(),
            "el clic del botón no debe iniciar una selección"
        );

        // Segunda fila del selector (borde + ayuda + blanco + índice 1).
        let picker = crate::ui::language_picker_area(area);
        handle_mouse(&mut app, click(picker.x + 4, picker.y + 4), area);
        assert_eq!(app.language(), Language::En);
        assert!(!app.show_languages, "aplicar un idioma cierra el selector");
    }

    #[test]
    fn language_picker_click_outside_closes_without_changes() {
        let mut app = app();
        let area = Rect::new(0, 0, 80, 24);
        let click = |column, row| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };

        app.open_languages();
        handle_mouse(&mut app, click(2, 2), area);
        assert!(!app.show_languages);
        assert_eq!(app.language(), Language::Es, "cerrar no cambia el idioma");
    }

    #[test]
    fn language_key_opens_the_picker_and_enter_applies_the_choice() {
        let mut app = app();
        handle_key(&mut app, press(TermCode::Char('L'), KeyModifiers::NONE));
        assert!(app.show_languages);
        assert_eq!(app.language_cursor, 0);
        assert_eq!(app.language(), Language::Es, "abrir no cambia el idioma");

        handle_key(&mut app, press(TermCode::Down, KeyModifiers::NONE));
        assert_eq!(app.language_cursor, 1);

        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.language(), Language::En);
        assert!(!app.show_languages);
        assert_eq!(app.strings().detail, "Details");

        // Esc cierra el selector sin tocar el idioma activo.
        handle_key(&mut app, press(TermCode::Char('L'), KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.show_languages);
        assert_eq!(app.language(), Language::En);
    }

    fn press(code: TermCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new_with_kind(code, modifiers, KeyEventKind::Press)
    }

    #[test]
    fn maps_navigation_keys() {
        let mut app = app();
        app.tabs[app.active].entries = Vec::new();
        handle_key(&mut app, press(TermCode::Char('?'), KeyModifiers::NONE));
        assert!(app.show_help);

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.show_help);
    }

    #[test]
    fn comma_opens_preferences() {
        let mut app = app();
        handle_key(&mut app, press(TermCode::Char(','), KeyModifiers::NONE));
        assert!(app.show_prefs);
        handle_key(&mut app, press(TermCode::Char(','), KeyModifiers::NONE));
        assert!(!app.show_prefs);
    }

    #[test]
    fn preferences_can_open_config_in_editor() {
        let mut app = app();
        app.config_path = PathBuf::from("/tmp/config.toml");
        app.show_prefs = true;

        handle_key(&mut app, press(TermCode::Char('e'), KeyModifiers::NONE));

        assert!(!app.show_prefs);
        assert_eq!(
            app.request,
            Some(Request::Edit(PathBuf::from("/tmp/config.toml")))
        );
    }

    #[test]
    fn preferences_rebind_unbound_key_and_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let mut app = app();
        app.config_path = config_path.clone();
        app.show_prefs = true;
        app.prefs_key_cursor = levi_core::keymap::SORTED_ACTIONS
            .iter()
            .position(|action| *action == Action::Help)
            .expect("acción help");

        // Intro inicia la captura.
        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(app.prefs_capture);

        // Una tecla sin asignar antes no debe ser ignorada.
        handle_key(&mut app, press(TermCode::Char('z'), KeyModifiers::NONE));
        assert!(!app.prefs_capture);
        let keys = app.keymap.keys_for(Action::Help);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].to_string(), "z");

        // `s` guarda aunque no esté asignada en el keymap.
        handle_key(&mut app, press(TermCode::Char('s'), KeyModifiers::NONE));
        let saved = std::fs::read_to_string(&config_path).expect("config guardada");
        assert!(saved.contains("help = \"z\""), "config: {saved}");
    }

    #[test]
    fn prefs_search_bar_filters_actions() {
        let mut app = app();
        app.show_prefs = true;

        // «/» enfoca la barra de búsqueda de acciones.
        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(app.prefs_search_active);
        assert!(app.prefs_query.is_empty());

        // Escribir filtra: «salir» deja solo la acción de salir y el cursor
        // cae en ella.
        for character in "salir".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.prefs_query, "salir");
        let visible = app.prefs_visible_actions();
        assert_eq!(visible.len(), 1, "«salir» solo coincide con salir");
        assert_eq!(levi_core::keymap::SORTED_ACTIONS[visible[0]], Action::Quit);
        assert_eq!(app.prefs_key_cursor, visible[0]);

        // «/» con la barra abierta no cierra ni se escribe.
        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(app.prefs_search_active);
        assert_eq!(app.prefs_query, "salir");

        // Intro confirma: el filtro queda aplicado y el foco vuelve a la
        // lista para editar la tecla de la acción seleccionada.
        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(!app.prefs_search_active);
        assert_eq!(app.prefs_query, "salir");
        assert_eq!(app.prefs_visible_actions(), vec![visible[0]]);

        // Esc limpia el filtro sin cerrar el overlay…
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(app.show_prefs);
        assert!(app.prefs_query.is_empty());
        assert_eq!(
            app.prefs_visible_actions().len(),
            levi_core::keymap::SORTED_ACTIONS.len()
        );

        // …y el segundo Esc sí lo cierra.
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.show_prefs);
    }

    #[test]
    fn prefs_search_bar_takes_over_the_overlay_keys() {
        let mut app = app();
        app.show_prefs = true;

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        // Mientras la barra tiene el foco, «s», «r» y «t» se escriben en la
        // consulta en lugar de guardar, restablecer o abrir temas.
        for key in ['s', 'r', 't'] {
            handle_key(&mut app, press(TermCode::Char(key), KeyModifiers::NONE));
        }
        assert_eq!(app.prefs_query, "srt");
        assert!(!app.show_themes);
        assert!(
            app.status.is_none(),
            "«s» no debe guardar con la barra activa"
        );

        // Esc cancela la barra; el overlay sigue abierto.
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.prefs_search_active);
        assert!(app.prefs_query.is_empty());
        assert!(app.show_prefs);
    }

    #[test]
    fn prefs_search_bar_matches_assigned_keys() {
        let mut app = app();
        app.show_prefs = true;

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        // Buscar por la tecla asignada encuentra la acción que la usa.
        for character in "f6".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        let visible = app.prefs_visible_actions();
        assert_eq!(visible.len(), 1, "«f6» solo coincide con la super búsqueda");
        assert_eq!(
            levi_core::keymap::SORTED_ACTIONS[visible[0]],
            Action::GlobalSearch
        );

        // Sin coincidencias, Intro confirma el filtro y un segundo Intro
        // avisa en vez de iniciar la captura.
        for _ in 0..2 {
            handle_key(&mut app, press(TermCode::Backspace, KeyModifiers::NONE));
        }
        for character in "zzz".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert!(app.prefs_visible_actions().is_empty());
        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(!app.prefs_search_active);
        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(!app.prefs_capture, "no se captura sin coincidencias");
        assert!(app.status.as_ref().is_some_and(|status| !status.error));
    }

    #[test]
    fn ctrl_s_opens_shell_and_enter_requests_command() {
        let mut app = app();
        app.tabs[app.active].cwd = PathBuf::from("/tmp");

        handle_key(&mut app, press(TermCode::Char('s'), KeyModifiers::CONTROL));
        assert!(
            app.prompt
                .as_ref()
                .is_some_and(|prompt| prompt.kind == crate::app::PromptKind::Shell)
        );

        for character in "echo hola".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));

        assert!(app.prompt.is_none());
        match app.request.take() {
            Some(Request::Shell { command, cwd }) => {
                assert_eq!(command, "echo hola");
                assert_eq!(cwd, PathBuf::from("/tmp"));
            }
            other => panic!("se esperaba Request::Shell, llegó {other:?}"),
        }
    }

    #[test]
    fn unimplemented_action_shows_message() {
        let mut app = app();
        handle_key(&mut app, press(TermCode::Char('m'), KeyModifiers::NONE));
        assert!(
            app.status
                .as_ref()
                .is_some_and(|status| status.text.contains("marcador"))
        );
    }

    #[test]
    fn ctrl_f_filters_entries_and_enter_keeps_filter() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        std::fs::write(dir.path().join("beta.txt"), "").expect("archivo");
        std::fs::create_dir(dir.path().join("alpha_dir")).expect("carpeta");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        assert!(
            app.tabs[app.active].search_active,
            "Ctrl+F abre la búsqueda"
        );

        for character in "alpha".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.tabs[app.active].filter, "alpha");
        assert_eq!(app.tabs[app.active].entries.len(), 2);
        assert!(
            app.tabs[app.active]
                .entries
                .iter()
                .all(|entry| entry.name.contains("alpha"))
        );

        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert_eq!(
            app.tabs[app.active].filter, "alpha",
            "Intro conserva el filtro"
        );
        assert_eq!(app.tabs[app.active].entries.len(), 2);
    }

    /// Espera (con tope de tiempo) a que la vista previa en segundo plano
    /// contenga una línea concreta, igual que hace el bucle principal.
    fn wait_for_preview(app: &mut App, expected: &str) {
        for _ in 0..500 {
            app.poll_previews();
            if app.tabs[app.active]
                .preview_lines
                .iter()
                .any(|line| line == expected)
            {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("la vista previa nunca mostró «{expected}»");
    }

    #[test]
    fn preview_updates_when_file_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("log.txt");
        std::fs::write(&file, "uno\n").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        wait_for_preview(&mut app, "uno");

        std::fs::write(&file, "dos lineas\n").expect("archivo actualizado");
        app.tick_preview();

        wait_for_preview(&mut app, "dos lineas");
    }

    #[test]
    fn copy_and_cut_show_feedback() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("uno.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.clipboard.len(), 1);
        assert!(!app.clipboard_cut);
        assert!(
            app.status
                .as_ref()
                .is_some_and(|status| status.text.contains("copiados"))
        );

        handle_key(&mut app, press(TermCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(app.clipboard.len(), 1);
        assert!(app.clipboard_cut);
        assert!(
            app.status
                .as_ref()
                .is_some_and(|status| status.text.contains("cortados"))
        );
    }

    #[test]
    fn global_search_enter_copies_selected_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("levi.md"), "# Levi").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());
        app.tabs[app.active].search_global = true;
        app.tabs[app.active].global_results = app.tabs[app.active].entries.clone();
        app.tabs[app.active].filter = "levi".to_string();
        app.tabs[app.active].search_active = true;

        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));

        assert!(!app.tabs[app.active].search_global);
        assert!(!app.tabs[app.active].search_active);
        assert_eq!(app.clipboard.len(), 1);
        assert_eq!(app.clipboard[0], dir.path().join("levi.md"));
    }

    #[test]
    fn filter_ignores_case_and_accents() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Música.txt"), "").expect("archivo");
        std::fs::write(dir.path().join("MUSICA.txt"), "").expect("archivo");
        std::fs::write(dir.path().join("otro.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        for character in "musica".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }

        assert_eq!(app.tabs[app.active].entries.len(), 2);
        assert!(
            app.tabs[app.active].entries.iter().all(|entry| entry
                .name
                .to_lowercase()
                .contains("musica")
                || entry.name.to_lowercase().contains("música"))
        );

        // También funciona al escribir con acento.
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        for character in "Música".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.tabs[app.active].entries.len(), 2);
    }

    #[test]
    fn search_allows_editing_with_cursor() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("abc.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        for character in "abc".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.tabs[app.active].filter, "abc");

        handle_key(&mut app, press(TermCode::Left, KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(app.tabs[app.active].filter, "abXc");

        handle_key(&mut app, press(TermCode::Home, KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Char('Z'), KeyModifiers::NONE));
        assert_eq!(app.tabs[app.active].filter, "ZabXc");

        handle_key(&mut app, press(TermCode::End, KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.tabs[app.active].filter, "ZabX");

        handle_key(&mut app, press(TermCode::Home, KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.tabs[app.active].filter, "abX");
    }

    #[test]
    fn delete_confirmation_lists_count_and_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        for name in ["zeta.txt", "alfa.txt", "beta.txt"] {
            std::fs::write(dir.path().join(name), "").expect("archivo");
        }
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('a'), KeyModifiers::CONTROL));
        handle_key(&mut app, press(TermCode::Char('d'), KeyModifiers::NONE));

        let prompt = app.prompt.as_ref().expect("confirmación de papelera");
        assert_eq!(prompt.kind, crate::app::PromptKind::ConfirmTrash);
        assert_eq!(prompt.items, vec!["alfa.txt", "beta.txt", "zeta.txt"]);

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(app.prompt.is_none());

        handle_key(&mut app, press(TermCode::Char('D'), KeyModifiers::NONE));
        let prompt = app.prompt.as_ref().expect("confirmación de borrado");
        assert_eq!(prompt.kind, crate::app::PromptKind::ConfirmDeletePermanent);
        assert_eq!(prompt.items, vec!["alfa.txt", "beta.txt", "zeta.txt"]);
    }

    #[test]
    fn slash_starts_global_search_and_escape_restores() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        std::fs::write(dir.path().join("beta.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(
            app.tabs[app.active].search_active,
            "/ abre la super búsqueda"
        );
        assert!(
            app.tabs[app.active].search_global,
            "/ activa el modo de super búsqueda"
        );

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert!(!app.tabs[app.active].search_global);
        assert!(app.tabs[app.active].filter.is_empty());
    }

    #[test]
    fn global_search_opens_with_the_opening_keys_and_closes_with_escape() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("levi.md"), "# Levi").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        // «/», F6 y Ctrl+Shift+F abren la super búsqueda…
        for key in [
            press(TermCode::Char('/'), KeyModifiers::NONE),
            press(TermCode::F(6), KeyModifiers::NONE),
            press(
                TermCode::Char('F'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
        ] {
            handle_key(&mut app, key);
            assert!(app.tabs[app.active].search_active);
            assert!(app.tabs[app.active].search_global);

            // …pero no la cierran: mientras está abierta no hacen nada.
            handle_key(&mut app, key);
            assert!(
                app.tabs[app.active].search_active,
                "la tecla que abre no cierra la barra"
            );
            assert!(app.tabs[app.active].filter.is_empty(), "«/» no se escribe");

            // Esc siempre cierra y cancela.
            handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
            assert!(!app.tabs[app.active].search_active);
            assert!(!app.tabs[app.active].search_global);
        }
    }

    #[test]
    fn opening_keys_do_not_close_the_normal_search() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        assert!(app.tabs[app.active].search_active);
        assert!(!app.tabs[app.active].search_global);

        // La tecla que abre no cierra, y «/» no se escribe en el filtro.
        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        handle_key(&mut app, press(TermCode::Char('f'), KeyModifiers::CONTROL));
        assert!(
            app.tabs[app.active].search_active,
            "Ctrl+F no cierra la barra que abrió"
        );
        assert!(app.tabs[app.active].filter.is_empty());

        // Esc cierra cancelando la búsqueda.
        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert!(app.tabs[app.active].filter.is_empty());
    }

    #[test]
    fn remapped_opening_keys_only_open() {
        // Reproduce una configuración remapeada: «/» abre la búsqueda normal
        // y ctrl+F la super búsqueda. Las teclas de apertura nunca cierran;
        // las barras se cierran con Esc.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        let mut keymap = Keymap::defaults();
        keymap.rebind(Action::GlobalSearch, Key::parse("ctrl+F").expect("ctrl+F"));
        keymap.rebind(Action::Filter, Key::parse("/").expect("slash"));
        let mut app = App::with_config(
            dir.path().to_path_buf(),
            Config::default(),
            Theme::default(),
            keymap,
        );

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(app.tabs[app.active].search_active);
        assert!(
            !app.tabs[app.active].search_global,
            "con «/» remapeada abre la búsqueda normal"
        );

        for character in "al".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.tabs[app.active].filter, "al");

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(
            app.tabs[app.active].search_active,
            "«/» no cierra la barra, solo la abre"
        );
        assert_eq!(
            app.tabs[app.active].filter, "al",
            "«/» no se escribe en el filtro"
        );

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert!(app.tabs[app.active].filter.is_empty());
    }

    #[test]
    fn slash_and_f6_open_the_global_search() {
        // Con el keymap por defecto «/» y F6 abren la super búsqueda; la
        // barra se cierra con Esc.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(app.tabs[app.active].search_active);
        assert!(
            app.tabs[app.active].search_global,
            "«/» debe abrir la super búsqueda"
        );

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);

        handle_key(&mut app, press(TermCode::F(6), KeyModifiers::NONE));
        assert!(app.tabs[app.active].search_active);
        assert!(
            app.tabs[app.active].search_global,
            "F6 también abre la super búsqueda"
        );
    }

    #[test]
    fn slash_reopens_a_fresh_global_search() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        for character in "al".chars() {
            handle_key(
                &mut app,
                press(TermCode::Char(character), KeyModifiers::NONE),
            );
        }

        handle_key(&mut app, press(TermCode::Esc, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert!(
            app.tabs[app.active].filter.is_empty(),
            "Esc cancela y limpia la consulta"
        );

        handle_key(&mut app, press(TermCode::Char('/'), KeyModifiers::NONE));
        assert!(app.tabs[app.active].search_active);
        assert!(
            app.tabs[app.active].search_global,
            "«/» reabre la super búsqueda, no la normal"
        );
        assert!(
            app.tabs[app.active].filter.is_empty(),
            "reabrir empieza de cero"
        );
    }

    #[test]
    fn enter_on_an_empty_global_search_just_closes_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("alpha.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        handle_key(&mut app, press(TermCode::F(6), KeyModifiers::NONE));
        assert!(app.tabs[app.active].search_global);

        handle_key(&mut app, press(TermCode::Enter, KeyModifiers::NONE));
        assert!(!app.tabs[app.active].search_active);
        assert!(
            app.status.is_none(),
            "sin consulta no intenta copiar ni avisa"
        );
        assert!(
            !app.tabs[app.active].entries.is_empty(),
            "la lista vuelve al listado normal del directorio"
        );
    }

    #[test]
    fn ctrl_p_goes_to_trash() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("uno.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());
        app.tabs[app.active].locations_mode = true;

        handle_key(&mut app, press(TermCode::Char('p'), KeyModifiers::CONTROL));

        match levi_core::platform::trash_dir() {
            // Papelera navegable: Ctrl+P entra en ella.
            Some(trash) if trash.is_dir() => {
                assert_eq!(
                    app.tabs[app.active].cwd, trash,
                    "Ctrl+P navega a la papelera"
                );
                assert!(!app.tabs[app.active].locations_mode);
            }
            // Papelera aún sin crear: aviso informativo, sin cambiar de directorio.
            Some(_) => {
                assert_eq!(app.tabs[app.active].cwd, dir.path().to_path_buf());
                assert!(
                    app.status.as_ref().is_some_and(|status| !status.error),
                    "papelera sin crear debe avisar, no fallar"
                );
            }
            // Sin papelera navegable (p. ej. Windows): error amable.
            None => {
                assert_eq!(app.tabs[app.active].cwd, dir.path().to_path_buf());
                assert!(app.status.as_ref().is_some_and(|status| status.error));
            }
        }
    }

    #[test]
    fn selection_keys_mark_and_clear_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("uno.txt"), "").expect("archivo");
        std::fs::write(dir.path().join("dos.txt"), "").expect("archivo");
        let mut app = App::new(dir.path().to_path_buf());

        let first = app.tabs[app.active].entries[0].path.clone();
        handle_key(&mut app, press(TermCode::Char(' '), KeyModifiers::NONE));
        assert!(app.tabs[app.active].selected.contains(&first));

        handle_key(&mut app, press(TermCode::Char(' '), KeyModifiers::NONE));
        assert!(!app.tabs[app.active].selected.contains(&first));

        handle_key(&mut app, press(TermCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(
            app.tabs[app.active].selected.len(),
            app.tabs[app.active].entries.len()
        );

        handle_key(&mut app, press(TermCode::Char('d'), KeyModifiers::CONTROL));
        assert!(app.tabs[app.active].selected.is_empty());

        handle_key(&mut app, press(TermCode::Char(' '), KeyModifiers::CONTROL));
        assert_eq!(
            app.tabs[app.active].selected.len(),
            app.tabs[app.active].entries.len()
        );

        handle_key(&mut app, press(TermCode::Char('d'), KeyModifiers::CONTROL));
        handle_key(&mut app, press(TermCode::Null, KeyModifiers::CONTROL));
        assert_eq!(
            app.tabs[app.active].selected.len(),
            app.tabs[app.active].entries.len()
        );
    }
}
