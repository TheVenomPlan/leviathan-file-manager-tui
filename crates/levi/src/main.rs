use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, anyhow};
use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use directories::BaseDirs;
use levi_core::config::{self, IconsMode};
use levi_core::editor;
use levi_core::i18n::{self, Language};
use levi_core::keymap::{ALL, Keymap};
use levi_core::theme::Theme;
use levi_tui::app::{App, Request};
use levi_tui::terminal::{Tui, emergency_restore};
use levi_tui::{input, ui};
use tracing_subscriber::EnvFilter;

const HELP_TAIL: &str = "\
ATAJOS
  Los atajos se definen en [keys] de config.toml. Cada acción acepta una tecla
  o una lista de teclas, y los nombres valen en inglés o español:

    quit / salir            copy / copiar            paste / pegar
    trash / papelera        rename / renombrar       select_all / seleccionar_todo
    open / abrir            edit / editar            toggle_hidden / ocultos

  Ejemplo:

    [keys]
    quit = [\"q\", \"ctrl+c\"]
    copiar = \"y\"
    pegar = [\"p\", \"ctrl+v\"]

  Pulsa ',' dentro de la TUI para ver las preferencias y los atajos activos.
  Usa 'levi --keys' para consultarlos sin abrir la interfaz.

TEMAS
  default (azul estilo Bazzite), nord, light o el nombre de un archivo en
  ~/.config/levi/themes/. Los colores se escriben en HEX (#RRGGBB).
  --theme sobreescribe lo indicado en config.toml.

ICONOS
  emoji (grandes y con color), nerd (requiere una Nerd Font), ascii o none.
  --icons sobreescribe lo indicado en config.toml.

BÚSQUEDA Y SELECCIÓN
  Ctrl+F o /   filtrar el directorio mientras escribes (Intro confirma, Esc cancela)
  Space        marcar o desmarcar la entrada bajo el cursor
  Ctrl+A       marcar todas las entradas visibles
  Ctrl+D       vaciar la selección

SHELL
  Ctrl+S  abrir la barra Shell y ejecutar un comando en el directorio actual

ARCHIVOS
  o / Intro  abrir con la aplicación predeterminada del sistema
  e          editar con $VISUAL, $EDITOR o el editor de config.toml
  Ctrl+P     ir a la papelera del sistema

CONFIGURACIÓN
  ~/.config/levi/config.toml (Linux) o %APPDATA%\\levi\\config.toml (Windows)";

/// Administrador de archivos para la terminal.
#[derive(Debug, Parser)]
#[command(
    name = "levi",
    version,
    about = "Administrador de archivos para la terminal",
    after_help = HELP_TAIL
)]
struct Cli {
    /// Directorio donde empezar
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Tema: default (azul Bazzite), nord, light o un archivo de themes/
    #[arg(long)]
    theme: Option<String>,

    /// Iconos: emoji, nerd, ascii o none
    #[arg(long)]
    icons: Option<String>,

    /// Editor para la tecla `e` (sobreescribe la configuración)
    #[arg(long)]
    editor: Option<String>,

    /// Muestra los atajos activos y sale, sin abrir la TUI
    #[arg(long)]
    keys: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_dir = config_dir();

    let mut warnings = Vec::new();
    if let Some(warning) = config::ensure_default_file(&config_dir) {
        warnings.push(warning);
    }
    let (mut config, config_warnings) = config::load(&config_dir);
    warnings.extend(config_warnings);

    if let Some(theme) = cli.theme.as_deref() {
        config.theme = theme.to_string();
    }
    if let Some(icons) = cli.icons.as_deref() {
        config.icons = icons.parse::<IconsMode>().map_err(|err| anyhow!(err))?;
    }
    if let Some(editor) = cli.editor.as_deref() {
        config.editor = Some(editor.to_string());
    }

    let (keymap, key_warnings) = Keymap::from_config(&config.keys);
    warnings.extend(key_warnings);

    // Sin TUI: imprimir los atajos y terminar.
    if cli.keys {
        let s = i18n::strings(config.language);
        for warning in &warnings {
            eprintln!("{}", i18n::fill(s.warning_prefix, &[("warning", warning)]));
        }
        print_keys(&keymap, &config_dir, config.language);
        return Ok(());
    }

    let _log_guard = init_logging()?;
    install_panic_hook();

    let start = resolve_start(&cli.path, config.language)?;
    let (theme, theme_warning) = Theme::load(&config_dir, &config.theme);
    if let Some(warning) = theme_warning {
        warnings.push(warning);
    }

    tracing::info!(
        path = %start.display(),
        theme = %config.theme,
        icons = ?config.icons,
        keys = config.keys.len(),
        "iniciando levi"
    );

    let mut terminal = Tui::new().context("no se pudo iniciar la terminal")?;
    let mut app = App::with_config(start, config, theme, keymap);
    app.config_path = config::config_path(&config_dir);
    app.config_dir = config_dir.clone();
    if !warnings.is_empty() {
        app.set_error(warnings.join(" · "));
    }

    while !app.should_quit {
        // Refrescar la vista previa si el archivo cambió en disco.
        app.tick_preview();
        app.tick_theme();
        app.tick_dir();
        app.poll_dir_sizes();
        app.poll_previews();
        app.poll_global_search();

        let completed = terminal
            .terminal
            .draw(|frame| ui::render(frame, &mut app))
            .context("error al dibujar la interfaz")?;

        // La selección con el ratón se copia al soltar. El texto se extrae del
        // buffer recién dibujado para respetar exactamente lo que se ve.
        let copy_text = if app.copy_selection {
            app.copy_selection = false;
            app.text_selection
                .map(|selection| ui::selection_text(completed.buffer, selection))
        } else {
            None
        };

        if let Some(text) = copy_text {
            let s = app.strings();
            if text.is_empty() {
                app.set_message(s.no_text_selected.to_string());
            } else {
                match terminal.copy_to_clipboard(&text) {
                    Ok(()) => {
                        app.set_message(i18n::fill(
                            s.copied_chars,
                            &[("count", &text.chars().count().to_string())],
                        ));
                    }
                    Err(err) => {
                        app.set_error(i18n::fill(s.copy_failed, &[("err", &err.to_string())]));
                    }
                }
            }
            // Redibujar para mostrar el aviso de copiado sin esperar al tick.
            terminal
                .terminal
                .draw(|frame| ui::render(frame, &mut app))
                .context("error al dibujar la interfaz")?;
        }

        if event::poll(Duration::from_millis(250)).context("error al esperar eventos")? {
            match event::read().context("error al leer eventos")? {
                // Con el protocolo kitty de teclado activo, mantener una tecla
                // envía eventos `Repeat` (y también llegan `Release`): se
                // aceptan pulsaciones y repeticiones; solo se descartan las
                // liberaciones, o mantener ↑/↓ no movería el cursor.
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    app.clear_text_selection();
                    input::handle_key(&mut app, key);

                    if let Some(request) = app.request.take() {
                        handle_request(&mut terminal, &mut app, request)?;
                    }
                }
                Event::Mouse(mouse) => {
                    let area = terminal
                        .area()
                        .context("no se pudo obtener el tamaño de la terminal")?;
                    input::handle_mouse(&mut app, mouse, area);
                }
                Event::Resize(_, _) => app.clear_text_selection(),
                _ => {}
            }
        }
    }

    tracing::info!("levi cerrado");
    Ok(())
}

/// Imprime la tabla de atajos activos.
fn print_keys(keymap: &Keymap, config_dir: &Path, language: Language) {
    let s = i18n::strings(language);
    println!("{}", s.keys_title);
    println!(
        "{}\n",
        i18n::fill(
            s.keys_config,
            &[(
                "path",
                &config::config_path(config_dir).display().to_string()
            )],
        )
    );
    println!(
        "{:<20} {:<32} {}",
        s.keys_action, s.keys_description, s.keys_keys
    );

    for action in ALL {
        let keys = keymap.keys_for(action);
        let keys = if keys.is_empty() {
            s.keys_unassigned.to_string()
        } else {
            keys.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        };
        let status = if action.implemented() {
            ""
        } else {
            s.keys_soon
        };
        println!(
            "{:<20} {:<32} {keys}{status}",
            action.name(),
            action.label_in(language)
        );
    }
}

/// Ejecuta las acciones que necesitan acceso a la terminal (abrir/editar).
fn handle_request(tui: &mut Tui, app: &mut App, request: Request) -> anyhow::Result<()> {
    match request {
        Request::Open(path) => {
            if let Err(err) = open::that(&path) {
                let text = i18n::fill(
                    app.strings().open_failed,
                    &[
                        ("path", &path.display().to_string()),
                        ("err", &err.to_string()),
                    ],
                );
                app.set_error(text);
            }
        }
        Request::Edit(path) => run_editor(tui, app, &path)?,
        Request::Shell { command, cwd } => run_shell(tui, app, &command, &cwd)?,
    }
    Ok(())
}

/// Ejecuta un comando de shell en el directorio de la pestaña activa.
fn run_shell(tui: &mut Tui, app: &mut App, command: &str, cwd: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    let mut process = {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    };
    #[cfg(not(windows))]
    let mut process = {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        process
    };
    process.current_dir(cwd);

    tracing::info!(command, cwd = %cwd.display(), "ejecutando comando de shell");

    tui.suspend().context("no se pudo suspender la terminal")?;
    let status = process.status();
    tui.resume().context("no se pudo restaurar la terminal")?;
    app.refresh();

    match status {
        Ok(status) if status.success() => {
            let text = i18n::fill(app.strings().shell_done, &[("command", command)]);
            app.set_message(text);
        }
        Ok(status) => {
            let text = i18n::fill(
                app.strings().shell_exit,
                &[("command", command), ("status", &status.to_string())],
            );
            app.set_error(text);
        }
        Err(err) => {
            let text = i18n::fill(
                app.strings().shell_failed,
                &[("command", command), ("err", &err.to_string())],
            );
            app.set_error(text);
        }
    }
    Ok(())
}

fn run_editor(tui: &mut Tui, app: &mut App, path: &Path) -> anyhow::Result<()> {
    let editor = editor::resolve(app.config.editor.as_deref());
    let mut command = Command::new(&editor.command);
    command.args(&editor.args).arg(path);

    tracing::info!(editor = %editor.command, terminal = editor.terminal, path = %path.display(), "abriendo editor");

    if editor.terminal {
        tui.suspend().context("no se pudo suspender la terminal")?;
        let status = command.status();
        tui.resume().context("no se pudo restaurar la terminal")?;
        app.refresh();

        match status {
            Ok(status) if status.success() => {
                if path == app.config_path {
                    app.set_message(app.strings().config_edited.to_string());
                } else {
                    let text = i18n::fill(
                        app.strings().edited,
                        &[("path", &path.display().to_string())],
                    );
                    app.set_message(text);
                }
            }
            Ok(status) => {
                let text = i18n::fill(
                    app.strings().editor_exit,
                    &[("status", &status.to_string())],
                );
                app.set_error(text);
            }
            Err(err) => {
                let text = i18n::fill(
                    app.strings().launch_failed,
                    &[("command", &editor.command), ("err", &err.to_string())],
                );
                app.set_error(text);
            }
        }
    } else {
        match command.spawn() {
            Ok(_) => {
                let text = i18n::fill(app.strings().opening_with, &[("command", &editor.command)]);
                app.set_message(text);
            }
            Err(err) => {
                let text = i18n::fill(
                    app.strings().launch_failed,
                    &[("command", &editor.command), ("err", &err.to_string())],
                );
                app.set_error(text);
            }
        }
    }

    Ok(())
}

/// Convierte la ruta recibida en una ruta absoluta y existente.
fn resolve_start(path: &Path, language: Language) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let resolved = std::fs::canonicalize(&absolute).unwrap_or(absolute);

    if !resolved.is_dir() {
        let text = i18n::fill(
            i18n::strings(language).not_a_directory,
            &[("path", &resolved.display().to_string())],
        );
        anyhow::bail!(text);
    }
    Ok(resolved)
}

/// Directorio de configuración: `~/.config/levi` (Linux) o `%APPDATA%\levi` (Windows).
fn config_dir() -> PathBuf {
    BaseDirs::new().map_or_else(|| PathBuf::from("."), |base| base.config_dir().join("levi"))
}

/// Configura `tracing` para escribir en un archivo, nunca en la terminal.
fn init_logging() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let dir =
        BaseDirs::new().map_or_else(|| PathBuf::from("."), |base| base.cache_dir().join("levi"));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("no se pudo crear el directorio de logs «{}»", dir.display()))?;

    let file = tracing_appender::rolling::never(&dir, "levi.log");
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(filter)
        .with_target(false)
        .init();

    Ok(guard)
}

/// Restaura la terminal antes de mostrar un panic.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        emergency_restore();
        default_hook(info);
    }));
}
