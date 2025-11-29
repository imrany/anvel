use std::fs;
use std::path::{Path, PathBuf};

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<DirEntry>,
    selected_file: Option<usize>,
    error_message: Option<String>,
}

struct DirEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

impl Default for FileExplorer {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mut explorer = Self {
            current_path: home.clone(),
            entries: Vec::new(),
            selected_file: None,
            error_message: None,
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
                    if let Ok(metadata) = entry.metadata() {
                        self.entries.push(DirEntry {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path(),
                            is_dir: metadata.is_dir(),
                            size: metadata.len(),
                        });
                    }
                }

                // Sort: directories first, then files
                self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to read directory: {}", e));
            }
        }
    }

    fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path.clone();
        self.load_directory(&path);
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
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

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⬆ Up").clicked() {
                    self.go_up();
                }

                if ui.button("🏠 Home").clicked() {
                    if let Some(home) = dirs::home_dir() {
                        self.navigate_to(home);
                    }
                }

                ui.separator();

                ui.label("Path:");
                ui.label(self.current_path.display().to_string());
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);

                let mut navigate_to_path: Option<PathBuf> = None;

                for (i, entry) in self.entries.iter().enumerate() {
                    let is_selected = self.selected_file == Some(i);

                    let response = ui.selectable_label(
                        is_selected,
                        format!(
                            "{} {}  {}",
                            if entry.is_dir { "📁" } else { "📄" },
                            entry.name,
                            if entry.is_dir {
                                String::new()
                            } else {
                                format!("({})", Self::format_size(entry.size))
                            }
                        ),
                    );

                    if response.clicked() {
                        self.selected_file = Some(i);
                        if entry.is_dir {
                            navigate_to_path = Some(entry.path.clone());
                        }
                    }

                    if response.double_clicked() {
                        if entry.is_dir {
                            navigate_to_path = Some(entry.path.clone());
                        } else {
                            // Open file with default application
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

                if let Some(path) = navigate_to_path {
                    self.navigate_to(path);
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} items", self.entries.len()));

                if let Some(selected_idx) = self.selected_file {
                    if let Some(entry) = self.entries.get(selected_idx) {
                        ui.separator();
                        ui.label(format!("Selected: {}", entry.name));
                    }
                }
            });
        });
    }
}
