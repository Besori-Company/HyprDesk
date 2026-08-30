// Entry point — answers terminal flags, then initializes the app and hands off to the UI layer.
// Punto de entrada — responde a las opciones de terminal, inicializa la app y delega en la capa de UI.

mod backend;
mod config;
mod i18n;
mod ui;

const VERSION: &str = concat!("HyprDesk v", env!("CARGO_PKG_VERSION"));

fn main() -> iced::Result {
    if let Some(arg) = std::env::args().nth(1) {
        // Help follows the language configured in the app / La ayuda sigue el idioma configurado en la app
        i18n::set_lang(&config::load_config().app_lang);
        match arg.as_str() {
            "-V" | "--version" => println!("{VERSION}"),
            "-h" | "--help" => println!("{}", help()),
            other => {
                eprintln!("{}", i18n::t("cli_unknown").replace("{}", other));
                eprintln!("{}", help());
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    ui::run()
}

fn help() -> String {
    use i18n::t;
    format!(
        "{VERSION}\n{}\n\n{}\n\n  -V, --version   {}\n  -h, --help      {}\n\n{}",
        t("cli_description"),
        t("cli_usage"),
        t("cli_opt_version"),
        t("cli_opt_help"),
        t("cli_no_args"),
    )
}
