use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use egui::{Color32, RichText, Vec2};

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<DirEntry>,
    filtered_entries: Vec<usize>,
    selected_file: Option<usize>,
    error_message: Option<String>,
    search_query: String,
    clipboard: Option<(PathBuf, FileOperation)>,
    show_hidden: bool,
    sort_by: SortBy,
    view_mode: ViewMode,
    path_history: Vec<PathBuf>,
    history_index: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Name,
    Size,
    Modified,
    Type,
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    List,
    Details,
}

#[derive(Clone)]
enum FileOperation {
    Copy,
    Cut,
}

struct DirEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
    extension: String,
}

impl Default for FileExplorer {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mut explorer = Self {
            current_path: home.clone(),
            entries: Vec::new(),
            filtered_entries: Vec::new(),
            selected_file: None,
            error_message: None,
            search_query: String::new(),
            clipboard: None,
            show_hidden: false,
            sort_by: SortBy::Name,
            view_mode: ViewMode::Details,
            path_history: vec![home.clone()],
            history_index: 0,
        };
        explorer.load_directory(&home);
        explorer
    }
}

impl FileExplorer {
    fn load_directory(&mut self, path: &Path) {
        self.entries.clear();
        self.selected_file = None;
        self.error_message = None;

        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }

                    if let Ok(metadata) = entry.metadata() {
                        let extension = entry.path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();

                        self.entries.push(DirEntry {
                            name,
                            path: entry.path(),
                            is_dir: metadata.is_dir(),
                            size: metadata.len(),
                            modified: metadata.modified().ok(),
                            extension,
                        });
                    }
                }

                self.sort_entries();
                self.apply_filter();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to read directory: {}", e));
            }
        }
    }

    fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            match self.sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => b.size.cmp(&a.size),
                SortBy::Modified => b.modified.cmp(&a.modified),
                SortBy::Type => a.extension.to_lowercase().cmp(&b.extension.to_lowercase()),
            }
        });
    }

    fn apply_filter(&mut self) {
        self.filtered_entries.clear();

        if self.search_query.is_empty() {
            self.filtered_entries = (0..self.entries.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.name.to_lowercase().contains(&query) {
                    self.filtered_entries.push(i);
                }
            }
        }
    }

    fn navigate_to(&mut self, path: PathBuf) {
        if path == self.current_path {
            return;
        }

        self.current_path = path.clone();
        self.load_directory(&path);

        if self.history_index < self.path_history.len() - 1 {
            self.path_history.truncate(self.history_index + 1);
        }
        self.path_history.push(path);
        self.history_index = self.path_history.len() - 1;
    }

    fn go_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let path = self.path_history[self.history_index].clone();
            self.current_path = path.clone();
            self.load_directory(&path);
        }
    }

    fn go_forward(&mut self) {
        if self.history_index < self.path_history.len() - 1 {
            self.history_index += 1;
            let path = self.path_history[self.history_index].clone();
            self.current_path = path.clone();
            self.load_directory(&path);
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
        }
    }

    fn copy_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(entry_idx) = self.filtered_entries.get(idx) {
                if let Some(entry) = self.entries.get(*entry_idx) {
                    self.clipboard = Some((entry.path.clone(), FileOperation::Copy));
                }
            }
        }
    }

    fn cut_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(entry_idx) = self.filtered_entries.get(idx) {
                if let Some(entry) = self.entries.get(*entry_idx) {
                    self.clipboard = Some((entry.path.clone(), FileOperation::Cut));
                }
            }
        }
    }

    fn paste_file(&mut self) {
        if let Some((source, operation)) = &self.clipboard {
            let file_name = source.file_name().unwrap();
            let dest = self.current_path.join(file_name);

            match operation {
                FileOperation::Copy => {
                    if source.is_file() {
                        let _ = fs::copy(source, &dest);
                    } else if source.is_dir() {
                        let _ = self.copy_dir_all(source, &dest);
                    }
                }
                FileOperation::Cut => {
                    let _ = fs::rename(source, &dest);
                    self.clipboard = None;
                }
            }

            self.load_directory(&self.current_path.clone());
        }
    }

    fn copy_dir_all(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                self.copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    fn delete_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(entry_idx) = self.filtered_entries.get(idx) {
                if let Some(entry) = self.entries.get(*entry_idx) {
                    let result = if entry.is_dir {
                        fs::remove_dir_all(&entry.path)
                    } else {
                        fs::remove_file(&entry.path)
                    };

                    if let Err(e) = result {
                        self.error_message = Some(format!("Failed to delete: {}", e));
                    } else {
                        self.selected_file = None;
                        self.load_directory(&self.current_path.clone());
                    }
                }
            }
        }
    }

    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    fn format_time(time: Option<SystemTime>) -> String {
        time.and_then(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .map(|d| {
                    let secs = d.as_secs();
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let diff = now.saturating_sub(secs);

                    if diff < 60 {
                        "Just now".to_string()
                    } else if diff < 3600 {
                        format!("{} min ago", diff / 60)
                    } else if diff < 86400 {
                        format!("{} hours ago", diff / 3600)
                    } else if diff < 604800 {
                        format!("{} days ago", diff / 86400)
                    } else {
                        format!("{} weeks ago", diff / 604800)
                    }
                })
        })
        .unwrap_or_else(|| "Unknown".to_string())
    }

    fn get_file_icon(entry: &DirEntry) -> &str {
        if entry.is_dir {
            return "📁";
        }

        match entry.extension.to_lowercase().as_str() {
            "rs" => "🦀",
            "toml" => "⚙️",
            "md" => "📝",
            "txt" => "📄",
            "pdf" => "📕",
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "bmp" => "🖼️",
            "mp3" | "wav" | "ogg" | "flac" => "🎵",
            "mp4" | "avi" | "mkv" | "mov" => "🎬",
            "zip" | "tar" | "gz" | "7z" | "rar" => "📦",
            "js" | "ts" | "jsx" | "tsx" => "🟨",
            "py" => "🐍",
            "java" => "☕",
            "cpp" | "c" | "h" => "⚡",
            "html" | "css" => "🌐",
            "json" | "xml" | "yaml" | "yml" => "📋",
            _ => "📄",
        }
    }

    fn render_breadcrumbs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Path:").color(Color32::from_rgb(100, 100, 100)));

            let mut components: Vec<PathBuf> = Vec::new();
            let mut current = self.current_path.as_path();

            components.push(current.to_path_buf());
            while let Some(parent) = current.parent() {
                if parent.as_os_str().is_empty() {
                    break;
                }
                components.push(parent.to_path_buf());
                current = parent;
            }

            components.reverse();

            for (i, component) in components.iter().enumerate() {
                if i > 0 {
                    ui.label(RichText::new("/").color(Color32::from_rgb(150, 150, 150)));
                }

                let name = component
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| component.to_str().unwrap_or(""));

                if ui.link(name).clicked() {
                    self.navigate_to(component.clone());
                }
            }
        });
    }
}

impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                self.load_directory(&self.current_path.clone());
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                self.copy_file();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X) {
                self.cut_file();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                self.paste_file();
            }
            if i.key_pressed(egui::Key::Delete) {
                self.delete_file();
            }
            if i.key_pressed(egui::Key::Backspace) || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) {
                self.go_back();
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) {
                self.go_forward();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                self.show_hidden = !self.show_hidden;
                self.load_directory(&self.current_path.clone());
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let back_enabled = self.history_index > 0;
                let forward_enabled = self.history_index < self.path_history.len() - 1;

                ui.add_enabled_ui(back_enabled, |ui| {
                    if ui.button(RichText::new("◀").size(16.0)).clicked() {
                        self.go_back();
                    }
                });

                ui.add_enabled_ui(forward_enabled, |ui| {
                    if ui.button(RichText::new("▶").size(16.0)).clicked() {
                        self.go_forward();
                    }
                });

                if ui.button(RichText::new("⬆").size(16.0)).clicked() {
                    self.go_up();
                }

                if ui.button(RichText::new("🏠").size(16.0)).clicked() {
                    if let Some(home) = dirs::home_dir() {
                        self.navigate_to(home);
                    }
                }

                if ui.button(RichText::new("🔄").size(16.0)).clicked() {
                    self.load_directory(&self.current_path.clone());
                }

                ui.separator();

                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Search files...")
                        .desired_width(200.0)
                );

                if search_response.changed() {
                    self.apply_filter();
                }

                ui.separator();

                if ui.button(if self.show_hidden { "👁 Show Hidden" } else { "👁‍🗨 Hide Hidden" }).clicked() {
                    self.show_hidden = !self.show_hidden;
                    self.load_directory(&self.current_path.clone());
                }
            });

            ui.add_space(4.0);
            self.render_breadcrumbs(ui);
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sort by:").color(Color32::from_rgb(100, 100, 100)));

                if ui.selectable_label(self.sort_by == SortBy::Name, "Name").clicked() {
                    self.sort_by = SortBy::Name;
                    self.sort_entries();
                    self.apply_filter();
                }
                if ui.selectable_label(self.sort_by == SortBy::Size, "Size").clicked() {
                    self.sort_by = SortBy::Size;
                    self.sort_entries();
                    self.apply_filter();
                }
                if ui.selectable_label(self.sort_by == SortBy::Modified, "Modified").clicked() {
                    self.sort_by = SortBy::Modified;
                    self.sort_entries();
                    self.apply_filter();
                }
                if ui.selectable_label(self.sort_by == SortBy::Type, "Type").clicked() {
                    self.sort_by = SortBy::Type;
                    self.sort_entries();
                    self.apply_filter();
                }

                ui.separator();

                ui.label(RichText::new("View:").color(Color32::from_rgb(100, 100, 100)));
                if ui.selectable_label(self.view_mode == ViewMode::List, "List").clicked() {
                    self.view_mode = ViewMode::List;
                }
                if ui.selectable_label(self.view_mode == ViewMode::Details, "Details").clicked() {
                    self.view_mode = ViewMode::Details;
                }
            });
            ui.separator();
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error_message {
                ui.colored_label(Color32::RED, RichText::new(error).strong());
                ui.add_space(8.0);
            }

            if self.filtered_entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(RichText::new("No files found").size(18.0).color(Color32::from_rgb(150, 150, 150)));
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.view_mode {
                    ViewMode::List => {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 1.0);

                        let mut navigate_to_path: Option<PathBuf> = None;

                        for (i, &entry_idx) in self.filtered_entries.iter().enumerate() {
                            if let Some(entry) = self.entries.get(entry_idx) {
                                let is_selected = self.selected_file == Some(i);

                                let response = ui.selectable_label(
                                    is_selected,
                                    RichText::new(format!(
                                        "{} {}",
                                        Self::get_file_icon(entry),
                                        entry.name
                                    )).size(14.0),
                                );

                                if response.clicked() {
                                    self.selected_file = Some(i);
                                }

                                if response.double_clicked() {
                                    if entry.is_dir {
                                        navigate_to_path = Some(entry.path.clone());
                                    } else {
                                        #[cfg(target_os = "windows")]
                                        {
                                            let path = entry.path.to_str().unwrap();
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "start", "", path])
                                                .spawn();
                                        }

                                        #[cfg(target_os = "macos")]
                                        {
                                            let _ = std::process::Command::new("open").arg(&entry.path).spawn();
                                        }

                                        #[cfg(target_os = "linux")]
                                        {
                                            let _ = std::process::Command::new("xdg-open")
                                                .arg(&entry.path)
                                                .spawn();
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(path) = navigate_to_path {
                            self.navigate_to(path);
                        }
                    }
                    ViewMode::Details => {
                        use egui_extras::{Column, TableBuilder};

                        TableBuilder::new(ui)
                            .striped(true)
                            .column(Column::auto().at_least(300.0))
                            .column(Column::auto().at_least(80.0))
                            .column(Column::auto().at_least(100.0))
                            .column(Column::auto().at_least(80.0))
                            .header(24.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Name");
                                });
                                header.col(|ui| {
                                    ui.strong("Size");
                                });
                                header.col(|ui| {
                                    ui.strong("Modified");
                                });
                                header.col(|ui| {
                                    ui.strong("Type");
                                });
                            })
                            .body(|mut body| {
                                let mut navigate_to_path: Option<PathBuf> = None;

                                for (i, &entry_idx) in self.filtered_entries.iter().enumerate() {
                                    if let Some(entry) = self.entries.get(entry_idx) {
                                        let is_selected = self.selected_file == Some(i);

                                        body.row(22.0, |mut row| {
                                            row.col(|ui| {
                                                let response = ui.selectable_label(
                                                    is_selected,
                                                    RichText::new(format!("{} {}", Self::get_file_icon(entry), entry.name)),
                                                );

                                                if response.clicked() {
                                                    self.selected_file = Some(i);
                                                }

                                                if response.double_clicked() {
                                                    if entry.is_dir {
                                                        navigate_to_path = Some(entry.path.clone());
                                                    } else {
                                                        #[cfg(target_os = "windows")]
                                                        {
                                                            let path = entry.path.to_str().unwrap();
                                                            let _ = std::process::Command::new("cmd")
                                                                .args(["/C", "start", "", path])
                                                                .spawn();
                                                        }

                                                        #[cfg(target_os = "macos")]
                                                        {
                                                            let _ = std::process::Command::new("open").arg(&entry.path).spawn();
                                                        }

                                                        #[cfg(target_os = "linux")]
                                                        {
                                                            let _ = std::process::Command::new("xdg-open")
                                                                .arg(&entry.path)
                                                                .spawn();
                                                        }
                                                    }
                                                }
                                            });

                                            row.col(|ui| {
                                                if !entry.is_dir {
                                                    ui.label(RichText::new(Self::format_size(entry.size)).color(Color32::from_rgb(120, 120, 120)));
                                                }
                                            });

                                            row.col(|ui| {
                                                ui.label(RichText::new(Self::format_time(entry.modified)).color(Color32::from_rgb(120, 120, 120)));
                                            });

                                            row.col(|ui| {
                                                let type_str = if entry.is_dir {
                                                    "Folder"
                                                } else if entry.extension.is_empty() {
                                                    "File"
                                                } else {
                                                    &entry.extension
                                                };
                                                ui.label(RichText::new(type_str).color(Color32::from_rgb(120, 120, 120)));
                                            });
                                        });
                                    }
                                }

                                if let Some(path) = navigate_to_path {
                                    self.navigate_to(path);
                                }
                            });
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} items", self.filtered_entries.len())).color(Color32::from_rgb(100, 100, 100)));

                if let Some(selected_idx) = self.selected_file {
                    if let Some(&entry_idx) = self.filtered_entries.get(selected_idx) {
                        if let Some(entry) = self.entries.get(entry_idx) {
                            ui.separator();
                            ui.label(RichText::new(format!("Selected: {}", entry.name)).color(Color32::from_rgb(80, 80, 200)));

                            if !entry.is_dir {
                                ui.separator();
                                ui.label(RichText::new(Self::format_size(entry.size)).color(Color32::from_rgb(100, 100, 100)));
                            }
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Ctrl+C: Copy | Ctrl+X: Cut | Ctrl+V: Paste | Del: Delete | F5: Refresh").color(Color32::from_rgb(120, 120, 120)).size(11.0));
                });
            });
            ui.add_space(4.0);
        });
    }
}
