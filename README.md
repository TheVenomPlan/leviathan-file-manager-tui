# Leviathan File Manager TUI

Administrador de archivos para terminal inspirado en Ranger y Yazi, escrito en Rust.

> **Sobre el nombre:** el proyecto se llamaba antes «Levi File Manager TUI». El cambio a **Leviathan File Manager TUI** evita la colisión con la marca de calzado Levi's. Por ahora se mantienen los nombres internos (`levi-core`, `levi-tui`, `levi`) y el binario `levi`.

Incluye navegación tipo vim, paneles padre/actual/vista previa, pestañas, operaciones de archivos, previews, openers, temas, búsqueda y selección múltiple. No tiene editor de texto embebido: la edición usa `$EDITOR` o `$VISUAL`.

## Instalación en Linux

### Requisitos

- Rust 1.99 o superior y Cargo. Si no los tienes, instálalos con [rustup](https://rustup.rs/):

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```

- Un enlazador C y las herramientas básicas de compilación (algunas dependencias lo necesitan):

  ```bash
  # Debian / Ubuntu
  sudo apt update && sudo apt install build-essential pkg-config

  # Fedora
  sudo dnf install gcc make

  # Arch Linux
  sudo pacman -S base-devel
  ```

### Desde el repositorio

```bash
git clone https://github.com/TheVenomPlan/leviathan-file-manager-tui.git
cd leviathan-file-manager-tui
cargo install --path crates/levi --locked
```

El binario queda en `~/.cargo/bin/levi`. Asegúrate de tener `~/.cargo/bin` en tu `PATH` (`source "$HOME/.cargo/env"` lo añade para la sesión actual).

### Sin instalar (desarrollo)

```bash
cargo run -p levi --locked
```

## Uso

```bash
levi                      # abre el directorio actual
levi /ruta/a/un/directorio
levi --theme nord --icons nerd
levi --keys               # muestra los atajos activos y sale
```

Otras opciones: `--editor` (fuerza el editor para la tecla `e`) y `--help`.

## Desarrollo

```bash
./scripts/dev.sh doctor   # comprueba el entorno
./scripts/dev.sh quick    # formato + clippy
./scripts/dev.sh check    # validación completa (incluye check de Windows)
```

## Compatibilidad con Windows

Las instrucciones de instalación para Windows se añadirán en una siguiente iteración.

## Licencia

MIT o Apache-2.0, a tu elección.
