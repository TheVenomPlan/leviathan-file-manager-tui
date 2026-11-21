MCP FIRST (OBLIGATORIO)

Antes de cualquier tarea:

    Conectarse a CodebaseMemory: http://127.0.0.1:9749/

    Cargar la skill principal del proyecto: skills/levi-context.md

    Consultar primero CodebaseMemory para recuperar:
        arquitectura
        convenciones
        roadmap
        estado actual
        decisiones del proyecto
        bugs conocidos
        funcionalidades implementadas

    Recuperar únicamente el contexto relevante para la tarea actual.

    Leer AGENTS.md únicamente si la información obtenida desde CodebaseMemory es insuficiente.

    No reindexar información ya existente en CodebaseMemory.

    Tras cambios importantes en arquitectura, funcionalidades o decisiones de diseño:
        actualizar CodebaseMemory
        actualizar la memoria relacionada
        mantener sincronizada la documentación

Prioridad de Contexto

Seguir estrictamente este orden:

    CodebaseMemory (http://127.0.0.1:9749/)
    skills/levi-context.md
    Código fuente
    AGENTS.md

La información obtenida desde CodebaseMemory tiene prioridad sobre este archivo.
Proyecto

Leviathan File Manager TUI.

Administrador de archivos para terminal inspirado en Ranger y Yazi.

Escrito íntegramente en Rust.

Plataformas objetivo:

    Linux (principal)
    Windows (compatibilidad obligatoria)

El MVP incluye:

    navegación tipo vim
    panel padre
    panel actual
    panel preview
    operaciones de archivos
    previews
    pestañas
    openers
    temas

No existe editor de texto embebido.
Arquitectura

Workspace:

    levi-core
    levi-tui
    levi

levi-core

Responsable de:

    filesystem
    operaciones
    configuración
    temas
    previews
    lógica de negocio

Restricciones:

    No depende de ratatui.
    No depende de crossterm.
    No contiene código de interfaz.

levi-tui

Responsable de:

    renderizado
    widgets
    overlays
    input
    terminal
    experiencia visual

No debe contener lógica de negocio.
levi

Responsable de:

    CLI
    bootstrap
    entrada principal de la aplicación

Flujo de Datos

crossterm
    ↓
App::update()
    ↓
ui::render()
    ↓
levi-core

La UI consume eventos.

La lógica vive en el core.

Las operaciones pesadas se ejecutan en workers.

La UI nunca debe bloquear.
Decisiones No Negociables

    No existe editor embebido.
    La edición se realiza mediante $EDITOR o $VISUAL.
    PathBuf siempre.
    unwrap() prohibido fuera de tests.
    levi-core no depende de ratatui.
    levi-core no depende de crossterm.
    La UI nunca bloquea.
    Operaciones costosas siempre en workers.
    Borrar significa papelera por defecto.
    El borrado permanente requiere confirmación explícita.
    No escribir logs en stdout ni stderr.
    tracing escribe únicamente a archivo.
    Todo código específico de plataforma debe vivir en platform/.
    No asumir sensibilidad de mayúsculas/minúsculas.
    No asumir comportamiento exclusivo de Unix.
    Todo cambio debe seguir compilando en Windows.
    El crate publicado es levi-fm.
    El binario se llama levi.

Convenciones

    Rust Edition 2024.
    rustfmt obligatorio.
    clippy sin warnings.
    Código en inglés.
    UI y documentación en español.
    thiserror en levi-core.
    anyhow en el binario.
    Conventional Commits.
    Nada de lógica de negocio en widgets.
    Alias de teclado resueltos desde levi-core.
    La UI nunca compara teclas directamente.
    Posicionar cursores en campos de texto usando ancho de visualización (no solo conteo de caracteres).

Desarrollo de Funcionalidades

Antes de implementar:

    Consultar CodebaseMemory.
    Verificar si la funcionalidad ya existe.
    Identificar el crate correcto.
    Mantener la arquitectura actual.
    Mantener compatibilidad Linux y Windows.
    Evitar dependencias innecesarias.
    Mantener separación entre core y UI.

Corrección de Errores

Prioridades:

    Solución mínima.
    No romper APIs existentes.
    No modificar arquitectura innecesariamente.
    Mantener compatibilidad multiplataforma.
    Mantener clippy limpio.
    Mantener tests existentes.

Validación Obligatoria

Antes de considerar una tarea terminada:

cargo fmt --all -- --check

cargo clippy --workspace --all-targets --all-features -- -D warnings

cargo test --workspace --all-features

cargo check --workspace --all-targets --all-features --target x86_64-pc-windows-gnu

Atajo equivalente (sin dependencias externas):

./scripts/dev.sh check

Bucle rápido durante el desarrollo:

./scripts/dev.sh quick

Los hooks de git se activan una vez con:

./scripts/dev.sh hooks

Información Persistente

La siguiente información debe recuperarse desde CodebaseMemory y no mantenerse duplicada en este archivo:

    estado actual del proyecto
    fases completadas
    roadmap
    funcionalidades implementadas
    bugs conocidos
    trampas conocidas
    historial de decisiones
    atajos de teclado
    estructura detallada de módulos
    métricas de tests
    progreso de desarrollo
    documentación histórica