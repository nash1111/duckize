use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;
pub mod file_browser;
pub mod data_preview;
pub mod duckdb_manager;
pub mod table_viewer;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
    ratatui::restore();
    result
}
