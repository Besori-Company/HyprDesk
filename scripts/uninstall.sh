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
HYPR_OWN_CONF="$HOME/.config/hypr/hyprdesk.conf"
HYPR_OWN_LUA="$HOME/.config/hypr/hyprdesk.lua"

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
    if [ "$icon_count" -eq 1 ]; then
        msg "✓ Removed 1 icon" "✓ Eliminado 1 icono"
    else
        msg "✓ Removed $icon_count icons" "✓ Eliminados $icon_count iconos"
    fi
    removed=1
fi

# ── Hyprland settings / Ajustes de Hyprland ───────────────────
if [ -f "$HYPR_OWN_CONF" ] || [ -f "$HYPR_OWN_LUA" ] || [ -f "$HYPR_STARTUP" ] || [ -f "$HYPR_OPACITY" ] \
    || grep -qs "hyprdesk" "$HYPR_CONF" "$HYPR_LUA"; then
    ask "Remove HyprDesk's settings from Hyprland (monitors, opacity, autostart)? [y/N]" \
        "¿Eliminar los ajustes de HyprDesk de Hyprland (monitores, opacidad, autostart)? [s/N]" ans_hypr
    if [[ "$ans_hypr" =~ ^[sSyY]$ ]]; then
        # Includes go before their files, Lua reports a missing module as an error / Los include se quitan antes que sus ficheros, Lua da error si falta un módulo
        strip_block "$HYPR_CONF" '^[[:space:]]*# HyprDesk|^[[:space:]]*source[[:space:]]*=.*hyprdesk(-opacity)?[.]conf|^[[:space:]]*exec-once.*hyprdesk-startup[.]sh'
        strip_block "$HYPR_LUA"  '^[[:space:]]*-- HyprDesk|require.*hyprdesk|dofile.*hyprdesk-opacity|hl[.]keyword.*hyprdesk-startup[.]sh'
        rm -f "$HYPR_OWN_CONF" "$HYPR_OWN_LUA" "$HYPR_STARTUP" "$HYPR_OPACITY" "$HOME/.config/hypr/hyprdesk-opacity.lua"
        msg "✓ Removed HyprDesk's settings from the Hyprland config" \
            "✓ Eliminados los ajustes de HyprDesk de la config de Hyprland"
        if [ -d "$HOME/.config/hyprdesk/backups" ]; then
            msg "  Copies of the files HyprDesk migrated are in ~/.config/hyprdesk/backups" \
                "  Las copias de los ficheros que migró HyprDesk están en ~/.config/hyprdesk/backups"
        fi
        removed=1
    fi
fi

# ── Config and data / Configuración y datos ───────────────────
ask "Remove config, data and backups (~/.config/hyprdesk)? [y/N]" \
    "¿Eliminar configuración, datos y copias (~/.config/hyprdesk)? [s/N]" ans_cfg
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
