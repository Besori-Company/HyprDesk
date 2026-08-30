// Entry point — initializes the app and hands off to the UI layer.
// Punto de entrada — inicializa la app y delega en la capa de UI.

mod backend;
mod config;
mod i18n;
mod ui;

fn main() -> iced::Result {
    ui::run()
}
