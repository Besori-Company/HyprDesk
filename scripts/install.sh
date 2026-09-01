#!/bin/bash
# HyprDesk installer — downloads precompiled binary or builds from source.
# HyprDesk instalador — descarga el binario precompilado o compila desde el código fuente.

set -e

REPO="Besori-Company/HyprDesk"
BIN="$HOME/.local/bin"
DESK="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"

SYS_LANG="${LANG%%_*}"
msg() { [ "$SYS_LANG" = "es" ] && echo "$2" || echo "$1"; }

# ── Fetch latest version from GitHub API / Obtener la última versión ──
msg "Checking latest version..." "Comprobando la última versión..."
if command -v curl &>/dev/null; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' | cut -d'"' -f4)
elif command -v wget &>/dev/null; then
    VERSION=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' | cut -d'"' -f4)
fi

if [ -z "$VERSION" ]; then
    msg "Could not reach the GitHub API; installing the newest release anyway." \
        "No se pudo contactar con la API de GitHub; se instalará igualmente la última versión."
    VERSION=$(msg "latest" "última versión")
fi

echo "══════════════════════════════════════"
msg "  HyprDesk $VERSION — installer" "  HyprDesk $VERSION — instalador"
echo "══════════════════════════════════════"
echo ""

# ── Detect package manager / Detectar gestor de paquetes ──────
pkg_manager() {
    if   command -v pacman &>/dev/null; then echo "pacman"
    elif command -v apt    &>/dev/null; then echo "apt"
    elif command -v dnf    &>/dev/null; then echo "dnf"
    else echo "unknown"
    fi
}

# ── Install a package with the detected manager / Instala un paquete ──
install_pkg() {
    local pkg="$1"
    case "$PM" in
        pacman) sudo pacman -S --noconfirm --needed "$pkg" ;;
        apt)    sudo apt-get install -y "$pkg" ;;
        dnf)    sudo dnf install -y "$pkg" ;;
        *) msg "Install $pkg with your package manager." \
               "Instala $pkg con tu gestor de paquetes." ; return 1 ;;
    esac
}

# ── Download precompiled binary / Descargar binario ───────────
ARCH=$(uname -m)
# /latest/download/ redirects to the newest release, no API call and no rate limit / redirige a la última release, sin API ni límite de peticiones
BINARY_URL="https://github.com/$REPO/releases/latest/download/hyprdesk-linux-$ARCH"
BINARY=""
SRC_DIR=""
msg "Downloading HyprDesk $VERSION..." "Descargando HyprDesk $VERSION..."
TMP=$(mktemp)
if command -v curl &>/dev/null; then
    curl -fsSL "$BINARY_URL" -o "$TMP" 2>/dev/null && BINARY="$TMP" || true
elif command -v wget &>/dev/null; then
    wget -q "$BINARY_URL" -O "$TMP" 2>/dev/null && BINARY="$TMP" || true
fi

if [ -z "$BINARY" ] || [ ! -s "$BINARY" ]; then
    BINARY=""
    msg "No precompiled binary for $ARCH. Building from source..." \
        "No hay binario precompilado para $ARCH. Compilando desde el código fuente..."
fi

# ── Build from source / Compilar desde el código fuente ───────
if [ -z "$BINARY" ]; then
    if ! command -v cargo &>/dev/null; then
        source "$HOME/.cargo/env" 2>/dev/null || true
    fi

    # Piped through curl there is no source tree on disk, so reuse a local checkout or clone one / con curl | bash no hay código en disco: se reutiliza una copia local o se clona
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || echo ".")"
    if [ -f "$SCRIPT_DIR/../Cargo.toml" ] \
       && grep -q '^name = "hyprdesk"' "$SCRIPT_DIR/../Cargo.toml" 2>/dev/null; then
        SRC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    fi

    # Report every missing tool at once instead of one per run / informa de todo lo que falta de una vez, no de uno en uno
    MISSING=""
    command -v cargo &>/dev/null || MISSING="Rust/Cargo"
    if [ -z "$SRC_DIR" ] && ! command -v git &>/dev/null; then
        MISSING="${MISSING:+$MISSING, }git"
    fi

    if [ -n "$MISSING" ]; then
        echo ""
        msg "Cannot build automatically. Missing: $MISSING" \
            "No se puede compilar automáticamente. Falta: $MISSING"
        msg "Install what is missing and then run these commands by hand:" \
            "Instala lo que falte y luego ejecuta estos comandos a mano:"
        echo ""
        command -v cargo &>/dev/null || \
            echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        if [ -z "$SRC_DIR" ]; then
            echo "  git clone https://github.com/$REPO.git"
            echo "  cd HyprDesk"
        fi
        echo "  ./scripts/install.sh"
        echo ""
        rm -f "$TMP"
        exit 1
    fi

    if [ -z "$SRC_DIR" ]; then
        SRC_DIR=$(mktemp -d)
        trap 'rm -rf "$SRC_DIR"' EXIT
        msg "Cloning the source..." "Clonando el código fuente..."
        if ! git clone --depth 1 "https://github.com/$REPO.git" "$SRC_DIR" >/dev/null 2>&1; then
            msg "Could not clone the repository. Check your connection." \
                "No se pudo clonar el repositorio. Comprueba tu conexión."
            exit 1
        fi
    fi

    msg "Compiling (this takes a few minutes)..." "Compilando (esto tarda unos minutos)..."
    bash "$SRC_DIR/scripts/build.sh"
    BINARY="$SRC_DIR/build/hyprdesk"
fi

msg "✓ Binary ready" "✓ Binario listo"
echo ""

# ── Install dependencies / Instalar dependencias ─────────────
PM=$(pkg_manager)

_install_dep() {
    local label="$1" pkg="$2"
    msg "  Installing $label..." "  Instalando $label..."
    if install_pkg "$pkg"; then
        msg "  ✓ $label installed" "  ✓ $label instalado"
    else
        msg "  ! Could not install $label automatically. Install it manually." \
            "  ! No se pudo instalar $label automáticamente. Instálalo manualmente."
    fi
}

# brightnessctl — hardware backlight control / control de brillo por hardware
if ! command -v brightnessctl &>/dev/null; then
    _install_dep "brightnessctl" "brightnessctl"
fi

# night mode: hyprsunset on Arch, wlsunset elsewhere / modo noche: hyprsunset en Arch, wlsunset en el resto
if ! command -v hyprsunset &>/dev/null && ! command -v wlsunset &>/dev/null \
   && ! command -v gammastep &>/dev/null && ! command -v redshift &>/dev/null; then
    if [ "$PM" = "pacman" ]; then
        _install_dep "hyprsunset" "hyprsunset"
    else
        _install_dep "wlsunset" "wlsunset"
    fi
fi

# polkit (pkexec) — privilege escalation for profile settings / escalado de privilegios para el perfil
if ! command -v pkexec &>/dev/null; then
    _install_dep "polkit" "polkit"
fi

# gdbus (glib2) — AccountsService D-Bus for avatar and display name / para avatar y nombre de usuario
if ! command -v gdbus &>/dev/null; then
    case "$PM" in
        pacman) _install_dep "glib2" "glib2" ;;
        apt)    _install_dep "glib2 tools" "libglib2.0-bin" ;;
        dnf)    _install_dep "glib2" "glib2" ;;
        *) msg "Install glib2 with your package manager." "Instala glib2 con tu gestor de paquetes." ;;
    esac
fi

# accountsservice — daemon for avatar and display name via D-Bus / daemon para avatar y nombre de usuario
if ! systemctl list-unit-files 2>/dev/null | grep -q accounts-daemon; then
    _install_dep "accountsservice" "accountsservice"
fi

echo ""

# ── Install / Instalar ────────────────────────────────────────
mkdir -p "$BIN" "$DESK" "$ICONS"
# Unlink first: overwriting a running binary fails with "Text file busy" / desenlaza antes: sobrescribir un binario en ejecución da "Text file busy"
rm -f "$BIN/hyprdesk"
install -m 755 "$BINARY" "$BIN/hyprdesk"
rm -f "$TMP"
msg "✓ Installed to $BIN/hyprdesk" "✓ Instalado en $BIN/hyprdesk"

# Icon / Icono
LOCAL_ICON=""
[ -n "$SRC_DIR" ] && LOCAL_ICON="$SRC_DIR/hyprdesk/assets/icons/hyprdesk.png"
if [ -n "$LOCAL_ICON" ] && [ -f "$LOCAL_ICON" ]; then
    cp "$LOCAL_ICON" "$ICONS/hyprdesk.png"
else
    ICON_TMP=$(mktemp --suffix=.png)
    ICON_URL="https://raw.githubusercontent.com/$REPO/main/hyprdesk/assets/icons/hyprdesk.png"
    if command -v curl &>/dev/null; then
        curl -fsSL "$ICON_URL" -o "$ICON_TMP" 2>/dev/null || true
    elif command -v wget &>/dev/null; then
        wget -q "$ICON_URL" -O "$ICON_TMP" 2>/dev/null || true
    fi
    [ -s "$ICON_TMP" ] && cp "$ICON_TMP" "$ICONS/hyprdesk.png"
    rm -f "$ICON_TMP"
fi
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# .desktop entry / Entrada .desktop
cat > "$DESK/hyprdesk.desktop" << DESKTOP
[Desktop Entry]
Name=HyprDesk
GenericName=Control Panel
GenericName[es]=Panel de Control
Comment=Brightness, night mode and monitors for Hyprland
Comment[es]=Brillo, modo noche y monitores para Hyprland
Exec=$BIN/hyprdesk
Icon=hyprdesk
Terminal=false
Type=Application
Categories=Settings;HardwareSettings;
Keywords=brightness;night;profile;monitor;hyprland;opacity;
Keywords[es]=brillo;noche;perfil;monitor;hyprland;opacidad;
StartupNotify=true
StartupWMClass=hyprdesk
DESKTOP

update-desktop-database "$DESK" 2>/dev/null || true
msg "✓ Desktop entry registered" "✓ Entrada de escritorio registrada"

# ── PATH ──────────────────────────────────────────────────────
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    for RC in "$HOME/.zshrc" "$HOME/.bashrc"; do
        if [ -f "$RC" ] && ! grep -qF 'local/bin' "$RC" 2>/dev/null; then
            echo -e '\nexport PATH="$HOME/.local/bin:$PATH"' >> "$RC"
            msg "✓ PATH updated in $RC" "✓ PATH actualizado en $RC"
            break
        fi
    done
fi

echo ""
echo "══════════════════════════════════════"
msg "  Installation complete!" "  ¡Instalación completada!"
echo "══════════════════════════════════════"
echo ""
msg "  Run: hyprdesk" "  Ejecutar: hyprdesk"
echo ""
