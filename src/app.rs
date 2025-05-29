use crate::event::{AppEvent, Event, EventHandler};
use crate::file_browser::FileBrowser;
use crate::data_preview::DataPreview;
use crate::duckdb_manager::DuckDBManager;
use crate::table_viewer::TableViewer;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::ListState,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    FileBrowser,
    DataPreview,
    TableImport,
    QueryMode,
    TableViewer,
}

/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// Current mode
    pub mode: AppMode,
    /// File browser
    pub file_browser: FileBrowser,
    /// Data preview
    pub data_preview: DataPreview,
    /// DuckDB manager
    pub duckdb: DuckDBManager,
    /// Table viewer
    pub table_viewer: TableViewer,
    /// Selected file path
    pub selected_file: Option<PathBuf>,
    /// Table name input
    pub table_name: String,
    /// Status message
    pub status_message: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            events: EventHandler::new(),
            mode: AppMode::FileBrowser,
            file_browser: FileBrowser::new(),
            data_preview: DataPreview::new(),
            duckdb: DuckDBManager::new().expect("Failed to initialize DuckDB"),
            table_viewer: TableViewer::new(),
            selected_file: None,
            table_name: String::new(),
            status_message: "Welcome to Duckize! Navigate files with arrows, Enter to select. Press Tab to switch views.".to_string(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match self.mode {
            AppMode::FileBrowser => self.handle_file_browser_keys(key_event)?,
            AppMode::DataPreview => self.handle_data_preview_keys(key_event)?,
            AppMode::TableImport => self.handle_table_import_keys(key_event)?,
            AppMode::QueryMode => self.handle_query_mode_keys(key_event)?,
            AppMode::TableViewer => self.handle_table_viewer_keys(key_event)?,
        }
        Ok(())
    }

    fn handle_file_browser_keys(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Up => self.file_browser.navigate_up(),
            KeyCode::Down => self.file_browser.navigate_down(),
            KeyCode::Tab => {
                self.mode = AppMode::TableViewer;
                // Refresh tables and handle potential errors
                match self.table_viewer.refresh_tables(&self.duckdb) {
                    Ok(_) => {
                        if self.table_viewer.tables.is_empty() {
                            self.status_message = "Table Viewer: No tables loaded. Import CSV/Excel files first.".to_string();
                        } else {
                            self.status_message = "Table Viewer: Use arrows to navigate, Enter to view table details".to_string();
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Error loading tables: {}", e);
                        // Still switch to table viewer mode but with empty table list
                        self.table_viewer.tables.clear();
                        self.table_viewer.table_list_state = ListState::default();
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(file) = self.file_browser.get_selected_file() {
                    self.selected_file = Some(file.path.clone());
                    self.load_file_preview()?;
                    self.mode = AppMode::DataPreview;
                    self.status_message = "Use arrows to navigate, Space to select range, Enter to import".to_string();
                } else {
                    self.file_browser.enter_directory();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_data_preview_keys(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = AppMode::FileBrowser;
                self.status_message = "Back to file browser".to_string();
            }
            KeyCode::Up => {
                self.data_preview.navigate_up();
                self.data_preview.update_selection();
            }
            KeyCode::Down => {
                self.data_preview.navigate_down();
                self.data_preview.update_selection();
            }
            KeyCode::Left => {
                self.data_preview.navigate_left();
                self.data_preview.update_selection();
            }
            KeyCode::Right => {
                self.data_preview.navigate_right();
                self.data_preview.update_selection();
            }
            KeyCode::Char(' ') => {
                self.data_preview.toggle_selection();
            }
            KeyCode::Enter => {
                self.mode = AppMode::TableImport;
                self.table_name.clear();
                self.status_message = "Enter table name and press Enter to import".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_table_import_keys(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = AppMode::DataPreview;
                self.status_message = "Import cancelled".to_string();
            }
            KeyCode::Enter => {
                if !self.table_name.is_empty() {
                    self.import_to_duckdb()?;
                    self.mode = AppMode::FileBrowser;
                }
            }
            KeyCode::Char(c) => {
                self.table_name.push(c);
            }
            KeyCode::Backspace => {
                self.table_name.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_query_mode_keys(&mut self, _key_event: KeyEvent) -> color_eyre::Result<()> {
        // TODO: Implement query mode
        Ok(())
    }

    fn handle_table_viewer_keys(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = AppMode::FileBrowser;
                self.status_message = "Back to file browser. Press Tab to view tables.".to_string();
            }
            KeyCode::Tab => {
                self.mode = AppMode::FileBrowser;
                self.status_message = "Back to file browser. Press Tab to view tables.".to_string();
            }
            KeyCode::Up => {
                self.table_viewer.navigate_up();
            }
            KeyCode::Down => {
                self.table_viewer.navigate_down();
            }
            KeyCode::Enter => {
                self.table_viewer.select_table(&self.duckdb)?;
                if let Some(table_name) = &self.table_viewer.selected_table {
                    self.status_message = format!("Viewing table: {}. Use PageUp/PageDown to scroll data.", table_name);
                }
            }
            KeyCode::PageUp => {
                self.table_viewer.scroll_data_up();
            }
            KeyCode::PageDown => {
                self.table_viewer.scroll_data_down();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    fn load_file_preview(&mut self) -> color_eyre::Result<()> {
        if let Some(path) = &self.selected_file {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
                
            match ext.as_deref() {
                Some("csv") => {
                    self.data_preview.load_csv(path, 100)?;
                    self.status_message = format!("Loaded CSV: {} rows × {} columns", 
                        self.data_preview.total_rows, 
                        self.data_preview.total_cols);
                }
                Some("xlsx") | Some("xls") => {
                    self.data_preview.load_excel(path, None, 100)?;
                    self.status_message = format!("Loaded Excel: {} rows × {} columns", 
                        self.data_preview.total_rows, 
                        self.data_preview.total_cols);
                }
                _ => {
                    self.status_message = "Unsupported file type".to_string();
                }
            }
        }
        Ok(())
    }

    fn import_to_duckdb(&mut self) -> color_eyre::Result<()> {
        if let Some(path) = &self.selected_file {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
                
            let result = match ext.as_deref() {
                Some("csv") => {
                    self.duckdb.import_csv(
                        path, 
                        &self.table_name, 
                        &self.data_preview.selection,
                        true
                    )
                }
                Some("xlsx") | Some("xls") => {
                    self.duckdb.import_excel(
                        path,
                        &self.table_name,
                        "Sheet1", // TODO: Allow sheet selection
                        &self.data_preview.selection,
                        true
                    )
                }
                _ => {
                    return Ok(());
                }
            };
            
            match result {
                Ok(_) => {
                    self.status_message = format!("Successfully imported table: {}. Press Tab to view tables.", self.table_name);
                    // Refresh table list
                    self.table_viewer.refresh_tables(&self.duckdb).ok();
                }
                Err(e) => {
                    self.status_message = format!("Import failed: {}", e);
                }
            }
        }
        Ok(())
    }
}
