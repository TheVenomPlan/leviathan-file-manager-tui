//! Resolución del editor externo que se lanza con la tecla `e`.

/// Editor ya resuelto y listo para ejecutar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub command: String,
    pub args: Vec<String>,
    /// `true` si ocupa la terminal y hay que suspender la TUI.
    pub terminal: bool,
}

/// Resuelve el editor en cascada:
/// configuración > `$VISUAL` > `$EDITOR` > fallback del sistema.
pub fn resolve(configured: Option<&str>) -> Editor {
    let raw = configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| env_non_empty("VISUAL"))
        .or_else(|| env_non_empty("EDITOR"))
        .unwrap_or_else(|| default_editor().to_string());

    let mut parts = raw.split_whitespace();
    let command = parts.next().unwrap_or("vi").to_string();
    let args: Vec<String> = parts.map(str::to_string).collect();
    let terminal = is_terminal_editor(&command);

    Editor {
        command,
        args,
        terminal,
    }
}

/// `false` solo para editores gráficos conocidos; el resto se lanza en la terminal.
pub fn is_terminal_editor(command: &str) -> bool {
    const GUI: [&str; 9] = [
        "code",
        "codium",
        "subl",
        "gedit",
        "kate",
        "xed",
        "mousepad",
        "notepad",
        "notepad++",
    ];
    !GUI.contains(&basename(command).as_str())
}

fn basename(command: &str) -> String {
    let name = command.rsplit(['/', '\\']).next().unwrap_or(command);
    name.trim_end_matches(".exe").to_lowercase()
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn default_editor() -> &'static str {
    if cfg!(windows) {
        return "notepad";
    }
    if in_path("nano") {
        "nano"
    } else if in_path("vim") {
        "vim"
    } else {
        "vi"
    }
}

fn in_path(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        dir.join(command).is_file()
            || (cfg!(windows) && dir.join(format!("{command}.exe")).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_terminal_and_gui_editors() {
        assert!(is_terminal_editor("nvim"));
        assert!(is_terminal_editor("/usr/bin/nano"));
        assert!(!is_terminal_editor("code"));
        assert!(!is_terminal_editor("C:\\Program Files\\Code\\code.exe"));
        assert!(is_terminal_editor("mi-editor-raro"));
    }

    #[test]
    fn configured_editor_wins() {
        let editor = resolve(Some("nvim -u NONE"));
        assert_eq!(editor.command, "nvim");
        assert_eq!(editor.args, vec!["-u", "NONE"]);
        assert!(editor.terminal);
    }
}
