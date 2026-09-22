// Backend modules — system-level logic for display, monitors, opacity and profile.
// Módulos de backend — lógica de sistema para pantalla, monitores, opacidad y perfil.

pub mod display;
pub mod hyprconf;
#[cfg(test)]
mod live_tests;
pub mod migrate;
pub mod monitors;
pub mod opacity;
pub mod profile;
pub mod update;
