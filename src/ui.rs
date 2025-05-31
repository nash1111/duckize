use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use crate::app::{App, AppMode};

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        match self.mode {
            AppMode::DatabaseSelect => {
                self.render_database_select(chunks[0], buf);
            }
            AppMode::FileBrowser => {
                self.render_file_browser(chunks[0], buf);
            }
            AppMode::DataPreview => {
                self.render_data_preview(chunks[0], buf);
            }
            AppMode::TableImport => {
                self.render_table_import(chunks[0], buf);
            }
            AppMode::QueryMode => {
                self.render_query_mode(chunks[0], buf);
            }
            AppMode::TableViewer => {
                self.render_table_viewer(chunks[0], buf);
            }
        }

        self.render_status_bar(chunks[1], buf);
    }
}

impl App {
    fn render_database_select(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Database Selection")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let input_text = format!("Database file: {}_", self.database_path);
        let input = Paragraph::new(input_text)
            .block(block)
            .style(Style::default().fg(Color::Yellow))
            .centered();

        input.render(area, buf);
    }

    fn render_file_browser(&self, area: Rect, buf: &mut Buffer) {
        let mut file_browser = self.file_browser.clone();
        file_browser.render(area, buf);
    }

    fn render_data_preview(&self, area: Rect, buf: &mut Buffer) {
        let mut data_preview = self.data_preview.clone();
        data_preview.render(area, buf);
    }

    fn render_table_import(&self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(5),
            ])
            .split(area);

        let mut data_preview = self.data_preview.clone();
        data_preview.render(chunks[0], buf);

        let input_block = Block::default()
            .title("Import to DuckDB")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let input_text = format!("Table name: {}", self.table_name);
        let input = Paragraph::new(input_text)
            .block(input_block)
            .style(Style::default().fg(Color::Yellow));

        input.render(chunks[1], buf);
    }

    fn render_query_mode(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Query Mode")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let text = "Query mode coming soon...";
        let paragraph = Paragraph::new(text)
            .block(block)
            .centered();

        paragraph.render(area, buf);
    }

    fn render_table_viewer(&self, area: Rect, buf: &mut Buffer) {
        let mut table_viewer = self.table_viewer.clone();
        table_viewer.render(area, buf);
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let status_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain);

        let mode_text = match self.mode {
            AppMode::DatabaseSelect => "Database Select",
            AppMode::FileBrowser => "File Browser",
            AppMode::DataPreview => "Data Preview",
            AppMode::TableImport => "Table Import",
            AppMode::QueryMode => "Query Mode",
            AppMode::TableViewer => "Table Viewer",
        };

        let status_text = format!("[{}] {}", mode_text, self.status_message);
        let status = Paragraph::new(status_text)
            .block(status_block)
            .style(Style::default().fg(Color::Cyan));

        status.render(area, buf);
    }
}