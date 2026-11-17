#!/bin/sh
# Punto de entrada único para el desarrollo de Levi.
#
# Uso:
#   ./scripts/dev.sh <comando> [args...]
#
# Comandos:
#   quick     Formato + clippy (bucle rápido, ideal para pre-commit)
#   check     Validación completa: quick + tests + check de Windows
#   fmt       Comprueba el formato sin escribir
#   fix       Aplica rustfmt
#   clippy    Clippy en todo el workspace
#   test      Tests de todo el workspace
#   win       Check del target x86_64-pc-windows-gnu
#   run       Ejecuta la aplicación (pasa los argumentos a levi)
#   doctor    Comprueba las herramientas del entorno
#   hooks     Activa los hooks versionados en .githooks
#   help      Muestra esta ayuda
#
# No depende de just, make ni de ningún gestor de tareas: solo de sh y cargo.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

# Ajustes locales opcionales, ignorados por git: enlazadores, rutas, etc.
if [ -f "$ROOT/.env.dev" ]; then
    # shellcheck disable=SC1091
    . "$ROOT/.env.dev"
fi

# En algunas instalaciones cargo no está en PATH (por ejemplo, cuando se
# ejecuta desde un entorno sin el perfil del usuario cargado).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        PATH="$HOME/.cargo/bin:$PATH"
        export PATH
    else
        echo "error: cargo no está en PATH" >&2
        exit 127
    fi
fi

CARGO=${CARGO:-cargo}

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "error: falta '$1' en PATH" >&2
        exit 127
    fi
}

fmt_check() {
    need cargo
    "$CARGO" fmt --all -- --check
}

fmt_fix() {
    need cargo
    "$CARGO" fmt --all
}

clippy() {
    need cargo
    "$CARGO" clippy --workspace --all-targets --all-features -- -D warnings
}

test_workspace() {
    need cargo
    "$CARGO" test --workspace --all-features
}

win_check() {
    need cargo
    "$CARGO" check --workspace --all-targets --all-features --target x86_64-pc-windows-gnu
}

# Imprime "ok" o "FALTA" sin abortar el resto del diagnóstico.
report() {
    label=$1
    shift
    if "$@" >/dev/null 2>&1; then
        printf '  %-32s ok\n' "$label"
        return 0
    fi
    printf '  %-32s FALTA\n' "$label"
    return 1
}

win_target_ok() {
    if command -v rustup >/dev/null 2>&1; then
        rustup target list --installed 2>/dev/null | grep -qx 'x86_64-pc-windows-gnu'
        return
    fi
    libdir=$(rustc --print target-libdir --target x86_64-pc-windows-gnu 2>/dev/null) || return 1
    [ -d "$libdir" ]
}

doctor() {
    missing=0

    printf 'Levi: diagnóstico del entorno\n'
    printf '\nHerramientas obligatorias:\n'
    report "cargo" cargo --version || missing=1
    report "rustc" rustc --version || missing=1
    report "rustfmt" cargo fmt --version || missing=1
    report "clippy" cargo clippy --version || missing=1
    report "git" git --version || missing=1

    printf '\nTargets de compilación:\n'
    if win_target_ok; then
        printf '  %-32s ok\n' "x86_64-pc-windows-gnu"
    else
        printf '  %-32s FALTA\n' "x86_64-pc-windows-gnu"
        printf '      instala con: rustup target add x86_64-pc-windows-gnu\n'
        missing=1
    fi

    printf '\nEnlazadores:\n'
    if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1 \
        || [ -n "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER:-}" ]; then
        printf '  %-32s ok\n' "Linux (cc/gcc/clang)"
    else
        printf '  %-32s FALTA\n' "Linux (cc/gcc/clang)"
        printf '      instala build-essential, o define\n'
        printf '      CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER\n'
        missing=1
    fi
    if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        printf '  %-32s ok\n' "Windows (mingw-w64)"
    else
        printf '  %-32s opcional\n' "Windows (mingw-w64)"
        printf '      solo hace falta para enlazar binarios Windows; cargo check no lo usa\n'
    fi

    printf '\nHerramientas opcionales (mejoran el bucle de desarrollo):\n'
    for tool in just bacon cargo-nextest sccache mold; do
        if command -v "$tool" >/dev/null 2>&1; then
            printf '  %-32s ok\n' "$tool"
        else
            printf '  %-32s no instalada\n' "$tool"
        fi
    done

    printf '\n'
    if [ "$missing" -ne 0 ]; then
        echo "Faltan herramientas obligatorias; revisa los mensajes anteriores." >&2
        return 1
    fi
    echo "Entorno listo para ./scripts/dev.sh check."
}

install_hooks() {
    need git
    git config core.hooksPath .githooks
    chmod +x .githooks/pre-commit .githooks/pre-push 2>/dev/null || true
    echo "hooks activados: core.hooksPath=.githooks"
    echo "  pre-commit -> ./scripts/dev.sh quick"
    echo "  pre-push   -> ./scripts/dev.sh check"
}

usage() {
    awk '
        /^# Uso:/ { show = 1 }
        show && /^#/ { sub(/^# ?/, ""); print; next }
        show && !/^#/ { exit }
    ' "$0"
}

cmd=${1:-help}
if [ $# -gt 0 ]; then
    shift
fi

case "$cmd" in
    quick)
        fmt_check
        clippy
        ;;
    check)
        fmt_check
        clippy
        test_workspace
        win_check
        ;;
    fmt) fmt_check ;;
    fix) fmt_fix ;;
    clippy) clippy ;;
    test) test_workspace ;;
    win) win_check ;;
    run)
        need cargo
        exec "$CARGO" run -p levi -- "$@"
        ;;
    doctor) doctor ;;
    hooks) install_hooks ;;
    help | -h | --help) usage ;;
    *)
        echo "error: comando desconocido '$cmd'" >&2
        echo >&2
        usage >&2
        exit 2
        ;;
esac
