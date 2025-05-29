use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Table, Row, Paragraph},
};
use crate::duckdb_manager::DuckDBManager;
use duckdb::Result as DuckDBResult;

#[derive(Clone)]
pub struct TableViewer {
    pub tables: Vec<String>,
    pub table_list_state: ListState,
    pub selected_table: Option<String>,
    pub schema: Vec<(String, String)>,
    pub data_preview: Vec<Vec<String>>,
    pub data_scroll_offset: usize,
}

impl TableViewer {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            table_list_state: ListState::default(),
            selected_table: None,
            schema: Vec::new(),
            data_preview: Vec::new(),
            data_scroll_offset: 0,
        }
    }

    pub fn refresh_tables(&mut self, duckdb: &DuckDBManager) -> DuckDBResult<()> {
        self.tables = duckdb.list_tables()?;
        if !self.tables.is_empty() && self.table_list_state.selected().is_none() {
            self.table_list_state.select(Some(0));
        }
        Ok(())
    }

    pub fn navigate_up(&mut self) {
        if let Some(i) = self.table_list_state.selected() {
            if i > 0 {
                self.table_list_state.select(Some(i - 1));
            }
        }
    }

    pub fn navigate_down(&mut self) {
        if let Some(i) = self.table_list_state.selected() {
            if i < self.tables.len().saturating_sub(1) {
                self.table_list_state.select(Some(i + 1));
            }
        }
    }

    pub fn scroll_data_up(&mut self) {
        if self.data_scroll_offset > 0 {
            self.data_scroll_offset -= 1;
        }
    }

    pub fn scroll_data_down(&mut self) {
        if self.data_scroll_offset < self.data_preview.len().saturating_sub(10) {
            self.data_scroll_offset += 1;
        }
    }

    pub fn select_table(&mut self, duckdb: &DuckDBManager) -> DuckDBResult<()> {
        // Early return if no tables are available
        if self.tables.is_empty() {
            return Ok(());
        }
        
        if let Some(i) = self.table_list_state.selected() {
            if let Some(table_name) = self.tables.get(i) {
                self.selected_table = Some(table_name.clone());
                
                // Try to get schema, handle errors gracefully
                match duckdb.get_table_schema(table_name) {
                    Ok(schema) => self.schema = schema,
                    Err(_) => {
                        self.schema = Vec::new();
                        self.data_preview = Vec::new();
                        return Ok(()); // Return early if we can't get schema
                    }
                }
                
                // Get preview data (first 100 rows)
                // Properly escape table name for SQL query
                let escaped_table_name = table_name.replace("\"", "\"\"");
                let query = format!("SELECT * FROM \"{}\" LIMIT 100", escaped_table_name);
                match duckdb.execute_query(&query) {
                    Ok(data) => self.data_preview = data,
                    Err(_) => self.data_preview = Vec::new(),
                }
                self.data_scroll_offset = 0;
            }
        }
        Ok(())
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(area);

        self.render_table_list(chunks[0], buf);
        self.render_table_details(chunks[1], buf);
    }

    fn render_table_list(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self.tables
            .iter()
            .map(|table| ListItem::new(format!("📊 {}", table)))
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("Tables")
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray));

        StatefulWidget::render(list, area, buf, &mut self.table_list_state);
    }

    fn render_table_details(&mut self, area: Rect, buf: &mut Buffer) {
        if let Some(table_name) = self.selected_table.clone() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(self.schema.len() as u16 + 3),
                    Constraint::Min(0),
                ])
                .split(area);

            // Render schema
            self.render_schema(chunks[0], buf, &table_name);

            // Render data preview
            self.render_data_preview(chunks[1], buf);
        } else {
            let block = Block::default()
                .title("Table Details")
                .borders(Borders::ALL);
            
            let text = "Select a table to view details";
            let paragraph = Paragraph::new(text)
                .block(block)
                .centered();
            
            paragraph.render(area, buf);
        }
    }

    fn render_schema(&self, area: Rect, buf: &mut Buffer, table_name: &str) {
        let header = Row::new(vec!["Column", "Type"])
            .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self.schema
            .iter()
            .map(|(col_name, col_type)| {
                Row::new(vec![col_name.clone(), col_type.clone()])
            })
            .collect();

        let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
            .header(header)
            .block(Block::default()
                .title(format!("Schema: {}", table_name))
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        Widget::render(table, area, buf);
    }

    fn render_data_preview(&self, area: Rect, buf: &mut Buffer) {
        if self.data_preview.is_empty() {
            let block = Block::default()
                .title("Data Preview")
                .borders(Borders::ALL);
            
            let text = "No data to display";
            let paragraph = Paragraph::new(text)
                .block(block)
                .centered();
            
            paragraph.render(area, buf);
            return;
        }

        // Get column headers from schema
        let headers: Vec<String> = self.schema.iter().map(|(name, _)| name.clone()).collect();
        let header_row = Row::new(headers).style(Style::default().add_modifier(Modifier::BOLD));

        // Create data rows with scrolling
        let visible_rows = self.data_preview
            .iter()
            .skip(self.data_scroll_offset)
            .take(area.height.saturating_sub(3) as usize)
            .map(|row| {
                Row::new(row.iter().map(|cell| cell.clone()).collect::<Vec<String>>())
            })
            .collect::<Vec<Row>>();

        let widths: Vec<Constraint> = self.schema
            .iter()
            .map(|_| Constraint::Percentage(100 / self.schema.len() as u16))
            .collect();

        let visible_count = visible_rows.len();
        let table = Table::new(visible_rows, widths)
            .header(header_row)
            .block(Block::default()
                .title(format!("Data Preview (showing {} of {} rows)", 
                    visible_count, 
                    self.data_preview.len()))
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        Widget::render(table, area, buf);
    }
}