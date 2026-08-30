#!/bin/bash
# HyprDesk uninstaller
# HyprDesk desinstalador

set -e

BIN="$HOME/.local/bin"
DESK="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/scalable/apps"
HYPR_CONF="$HOME/.config/hypr/hyprland.conf"
HYPR_STARTUP="$HOME/.config/hypr/hyprdesk-startup.sh"

SYS_LANG="${LANG%%_*}"
msg() { [ "$SYS_LANG" = "es" ] && echo "$2" || echo "$1"; }
ask() { [ "$SYS_LANG" = "es" ] && read -p "$2 " "$3" || read -p "$1 " "$3"; }

echo "══════════════════════════════════════"
msg "  HyprDesk — uninstaller" "  HyprDesk — desinstalador"
echo "══════════════════════════════════════"
echo ""

ask "Uninstall HyprDesk? [y/N]" "¿Desinstalar HyprDesk? [s/N]" ans
if [[ ! "$ans" =~ ^[sSyY]$ ]]; then
    msg "Cancelled." "Cancelado."
    exit 0
fi

removed=0

# ── Binary and desktop entry / Binario y entrada de escritorio ───
[ -f "$BIN/hyprdesk" ]          && rm -f "$BIN/hyprdesk"          && msg "✓ Removed $BIN/hyprdesk" "✓ Eliminado $BIN/hyprdesk" && removed=1
[ -f "$DESK/hyprdesk.desktop" ] && rm -f "$DESK/hyprdesk.desktop" && msg "✓ Removed .desktop entry" "✓ Eliminado .desktop"      && removed=1
update-desktop-database "$DESK" 2>/dev/null || true

# ── Icons / Iconos ────────────────────────────────────────────
icon_count=0
for icon in "$ICONS"/hd-*-symbolic.svg; do
    [ -f "$icon" ] && rm -f "$icon" && icon_count=$((icon_count + 1))
done
if [ "$icon_count" -gt 0 ]; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
    msg "✓ Removed $icon_count icons" "✓ Eliminados $icon_count iconos"
    removed=1
fi

# ── Autostart / Inicio automático ─────────────────────────────
if [ -f "$HYPR_STARTUP" ] || ([ -f "$HYPR_CONF" ] && grep -q "hyprdesk-startup.sh" "$HYPR_CONF"); then
    ask "Remove autostart from Hyprland config? [y/N]" \
        "¿Eliminar autostart de la config de Hyprland? [s/N]" ans_auto
    if [[ "$ans_auto" =~ ^[sSyY]$ ]]; then
        [ -f "$HYPR_STARTUP" ] && rm -f "$HYPR_STARTUP" && \
            msg "✓ Removed $HYPR_STARTUP" "✓ Eliminado $HYPR_STARTUP"
        if [ -f "$HYPR_CONF" ] && grep -q "hyprdesk-startup.sh" "$HYPR_CONF"; then
            sed -i '/# HyprDesk autostart/d; /hyprdesk-startup\.sh/d' "$HYPR_CONF"
            msg "✓ Removed autostart line from hyprland.conf" \
                "✓ Eliminada línea de autostart de hyprland.conf"
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
