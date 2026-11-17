//! Inicialización y restauración de la terminal con garantías.

use std::io::{self, Stdout, stdout};

use crossterm::clipboard::CopyToClipboard;
use crossterm::cursor::{self, SetCursorStyle};
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

/// Ratón: seguimiento de botones y arrastre, sin eventos de movimiento libre.
const ENABLE_MOUSE: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1006h";
const DISABLE_MOUSE: &str = "\x1b[?1006l\x1b[?1002l\x1b[?1000l";

/// Terminal en modo TUI que se restaura automáticamente al salir del ámbito.
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(
            out,
            EnterAlternateScreen,
            Print(ENABLE_MOUSE),
            SetCursorStyle::SteadyBar
        )?;
        // Mejora la detección de combinaciones como Ctrl+Shift+F (Kitty, etc.).
        let _ = execute!(
            out,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        );
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(Self { terminal })
    }

    /// Área visible de la terminal.
    pub fn area(&self) -> io::Result<Rect> {
        let size = self.terminal.size()?;
        Ok(Rect::new(0, 0, size.width, size.height))
    }

    /// Copia texto al portapapeles del terminal mediante OSC 52.
    pub fn copy_to_clipboard(&self, text: &str) -> io::Result<()> {
        execute!(stdout(), CopyToClipboard::to_clipboard_from(text))
    }

    /// Libera la terminal para ejecutar un programa externo (por ejemplo un editor).
    pub fn suspend(&mut self) -> io::Result<()> {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        disable_raw_mode()?;
        execute!(
            stdout(),
            Print(DISABLE_MOUSE),
            LeaveAlternateScreen,
            cursor::Show,
            SetCursorStyle::DefaultUserShape
        )?;
        Ok(())
    }

    /// Recupera la TUI después de un programa externo.
    pub fn resume(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(
            stdout(),
            EnterAlternateScreen,
            Print(ENABLE_MOUSE),
            SetCursorStyle::SteadyBar
        )?;
        let _ = execute!(
            stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        );
        self.terminal.clear()?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = restore();
    }
}

/// Devuelve la terminal a su estado normal.
pub fn restore() -> io::Result<()> {
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    disable_raw_mode()?;
    execute!(
        stdout(),
        Print(DISABLE_MOUSE),
        LeaveAlternateScreen,
        cursor::Show,
        SetCursorStyle::DefaultUserShape
    )?;
    Ok(())
}

/// Restauración de emergencia para el hook de panic: nunca falla ni hace panic.
pub fn emergency_restore() {
    let _ = restore();
}
