Contexto maestro del proyecto Levi File Manager. Siempre usar CodebaseMemory antes de analizar código o proponer cambios.
PRIORIDAD OBLIGATORIA

Antes de cualquier tarea:

    Consultar MCP CodebaseMemory.
    Recuperar únicamente la memoria relevante.
    No leer AGENTS.md completo salvo que la memoria sea insuficiente.
    No volver a generar contexto ya almacenado.

PROYECTO

Levi File Manager TUI

Lenguaje:

    Rust

Workspace:

    levi-core
    levi-tui
    levi

ARQUITECTURA

Regla fundamental:

levi-core:

    lógica de negocio
    filesystem
    previews
    configuración
    temas
    sin dependencias de UI

levi-tui:

    ratatui
    render
    input
    terminal
    overlays

levi:

    CLI
    bootstrap

Flujo:

crossterm → App::update() → render() → levi-core

Workers:

    listado
    previews
    operaciones

La UI nunca debe bloquear.
RESTRICCIONES OBLIGATORIAS

    Sin editor embebido
    unwrap prohibido fuera de tests
    PathBuf obligatorio
    Linux primero
    Compatibilidad Windows obligatoria
    Core sin ratatui
    Core sin crossterm
    Borrar usa papelera
    Logging solo a archivo
    Nada de lógica de negocio en widgets

ESTADO ACTUAL

Completado:

    fase_0
    fase_1
    fase_2
    previews
    pestañas
    shell
    personalización
    búsqueda
    selección múltiple

Pendiente:

    filtro fuzzy
    NO_COLOR
    CI
    packaging
    Windows testing

DESARROLLO DE FEATURES

Antes de implementar:

    Identificar crate correcto.
    Revisar si la funcionalidad ya existe.
    Mantener arquitectura actual.
    Mantener compatibilidad Linux y Windows.
    Mantener clippy limpio.
    Mantener tests existentes.

BUGFIXES

Prioridad:

    solución mínima
    no romper API
    no cambiar arquitectura
    no introducir dependencias innecesarias

VALIDACIÓN FINAL

Ejecutar conceptualmente:

cargo fmt --all -- --check

cargo clippy --all-targets -- -D warnings

cargo test
cargo check --target x86_64-pc-windows-gnu