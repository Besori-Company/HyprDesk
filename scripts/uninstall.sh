#!/bin/bash
# HyprDesk uninstaller
# HyprDesk desinstalador

set -e

BIN="$HOME/.local/bin"
DESK="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"
LEGACY_ICONS="$HOME/.local/share/icons/hicolor/scalable/apps"
HYPR_CONF="$HOME/.config/hypr/hyprland.conf"
HYPR_STARTUP="$HOME/.config/hypr/hyprdesk-startup.sh"
HYPR_OPACITY="$HOME/.config/hypr/hyprdesk-opacity.conf"
HYPR_LUA="$HOME/.config/hypr/hyprland.lua"

SYS_LANG="${LANG%%_*}"
msg() { [ "$SYS_LANG" = "es" ] && echo "$2" || echo "$1"; }
# Answers come from the terminal, so `curl | bash` also works / Las respuestas vienen del terminal, para que `curl | bash` también funcione
if exec 3< /dev/tty 2>/dev/null; then HAS_TTY=1; else HAS_TTY=0; fi

ask() {
    local prompt
    if [ "$SYS_LANG" = "es" ]; then prompt="$2"; else prompt="$1"; fi
    if [ "$HAS_TTY" = 0 ]; then
        msg "No terminal available, nothing was removed." \
            "No hay terminal disponible, no se ha eliminado nada."
        exit 1
    fi
    # bash hides read's own prompt when stdin is a pipe, so it is printed here / bash oculta el prompt de read si la entrada es una tubería, así que se imprime aquí
    printf '%s ' "$prompt" > /dev/tty
    read -r -u 3 "$3"
}

echo "══════════════════════════════════════"
msg "  HyprDesk — uninstaller" "  HyprDesk — desinstalador"
echo "══════════════════════════════════════"
echo ""

ask "Uninstall HyprDesk? [y/N]" "¿Desinstalar HyprDesk? [s/N]" ans
if [[ ! "$ans" =~ ^[sSyY]$ ]]; then
    msg "Cancelled." "Cancelado."
    exit 0
fi

strip_block() {
    local file="$1" pattern="$2" tmp
    [ -f "$file" ] || return 0
    tmp="$(mktemp)"
    awk -v pat="$pattern" '
        $0 ~ pat { if (n > 0 && lines[n] ~ /^[[:space:]]*$/) n--; next }
        { lines[++n] = $0 }
        END { for (i = 1; i <= n; i++) print lines[i] }
    ' "$file" > "$tmp" && cat "$tmp" > "$file"
    rm -f "$tmp"
}

# Which package manager installed hyprdesk / Qué gestor de paquetes instaló hyprdesk
system_package() {
    if   command -v rpm    &>/dev/null && rpm -q hyprdesk        &>/dev/null; then echo "dnf"
    elif command -v dpkg   &>/dev/null && dpkg -s hyprdesk       &>/dev/null; then echo "apt"
    elif command -v pacman &>/dev/null && pacman -Qq hyprdesk    &>/dev/null; then echo "pacman"
    fi
}

removed=0

# ── Binary and desktop entry / Binario y entrada de escritorio ───
[ -f "$BIN/hyprdesk" ]          && rm -f "$BIN/hyprdesk"          && msg "✓ Removed $BIN/hyprdesk" "✓ Eliminado $BIN/hyprdesk" && removed=1
[ -f "$DESK/hyprdesk.desktop" ] && rm -f "$DESK/hyprdesk.desktop" && msg "✓ Removed .desktop entry" "✓ Eliminado .desktop"      && removed=1
update-desktop-database "$DESK" 2>/dev/null || true

# ── System package / Paquete del sistema ──────────────────────
PKG="$(system_package)"
if [ -n "$PKG" ]; then
    ask "HyprDesk is also installed as a system package. Remove it (needs sudo)? [y/N]" \
        "HyprDesk también está instalado como paquete del sistema. ¿Eliminarlo (necesita sudo)? [s/N]" ans_pkg
    if [[ "$ans_pkg" =~ ^[sSyY]$ ]]; then
        # A wrong password gets a second chance / Una contraseña equivocada tiene una segunda oportunidad
        for try in 1 2; do
            case "$PKG" in
                dnf)    sudo dnf remove -y hyprdesk || true ;;
                apt)    sudo apt-get remove -y hyprdesk || true ;;
                pacman) sudo pacman -R --noconfirm hyprdesk || true ;;
            esac
            [ -z "$(system_package)" ] && break
            [ "$try" = 1 ] && msg "That did not work. Trying once more:" \
                                  "No ha funcionado. Se intenta una vez más:"
        done
        if [ -n "$(system_package)" ]; then
            msg "! The system package could not be removed, try it by hand." \
                "! No se pudo eliminar el paquete del sistema, inténtalo a mano."
        else
            msg "✓ Removed the system package" "✓ Eliminado el paquete del sistema"
            removed=1
        fi
    fi
fi

# ── Icons / Iconos ────────────────────────────────────────────
icon_count=0
[ -f "$ICONS/hyprdesk.png" ] && rm -f "$ICONS/hyprdesk.png" && icon_count=$((icon_count + 1))
# Icons from older versions / Iconos de versiones anteriores
for icon in "$LEGACY_ICONS"/hd-*-symbolic.svg; do
    [ -f "$icon" ] && rm -f "$icon" && icon_count=$((icon_count + 1))
done
if [ "$icon_count" -gt 0 ]; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
    msg "✓ Removed $icon_count icons" "✓ Eliminados $icon_count iconos"
    removed=1
fi

# ── Autostart / Inicio automático ─────────────────────────────
if [ -f "$HYPR_STARTUP" ] || grep -qs "hyprdesk-startup.sh" "$HYPR_CONF" "$HYPR_LUA"; then
    ask "Remove autostart from Hyprland config? [y/N]" \
        "¿Eliminar autostart de la config de Hyprland? [s/N]" ans_auto
    if [[ "$ans_auto" =~ ^[sSyY]$ ]]; then
        [ -f "$HYPR_STARTUP" ] && rm -f "$HYPR_STARTUP" && \
            msg "✓ Removed $HYPR_STARTUP" "✓ Eliminado $HYPR_STARTUP"
        if grep -qs "hyprdesk-startup.sh" "$HYPR_CONF" "$HYPR_LUA"; then
            strip_block "$HYPR_CONF" "HyprDesk autostart|hyprdesk-startup\\.sh"
            strip_block "$HYPR_LUA"  "HyprDesk autostart|hyprdesk-startup\\.sh"
            msg "✓ Removed autostart line from the Hyprland config" \
                "✓ Eliminada la línea de autostart de la config de Hyprland"
        fi
        removed=1
    fi
fi

# ── Per-app opacity / Opacidad por app ────────────────────────
if [ -f "$HYPR_OPACITY" ] || grep -qs "hyprdesk-opacity" "$HYPR_CONF" "$HYPR_LUA"; then
    ask "Remove per-app opacity rules from Hyprland config? [y/N]" \
        "¿Eliminar las reglas de opacidad por app de la config de Hyprland? [s/N]" ans_op
    if [[ "$ans_op" =~ ^[sSyY]$ ]]; then
        rm -f "$HYPR_OPACITY" "$HOME/.config/hypr/hyprdesk-opacity.lua"
        msg "✓ Removed the opacity file" "✓ Eliminado el fichero de opacidad"
        if grep -qs "hyprdesk-opacity" "$HYPR_CONF" "$HYPR_LUA"; then
            strip_block "$HYPR_CONF" "HyprDesk per-app opacity|hyprdesk-opacity"
            strip_block "$HYPR_LUA"  "HyprDesk per-app opacity|hyprdesk-opacity"
            msg "✓ Removed opacity line from the Hyprland config" \
                "✓ Eliminada la línea de opacidad de la config de Hyprland"
        fi
        removed=1
    fi
fi

# ── Config and data / Configuración y datos ───────────────────
ask "Remove config and data (~/.config/hyprdesk)? [y/N]" \
    "¿Eliminar configuración (~/.config/hyprdesk)? [s/N]" ans_cfg
if [[ "$ans_cfg" =~ ^[sSyY]$ ]]; then
    rm -rf "$HOME/.config/hyprdesk"
    msg "✓ Removed ~/.config/hyprdesk" "✓ Eliminado ~/.config/hyprdesk"
fi

if [ "$removed" -eq 0 ]; then
    msg "No HyprDesk installation found." "No se encontró instalación de HyprDesk."
else
    echo ""
    echo "══════════════════════════════════════"
    msg "  Uninstallation complete" "  Desinstalación completada"
    echo "══════════════════════════════════════"
fi
echo ""
