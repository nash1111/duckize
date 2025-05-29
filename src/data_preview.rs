use std::path::Path;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Table, Row, Cell, TableState},
};
use csv::ReaderBuilder;
use calamine::{Reader, Xlsx, open_workbook};

#[derive(Debug, Clone)]
pub struct DataRange {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Clone)]
pub struct DataPreview {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub total_cols: usize,
    pub selection: DataRange,
    pub table_state: TableState,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub selecting: bool,
    pub selection_start: Option<(usize, usize)>,
}

impl DataPreview {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            total_rows: 0,
            total_cols: 0,
            selection: DataRange {
                start_row: 0,
                end_row: 0,
                start_col: 0,
                end_col: 0,
            },
            table_state: TableState::default(),
            cursor_row: 0,
            cursor_col: 0,
            selecting: false,
            selection_start: None,
        }
    }

    pub fn load_csv(&mut self, path: &Path, preview_rows: usize) -> color_eyre::Result<()> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)?;

        self.headers = reader.headers()?.iter()
            .map(|s| s.to_string())
            .collect();
        
        self.total_cols = self.headers.len();
        self.rows.clear();

        for (i, result) in reader.records().enumerate() {
            if i >= preview_rows {
                break;
            }
            let record = result?;
            let row: Vec<String> = record.iter()
                .map(|s| s.to_string())
                .collect();
            self.rows.push(row);
        }

        self.total_rows = self.rows.len();
        self.selection.end_row = self.total_rows.saturating_sub(1);
        self.selection.end_col = self.total_cols.saturating_sub(1);
        
        if !self.rows.is_empty() {
            self.table_state.select(Some(0));
        }

        Ok(())
    }

    pub fn load_excel(&mut self, path: &Path, sheet_name: Option<&str>, preview_rows: usize) -> color_eyre::Result<()> {
        let mut workbook: Xlsx<_> = open_workbook(path)?;
        
        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name.unwrap_or(&sheet_names[0]);
        
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let (rows, cols) = range.get_size();
            self.total_rows = rows;
            self.total_cols = cols;
            
            self.headers.clear();
            self.rows.clear();
            
            for (row_idx, row) in range.rows().enumerate() {
                if row_idx >= preview_rows + 1 {
                    break;
                }
                
                let row_data: Vec<String> = row.iter()
                    .map(|cell| cell.to_string())
                    .collect();
                
                if row_idx == 0 {
                    self.headers = row_data;
                } else {
                    self.rows.push(row_data);
                }
            }
            
            self.selection.end_row = self.total_rows.saturating_sub(1);
            self.selection.end_col = self.total_cols.saturating_sub(1);
            
            if !self.rows.is_empty() {
                self.table_state.select(Some(0));
            }
        }
        
        Ok(())
    }

    pub fn navigate_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.table_state.select(Some(self.cursor_row));
        }
    }

    pub fn navigate_down(&mut self) {
        if self.cursor_row < self.rows.len().saturating_sub(1) {
            self.cursor_row += 1;
            self.table_state.select(Some(self.cursor_row));
        }
    }

    pub fn navigate_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn navigate_right(&mut self) {
        if self.cursor_col < self.total_cols.saturating_sub(1) {
            self.cursor_col += 1;
        }
    }

    pub fn toggle_selection(&mut self) {
        if !self.selecting {
            self.selecting = true;
            self.selection_start = Some((self.cursor_row, self.cursor_col));
            self.selection = DataRange {
                start_row: self.cursor_row,
                end_row: self.cursor_row,
                start_col: self.cursor_col,
                end_col: self.cursor_col,
            };
        } else {
            self.selecting = false;
            if let Some((start_row, start_col)) = self.selection_start {
                self.selection = DataRange {
                    start_row: start_row.min(self.cursor_row),
                    end_row: start_row.max(self.cursor_row),
                    start_col: start_col.min(self.cursor_col),
                    end_col: start_col.max(self.cursor_col),
                };
            }
            self.selection_start = None;
        }
    }

    pub fn update_selection(&mut self) {
        if self.selecting {
            if let Some((start_row, start_col)) = self.selection_start {
                self.selection = DataRange {
                    start_row: start_row.min(self.cursor_row),
                    end_row: start_row.max(self.cursor_row),
                    start_col: start_col.min(self.cursor_col),
                    end_col: start_col.max(self.cursor_col),
                };
            }
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let header_cells: Vec<Cell> = self.headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let style = if i >= self.selection.start_col && i <= self.selection.end_col {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default().bold()
                };
                Cell::from(h.clone()).style(style)
            })
            .collect();

        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = self.rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| {
                let cells: Vec<Cell> = row.iter()
                    .enumerate()
                    .map(|(col_idx, cell)| {
                        let is_selected = row_idx >= self.selection.start_row 
                            && row_idx <= self.selection.end_row
                            && col_idx >= self.selection.start_col 
                            && col_idx <= self.selection.end_col;
                        
                        let is_cursor = row_idx == self.cursor_row && col_idx == self.cursor_col;
                        
                        let style = match (is_selected, is_cursor) {
                            (true, true) => Style::default().bg(Color::Blue).fg(Color::White),
                            (true, false) => Style::default().bg(Color::DarkGray),
                            (false, true) => Style::default().bg(Color::Gray),
                            _ => Style::default(),
                        };
                        
                        Cell::from(cell.clone()).style(style)
                    })
                    .collect();
                Row::new(cells).height(1)
            })
            .collect();

        let widths: Vec<Constraint> = self.headers
            .iter()
            .map(|_| Constraint::Length(15))
            .collect();

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default()
                .title(format!("Data Preview - Selection: ({},{}) to ({},{})", 
                    self.selection.start_row + 1, 
                    self.selection.start_col + 1,
                    self.selection.end_row + 1, 
                    self.selection.end_col + 1))
                .borders(Borders::ALL))
            .row_highlight_style(Style::default().bg(Color::Gray));

        StatefulWidget::render(table, area, buf, &mut self.table_state);
    }
}