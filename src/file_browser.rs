use std::path::PathBuf;
use std::fs;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub extension: Option<String>,
}

#[derive(Clone)]
pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub state: ListState,
    pub filter_extensions: Vec<String>,
}

impl FileBrowser {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            state: ListState::default(),
            filter_extensions: vec!["csv".to_string(), "xlsx".to_string(), "xls".to_string()],
        };
        browser.refresh_entries();
        browser.state.select(Some(0));
        browser
    }

    pub fn refresh_entries(&mut self) {
        self.entries.clear();
        
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(FileEntry {
                path: parent.to_path_buf(),
                name: "..".to_string(),
                is_dir: true,
                extension: None,
            });
        }

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut files: Vec<FileEntry> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
                    let extension = if !is_dir {
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|s| s.to_lowercase())
                    } else {
                        None
                    };

                    if is_dir || extension.as_ref().map_or(false, |ext| {
                        self.filter_extensions.contains(ext)
                    }) {
                        Some(FileEntry {
                            path,
                            name,
                            is_dir,
                            extension,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            files.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });

            self.entries.extend(files);
        }

        if self.entries.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn navigate_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 {
                self.state.select(Some(i - 1));
            }
        }
    }

    pub fn navigate_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.entries.len().saturating_sub(1) {
                self.state.select(Some(i + 1));
            }
        }
    }

    pub fn enter_directory(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(entry) = self.entries.get(i) {
                if entry.is_dir {
                    self.current_dir = entry.path.clone();
                    self.refresh_entries();
                }
            }
        }
    }

    pub fn get_selected_file(&self) -> Option<&FileEntry> {
        self.state.selected()
            .and_then(|i| self.entries.get(i))
            .filter(|entry| !entry.is_dir)
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self.entries
            .iter()
            .map(|entry| {
                let icon = if entry.is_dir {
                    "📁 "
                } else {
                    match entry.extension.as_deref() {
                        Some("csv") => "📄 ",
                        Some("xlsx") | Some("xls") => "📊 ",
                        _ => "📄 ",
                    }
                };
                ListItem::new(format!("{}{}", icon, entry.name))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title(format!("File Browser - {}", self.current_dir.display()))
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray));

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}