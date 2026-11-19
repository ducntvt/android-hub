mod app;
mod appearance;
mod data_root;
mod i18n;
mod quit;

fn main() -> iced::Result {
    if std::env::args().nth(1).as_deref() == Some("--internal-uninstall") {
        if let Err(error) = android_hub_infrastructure::run_uninstall_helper() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    app::run()
}
