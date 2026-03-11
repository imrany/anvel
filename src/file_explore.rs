use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::thread;
use egui::{Color32, RichText, Vec2};

#[derive(Clone)]
struct ChatMessage {
    role: ChatRole,
    content: String,
}

#[derive(Clone, PartialEq)]
enum ChatRole { User, Assistant }

#[derive(Clone)]
struct MentionedFile {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    ext: String,
}

struct ContextMenuState {
    pos: egui::Pos2,
    entry_idx: usize,
}

#[derive(Clone)]
struct ShareDialog {
    name: String,
    #[allow(dead_code)]
    path: PathBuf,
    method: ShareMethod,
    state: ShareState,
    progress: f32,
    link: String,
    scan_timer: f32,
}

#[derive(Clone, PartialEq)]
enum ShareMethod { Lan, Bluetooth, Link, Cloud }

#[derive(Clone, PartialEq)]
enum ShareState {
    Scanning,
    Ready(Vec<String>),
    Transferring(String),
    Done(String),
}

#[derive(Clone)]
struct Notification {
    message: String,
    color: Color32,
    created: std::time::Instant,
}

// ─── Original types (unchanged) ───────────────────────────────────────────────

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

// ─── Main struct ──────────────────────────────────────────────────────────────

pub struct FileExplorer {
    // ── original fields ───────────────────────────────────────────────────────
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

    // ── new fields ────────────────────────────────────────────────────────────
    chat_messages: Vec<ChatMessage>,
    chat_input: String,
    ai_loading: bool,
    ai_response_receiver: Option<std::sync::mpsc::Receiver<String>>,
    at_mode: bool,
    at_query: String,
    mentioned_files: Vec<MentionedFile>,
    context_menu: Option<ContextMenuState>,
    share_dialog: Option<ShareDialog>,
    notifications: Vec<Notification>,
    renaming: Option<(usize, String)>,
    properties_dialog: Option<PathBuf>,
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

            chat_messages: vec![ChatMessage {
                role: ChatRole::Assistant,
                content: "Hi! I'm your AI file assistant.\nType @ to mention a file, or just ask me anything about your files.".into(),
            }],
            chat_input: String::new(),
            ai_loading: false,
            ai_response_receiver: None,
            at_mode: false,
            at_query: String::new(),
            mentioned_files: Vec::new(),
            context_menu: None,
            share_dialog: None,
            notifications: Vec::new(),
            renaming: None,
            properties_dialog: None,
        };
        explorer.load_directory(&home);
        explorer
    }
}

// ─── Original methods (unchanged) ─────────────────────────────────────────────

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
                SortBy::Name     => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size     => b.size.cmp(&a.size),
                SortBy::Modified => b.modified.cmp(&a.modified),
                SortBy::Type     => a.extension.to_lowercase().cmp(&b.extension.to_lowercase()),
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
                    self.push_notification(format!("Copied \"{}\"", entry.name), Color32::from_rgb(80, 180, 120));
                }
            }
        }
    }

    fn cut_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(entry_idx) = self.filtered_entries.get(idx) {
                if let Some(entry) = self.entries.get(*entry_idx) {
                    self.clipboard = Some((entry.path.clone(), FileOperation::Cut));
                    self.push_notification(format!("Cut \"{}\"", entry.name), Color32::from_rgb(240, 180, 60));
                }
            }
        }
    }

    fn paste_file(&mut self) {
        if let Some((source, operation)) = &self.clipboard.clone() {
            let file_name = source.file_name().unwrap();
            let dest = self.current_path.join(file_name);

            match operation {
                FileOperation::Copy => {
                    if source.is_file() {
                        let _ = fs::copy(source, &dest);
                    } else if source.is_dir() {
                        let _ = copy_dir_all(source, &dest);
                    }
                }
                FileOperation::Cut => {
                    let _ = fs::rename(source, &dest);
                    self.clipboard = None;
                }
            }

            self.load_directory(&self.current_path.clone());
            self.push_notification("Pasted successfully".into(), Color32::from_rgb(80, 180, 120));
        }
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
                        let name = entry.name.clone();
                        self.selected_file = None;
                        self.load_directory(&self.current_path.clone());
                        self.push_notification(format!("Deleted \"{}\"", name), Color32::from_rgb(230, 80, 80));
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

    fn get_file_icon(entry: &DirEntry) -> &'static str {
        if entry.is_dir {
            return "📁";
        }

        match entry.extension.to_lowercase().as_str() {
            "rs"                                       => "🦀",
            "toml"                                     => "⚙️",
            "md"                                       => "📝",
            "txt"                                      => "📄",
            "pdf"                                      => "📕",
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "bmp" => "🖼️",
            "mp3" | "wav" | "ogg" | "flac"             => "🎵",
            "mp4" | "avi" | "mkv" | "mov"              => "🎬",
            "zip" | "tar" | "gz" | "7z" | "rar"        => "📦",
            "js" | "ts" | "jsx" | "tsx"                => "🟨",
            "py"                                       => "🐍",
            "java"                                     => "☕",
            "cpp" | "c" | "h"                          => "⚡",
            "html" | "css"                             => "🌐",
            "json" | "xml" | "yaml" | "yml"            => "📋",
            _                                          => "📄",
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

            let mut navigate_to: Option<PathBuf> = None;
            for (i, component) in components.iter().enumerate() {
                if i > 0 {
                    ui.label(RichText::new("/").color(Color32::from_rgb(150, 150, 150)));
                }

                let name = component
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| component.to_str().unwrap_or(""));

                if ui.link(name).clicked() {
                    navigate_to = Some(component.clone());
                }
            }
            if let Some(p) = navigate_to {
                self.navigate_to(p);
            }
        });
    }
}

// ─── New methods ──────────────────────────────────────────────────────────────

impl FileExplorer {
    fn new_folder(&mut self) {
        let mut candidate = self.current_path.join("New Folder");
        let mut n = 1u32;
        while candidate.exists() {
            candidate = self.current_path.join(format!("New Folder ({})", n));
            n += 1;
        }
        let _ = fs::create_dir(&candidate);
        self.load_directory(&self.current_path.clone());
        self.push_notification("Created \"New Folder\"".into(), Color32::from_rgb(80, 180, 120));
    }

    fn open_file(path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let p = path.to_str().unwrap_or("");
            let _ = std::process::Command::new("cmd").args(["/C", "start", "", p]).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        let i  = self.selected_file?;
        let ei = self.filtered_entries.get(i)?;
        Some(self.entries.get(*ei)?.path.clone())
    }

    fn push_notification(&mut self, message: String, color: Color32) {
        self.notifications.push(Notification {
            message,
            color,
            created: std::time::Instant::now(),
        });
    }

    // ── AI chat ───────────────────────────────────────────────────────────────

    fn send_ai_message(&mut self) {
        let input = self.chat_input.trim().to_string();
        if input.is_empty() || self.ai_loading { return; }

        let ctx_note = if self.mentioned_files.is_empty() {
            String::new()
        } else {
            let files: Vec<String> = self.mentioned_files.iter().map(|f| {
                if f.is_dir { format!("{} (folder)", f.name) }
                else        { format!("{} (.{}, {})", f.name, f.ext, Self::format_size(f.size)) }
            }).collect();
            format!("\n\n[Files referenced: {}]", files.join(", "))
        };

        let full = format!("{}{}", input, ctx_note);
        self.chat_messages.push(ChatMessage { role: ChatRole::User, content: input });
        self.chat_input.clear();
        self.at_mode = false;
        self.mentioned_files.clear();
        self.ai_loading = true;

        // Build (role, content) pairs — no serde needed
        let history: Vec<(String, String)> = self.chat_messages.iter().enumerate().map(|(i, m)| {
            let role = if m.role == ChatRole::User { "user" } else { "assistant" }.to_string();
            let content = if i == self.chat_messages.len() - 1 { full.clone() } else { m.content.clone() };
            (role, content)
        }).collect();

        let (tx, rx) = std::sync::mpsc::channel::<String>();
        self.ai_response_receiver = Some(rx);

        thread::spawn(move || {
            let reply = call_anthropic(history)
                .unwrap_or_else(|e| format!("Error contacting AI: {}", e));
            let _ = tx.send(reply);
        });
    }

    fn process_at_input(&mut self, new_text: &str) {
        self.chat_input = new_text.to_string();
        if let Some(at_pos) = self.chat_input.rfind('@') {
            let before = &self.chat_input[..at_pos];
            if at_pos == 0 || before.ends_with(' ') {
                self.at_mode  = true;
                self.at_query = self.chat_input[at_pos + 1..].to_string();
                return;
            }
        }
        self.at_mode = false;
        self.at_query.clear();
    }

    fn select_mention(&mut self, entry_idx: usize) {
        if let Some(entry) = self.entries.get(entry_idx) {
            let mf = MentionedFile {
                name: entry.name.clone(), path: entry.path.clone(),
                is_dir: entry.is_dir, size: entry.size, ext: entry.extension.clone(),
            };
            let mname = mf.name.clone();
            if let Some(at) = self.chat_input.rfind('@') {
                self.chat_input = format!("{}@{} ", &self.chat_input[..at], mname);
            }
            if !self.mentioned_files.iter().any(|f| f.path == mf.path) {
                self.mentioned_files.push(mf);
            }
        }
        self.at_mode = false;
        self.at_query.clear();
    }

    fn at_candidates(&self) -> Vec<usize> {
        let q = self.at_query.to_lowercase();
        self.entries.iter().enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .take(8)
            .collect()
    }

    // ── Sharing ───────────────────────────────────────────────────────────────

    fn open_share(&mut self, method: ShareMethod) {
        if let Some(path) = self.selected_path() {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let link = format!("https://files.local/share/{:016x}", fnv_hash(&name));
            self.share_dialog = Some(ShareDialog {
                name, path: path, method,
                state: ShareState::Scanning,
                progress: 0.0, link, scan_timer: 0.0,
            });
        }
    }

    // ── Panels ────────────────────────────────────────────────────────────────

    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let back_enabled    = self.history_index > 0;
                let forward_enabled = self.history_index < self.path_history.len() - 1;

                ui.add_enabled_ui(back_enabled, |ui| {
                    if ui.button(RichText::new("◀").size(16.0)).clicked() { self.go_back(); }
                });
                ui.add_enabled_ui(forward_enabled, |ui| {
                    if ui.button(RichText::new("▶").size(16.0)).clicked() { self.go_forward(); }
                });
                if ui.button(RichText::new("⬆").size(16.0)).clicked()  { self.go_up(); }
                if ui.button(RichText::new("🏠").size(16.0)).clicked() {
                    if let Some(home) = dirs::home_dir() { self.navigate_to(home); }
                }
                if ui.button(RichText::new("🔄").size(16.0)).clicked() {
                    self.load_directory(&self.current_path.clone());
                }

                ui.separator();

                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Search files...")
                        .desired_width(200.0),
                );
                if search_response.changed() { self.apply_filter(); }

                ui.separator();

                if ui.button("📁 New Folder").clicked() { self.new_folder(); }

                if ui.button(if self.show_hidden { "👁 Show Hidden" } else { "👁‍🗨 Hide Hidden" }).clicked() {
                    self.show_hidden = !self.show_hidden;
                    self.load_directory(&self.current_path.clone());
                }
            });

            ui.add_space(4.0);
            self.render_breadcrumbs(ui);
            ui.add_space(8.0);
        });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sort by:").color(Color32::from_rgb(100, 100, 100)));

                if ui.selectable_label(self.sort_by == SortBy::Name,     "Name").clicked()     { self.sort_by = SortBy::Name;     self.sort_entries(); self.apply_filter(); }
                if ui.selectable_label(self.sort_by == SortBy::Size,     "Size").clicked()     { self.sort_by = SortBy::Size;     self.sort_entries(); self.apply_filter(); }
                if ui.selectable_label(self.sort_by == SortBy::Modified, "Modified").clicked() { self.sort_by = SortBy::Modified; self.sort_entries(); self.apply_filter(); }
                if ui.selectable_label(self.sort_by == SortBy::Type,     "Type").clicked()     { self.sort_by = SortBy::Type;     self.sort_entries(); self.apply_filter(); }

                ui.separator();

                ui.label(RichText::new("View:").color(Color32::from_rgb(100, 100, 100)));
                if ui.selectable_label(self.view_mode == ViewMode::List,    "List").clicked()    { self.view_mode = ViewMode::List; }
                if ui.selectable_label(self.view_mode == ViewMode::Details, "Details").clicked() { self.view_mode = ViewMode::Details; }

                // clipboard indicator
                if let Some((p, op)) = &self.clipboard {
                    ui.separator();
                    let label = match op { FileOperation::Copy => "📋 Copy", FileOperation::Cut => "✂️ Cut" };
                    let fname = p.file_name().unwrap_or_default().to_string_lossy();
                    ui.label(RichText::new(format!("{}: {}", label, fname))
                        .color(Color32::from_rgb(100, 160, 255)).size(12.0));
                }
            });
            ui.separator();
        });
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} items", self.filtered_entries.len()))
                    .color(Color32::from_rgb(100, 100, 100)));

                if let Some(selected_idx) = self.selected_file {
                    if let Some(&entry_idx) = self.filtered_entries.get(selected_idx) {
                        if let Some(entry) = self.entries.get(entry_idx) {
                            ui.separator();
                            ui.label(RichText::new(format!("Selected: {}", entry.name))
                                .color(Color32::from_rgb(80, 80, 200)));
                            if !entry.is_dir {
                                ui.separator();
                                ui.label(RichText::new(Self::format_size(entry.size))
                                    .color(Color32::from_rgb(100, 100, 100)));
                            }
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Ctrl+C Copy | Ctrl+X Cut | Ctrl+V Paste | Del Delete | F5 Refresh | Right-click for more | @ in chat to mention files")
                        .color(Color32::from_rgb(120, 120, 120)).size(11.0));
                });
            });
            ui.add_space(4.0);
        });
    }

    // ── File list ─────────────────────────────────────────────────────────────

    fn show_file_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(ref error) = self.error_message.clone() {
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

        let mut navigate_to_path: Option<PathBuf>            = None;
        let mut ctx_menu:         Option<(egui::Pos2, usize)> = None;
        let mut rename_done:      Option<(usize, String)>     = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.view_mode {
                ViewMode::List => {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 1.0);

                    for (i, &entry_idx) in self.filtered_entries.iter().enumerate() {
                        if let Some(entry) = self.entries.get(entry_idx) {
                            let is_selected = self.selected_file == Some(i);

                            // inline rename
                            if self.renaming.as_ref().map(|(ri, _)| *ri) == Some(i) {
                                let (_, new_name) = self.renaming.as_mut().unwrap();
                                let r = ui.add(egui::TextEdit::singleline(new_name).desired_width(220.0));
                                if r.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                    rename_done = self.renaming.clone();
                                }
                                if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                    rename_done = Some((i, String::new()));
                                }
                                continue;
                            }

                            let response = ui.selectable_label(
                                is_selected,
                                RichText::new(format!("{} {}", Self::get_file_icon(entry), entry.name)).size(14.0),
                            );

                            if response.clicked()        { self.selected_file = Some(i); }
                            if response.double_clicked() {
                                if entry.is_dir { navigate_to_path = Some(entry.path.clone()); }
                                else            { Self::open_file(&entry.path); }
                            }
                            if response.secondary_clicked() {
                                if let Some(pos) = ctx.input(|inp| inp.pointer.interact_pos()) {
                                    ctx_menu = Some((pos, entry_idx));
                                    self.selected_file = Some(i);
                                }
                            }
                        }
                    }
                }

                ViewMode::Details => {
                    use egui_extras::{Column, TableBuilder};

                    let mut nav:     Option<PathBuf>              = None;
                    let mut cmenu:   Option<(egui::Pos2, usize)>  = None;
                    let mut rndone:  Option<(usize, String)>      = None;

                    TableBuilder::new(ui)
                        .striped(true)
                        .sense(egui::Sense::click())
                        .column(Column::auto().at_least(300.0))
                        .column(Column::auto().at_least(80.0))
                        .column(Column::auto().at_least(100.0))
                        .column(Column::auto().at_least(80.0))
                        .header(24.0, |mut header| {
                            header.col(|ui| { ui.strong("Name"); });
                            header.col(|ui| { ui.strong("Size"); });
                            header.col(|ui| { ui.strong("Modified"); });
                            header.col(|ui| { ui.strong("Type"); });
                        })
                        .body(|mut body| {
                            for (i, &entry_idx) in self.filtered_entries.iter().enumerate() {
                                if let Some(entry) = self.entries.get(entry_idx) {
                                    let is_selected = self.selected_file == Some(i);

                                    body.row(22.0, |mut row| {
                                        row.set_selected(is_selected);

                                        let (_, name_resp) = row.col(|ui| {
                                            // inline rename
                                            if self.renaming.as_ref().map(|(ri, _)| *ri) == Some(i) {
                                                let (_, nn) = self.renaming.as_mut().unwrap();
                                                let r = ui.add(egui::TextEdit::singleline(nn).desired_width(200.0));
                                                if r.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                                    rndone = self.renaming.clone();
                                                }
                                            } else {
                                                ui.label(RichText::new(format!("{} {}", Self::get_file_icon(entry), entry.name)));
                                            }
                                        });

                                        row.col(|ui| {
                                            if !entry.is_dir {
                                                ui.label(RichText::new(Self::format_size(entry.size))
                                                    .color(Color32::from_rgb(120, 120, 120)));
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.label(RichText::new(Self::format_time(entry.modified))
                                                .color(Color32::from_rgb(120, 120, 120)));
                                        });
                                        row.col(|ui| {
                                            let type_str = if entry.is_dir { "Folder" }
                                                else if entry.extension.is_empty() { "File" }
                                                else { &entry.extension };
                                            ui.label(RichText::new(type_str).color(Color32::from_rgb(120, 120, 120)));
                                        });

                                        if name_resp.clicked()        { self.selected_file = Some(i); }
                                        if name_resp.double_clicked() {
                                            if entry.is_dir { nav = Some(entry.path.clone()); }
                                            else            { Self::open_file(&entry.path); }
                                        }
                                        if name_resp.secondary_clicked() {
                                            if let Some(pos) = ctx.input(|inp| inp.pointer.interact_pos()) {
                                                cmenu = Some((pos, entry_idx));
                                                self.selected_file = Some(i);
                                            }
                                        }
                                    });
                                }
                            }
                        });

                    if let Some(p) = nav    { navigate_to_path = Some(p); }
                    if let Some(c) = cmenu  { ctx_menu         = Some(c); }
                    if let Some(r) = rndone { rename_done      = Some(r); }
                }
            }
        });

        if let Some(path) = navigate_to_path { self.navigate_to(path); }
        if let Some((pos, idx)) = ctx_menu {
            self.context_menu = Some(ContextMenuState { pos, entry_idx: idx });
        }

        // commit rename
        if let Some((i, new_name)) = rename_done {
            if !new_name.is_empty() {
                if let Some(entry) = self.entries.get(i) {
                    let new_path = entry.path.parent().unwrap().join(&new_name);
                    let _ = fs::rename(&entry.path, &new_path);
                }
            }
            self.renaming = None;
            self.load_directory(&self.current_path.clone());
        }
    }

    // ── Context menu ──────────────────────────────────────────────────────────

    fn show_context_menu(&mut self, ctx: &egui::Context) {
        let Some(ref cm) = self.context_menu else { return };
        let pos       = cm.pos;
        let entry_idx = cm.entry_idx;

        let (epath, ename, eis_dir, esize, eext) = {
            let Some(e) = self.entries.get(entry_idx) else { self.context_menu = None; return };
            (e.path.clone(), e.name.clone(), e.is_dir, e.size, e.extension.clone())
        };

        let mut close:  bool          = false;
        let mut action: Option<String> = None;

        egui::Area::new(egui::Id::new("ctx_menu"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(30, 30, 40))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 80)))
                    .corner_radius(8.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        ui.add_space(4.0);

                        // file name header
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            let icon = if eis_dir { "📁" } else { Self::get_file_icon(self.entries.get(entry_idx).unwrap()) };
                            ui.label(RichText::new(format!("{} {}", icon, ename))
                                .color(Color32::from_rgb(160, 170, 200)).size(12.0).strong());
                        });
                        ui.add_space(2.0);
                        ui.separator();

                        if menu_item(ui, "↩️", "Open")              { action = Some("open".into());   close = true; }
                        if menu_item(ui, "✏️", "Rename")            { action = Some("rename".into()); close = true; }
                        if menu_item(ui, "📋", "Copy")              { action = Some("copy".into());   close = true; }
                        if menu_item(ui, "✂️", "Cut")               { action = Some("cut".into());    close = true; }
                        if self.clipboard.is_some() {
                            if menu_item(ui, "📌", "Paste Here")    { action = Some("paste".into());  close = true; }
                        }

                        ui.separator();
                        if menu_item(ui, "🤖", "Ask AI about this") { action = Some("ai".into());     close = true; }
                        if menu_item(ui, "ℹ️", "Properties")        { action = Some("props".into());  close = true; }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Share via").color(Color32::from_rgb(110, 110, 140)).size(11.0));
                        });
                        if menu_item(ui, "🌐", "  LAN Transfer")    { action = Some("share_lan".into());   close = true; }
                        if menu_item(ui, "📡", "  Bluetooth")       { action = Some("share_bt".into());    close = true; }
                        if menu_item(ui, "🔗", "  Copy Share Link") { action = Some("share_link".into());  close = true; }
                        if menu_item(ui, "☁️", "  Upload to Cloud") { action = Some("share_cloud".into()); close = true; }

                        ui.separator();
                        if menu_item_danger(ui, "🗑️", "Delete")     { action = Some("delete".into()); close = true; }
                        ui.add_space(4.0);
                    });

                if ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(ui.min_rect()) {
                    close = true;
                }
            });

        if close { self.context_menu = None; }

        let display_idx = self.filtered_entries.iter().position(|&e| e == entry_idx);

        if let Some(act) = action {
            match act.as_str() {
                "open" => {
                    if eis_dir { self.navigate_to(epath); }
                    else       { Self::open_file(&epath); }
                }
                "rename" => {
                    if let Some(di) = display_idx {
                        self.renaming     = Some((di, ename.clone()));
                        self.selected_file = Some(di);
                    }
                }
                "copy" => {
                    self.clipboard = Some((epath, FileOperation::Copy));
                    self.push_notification(format!("Copied \"{}\"", ename), Color32::from_rgb(80, 200, 120));
                }
                "cut" => {
                    self.clipboard = Some((epath, FileOperation::Cut));
                    self.push_notification(format!("Cut \"{}\"", ename), Color32::from_rgb(240, 180, 60));
                }
                "paste"  => { self.paste_file(); }
                "delete" => {
                    if let Some(di) = display_idx { self.selected_file = Some(di); }
                    self.delete_file();
                }
                "ai" => {
                    self.chat_input = format!("@{} ", ename);
                    self.mentioned_files.push(MentionedFile {
                        name: ename.clone(), path: epath,
                        is_dir: eis_dir, size: esize, ext: eext,
                    });
                    self.push_notification(
                        format!("Ready to ask AI about \"{}\" — type your question in the chat!", ename),
                        Color32::from_rgb(100, 180, 255),
                    );
                }
                "props"       => { self.properties_dialog = Some(epath); }
                "share_lan"   => { self.selected_file = display_idx; self.open_share(ShareMethod::Lan); }
                "share_bt"    => { self.selected_file = display_idx; self.open_share(ShareMethod::Bluetooth); }
                "share_link"  => { self.selected_file = display_idx; self.open_share(ShareMethod::Link); }
                "share_cloud" => { self.selected_file = display_idx; self.open_share(ShareMethod::Cloud); }
                _ => {}
            }
        }
    }

    // ── Share dialog ──────────────────────────────────────────────────────────

    fn show_share_dialog(&mut self, ctx: &egui::Context) {
        let Some(ref d) = self.share_dialog else { return };
        let title = match d.method {
            ShareMethod::Lan       => "🌐 Share via LAN",
            ShareMethod::Bluetooth => "📡 Share via Bluetooth",
            ShareMethod::Link      => "🔗 Copy Share Link",
            ShareMethod::Cloud     => "☁️ Upload to Cloud",
        };
        let fname    = d.name.clone();
        let link     = d.link.clone();
        let state    = d.state.clone();
        let progress = d.progress;
        let method   = d.method.clone();

        let mut close:          bool          = false;
        let mut start_transfer: Option<String> = None;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .min_width(360.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("File: {}", fname)).color(Color32::from_rgb(150, 160, 200)));
                ui.add_space(8.0);

                match method {
                    ShareMethod::Link => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Shareable link (expires in 24 h):")
                                .color(Color32::from_rgb(120, 130, 160)).size(12.0));
                            ui.add_space(4.0);
                            let mut l = link.clone();
                            ui.add(egui::TextEdit::singleline(&mut l).desired_width(310.0));
                            if ui.button("📋 Copy to Clipboard").clicked() {
                                ctx.copy_text(link.clone());
                                close = true;
                            }
                        });
                    }
                    _ => match &state {
                        ShareState::Scanning => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Scanning for nearby devices…");
                            });
                        }
                        ShareState::Ready(peers) => {
                            ui.label(RichText::new("Available devices:").size(12.0)
                                .color(Color32::from_rgb(120, 130, 160)));
                            ui.add_space(4.0);
                            for peer in peers {
                                let icon = if peer.contains("iPhone") || peer.contains("iPad") { "📱" } else { "💻" };
                                if ui.button(format!("{} {}", icon, peer)).clicked() {
                                    start_transfer = Some(peer.clone());
                                }
                            }
                        }
                        ShareState::Transferring(peer) => {
                            ui.label(format!("Sending to {}…", peer));
                            ui.add_space(4.0);
                            ui.add(egui::ProgressBar::new(progress).animate(true));
                        }
                        ShareState::Done(peer) => {
                            ui.label(RichText::new(format!("✅ Sent to {}!", peer))
                                .color(Color32::from_rgb(80, 220, 120)).size(16.0).strong());
                        }
                    }
                }

                ui.add_space(10.0);
                if ui.button("Close").clicked() { close = true; }
            });

        if let Some(peer) = start_transfer {
            if let Some(ref mut d) = self.share_dialog {
                d.state    = ShareState::Transferring(peer);
                d.progress = 0.0;
            }
        }
        if close { self.share_dialog = None; }
    }

    // ── Properties dialog ─────────────────────────────────────────────────────

    fn show_properties_dialog(&mut self, ctx: &egui::Context) {
        let Some(ref path) = self.properties_dialog.clone() else { return };
        let name     = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let md       = fs::metadata(path).ok();
        let size     = md.as_ref().map(|m| m.len()).unwrap_or(0);
        let is_dir   = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let modified = md.as_ref().and_then(|m| m.modified().ok());

        let mut open = true;
        egui::Window::new(format!("Properties — {}", name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .min_width(320.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("props").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                    ui.label(RichText::new("Name:").strong());    ui.label(&name);                   ui.end_row();
                    ui.label(RichText::new("Type:").strong());    ui.label(if is_dir { "Folder" } else { "File" }); ui.end_row();
                    ui.label(RichText::new("Size:").strong());    ui.label(Self::format_size(size)); ui.end_row();
                    ui.label(RichText::new("Modified:").strong()); ui.label(Self::format_time(modified)); ui.end_row();
                    ui.label(RichText::new("Path:").strong());
                    ui.label(RichText::new(path.to_string_lossy()).size(11.0).color(Color32::from_rgb(130, 140, 170)));
                    ui.end_row();
                });
            });
        if !open { self.properties_dialog = None; }
    }

    // ── Notifications ─────────────────────────────────────────────────────────

    fn show_notifications(&self, ctx: &egui::Context) {
        if self.notifications.is_empty() { return; }
        let screen = ctx.viewport_rect();
        let mut y  = screen.max.y - 16.0;

        for notif in self.notifications.iter().rev() {
            let age   = notif.created.elapsed().as_secs_f32();
            let alpha = ((5.0 - age).clamp(0.0, 1.0) * 255.0) as u8;
            let c     = notif.color;

            egui::Area::new(egui::Id::new(format!("notif_{:p}", notif)))
                .fixed_pos(egui::pos2(screen.max.x - 340.0, y - 44.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(28, 28, 40, alpha))
                        .stroke(egui::Stroke::new(1.0,
                            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 8, bottom: 8 })
                        .show(ui, |ui| {
                            ui.set_max_width(300.0);
                            ui.label(RichText::new(&notif.message)
                                .color(Color32::from_rgba_unmultiplied(215, 220, 240, alpha))
                                .size(13.0));
                        });
                });
            y -= 52.0;
        }
    }

    // ── AI chat sidebar ───────────────────────────────────────────────────────

    fn show_chat_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // header
            ui.horizontal(|ui| {
                ui.label(RichText::new("🤖").size(20.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new("File Assistant").strong().size(14.0));
                    ui.label(RichText::new("● Online").size(10.0).color(Color32::from_rgb(80, 200, 100)));
                });
            });
            ui.separator();

            // messages
            let available = ui.available_height();
            let input_h   = 70.0;
            let chips_h   = if !self.mentioned_files.is_empty() { 28.0 } else { 0.0 };
            let at_h      = if self.at_mode && !self.at_candidates().is_empty() { 130.0 } else { 0.0 };
            let msg_h     = (available - input_h - chips_h - at_h - 16.0).max(40.0);

            egui::ScrollArea::vertical()
                .id_salt("chat_scroll")
                .max_height(msg_h)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for msg in &self.chat_messages {
                        show_chat_bubble(ui, msg);
                        ui.add_space(4.0);
                    }
                    if self.ai_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Thinking…")
                                .color(Color32::from_rgb(130, 130, 160)).size(12.0).italics());
                        });
                    }
                });

            ui.separator();

            // @mention autocomplete
            if self.at_mode {
                let candidates = self.at_candidates();
                if !candidates.is_empty() {
                    egui::Frame::default()
                        .fill(Color32::from_rgb(30, 32, 45))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 65, 90)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin { left: 4, right: 4, top: 4, bottom: 4 })
                        .show(ui, |ui| {
                            ui.label(RichText::new("  Mention a file:")
                                .color(Color32::from_rgb(110, 120, 150)).size(11.0));
                            let mut select: Option<usize> = None;
                            for &idx in &candidates {
                                if let Some(entry) = self.entries.get(idx) {
                                    let lbl = format!("{} {}", Self::get_file_icon(entry), entry.name);
                                    if ui.selectable_label(false, RichText::new(lbl).size(12.0)).clicked() {
                                        select = Some(idx);
                                    }
                                }
                            }
                            if let Some(idx) = select { self.select_mention(idx); }
                        });
                }
            }

            // file chips
            if !self.mentioned_files.is_empty() {
                let mut remove: Option<usize> = None;
                ui.horizontal_wrapped(|ui| {
                    for (i, f) in self.mentioned_files.iter().enumerate() {
                        let chip = format!("{} {} ✕", if f.is_dir { "📁" } else { "📄" }, f.name);
                        if ui.small_button(RichText::new(chip)
                            .color(Color32::from_rgb(100, 160, 255)).size(11.0)).clicked()
                        {
                            remove = Some(i);
                        }
                    }
                });
                if let Some(i) = remove { self.mentioned_files.remove(i); }
            }

            // input
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut self.chat_input)
                        .hint_text("Ask about files… @ to mention")
                        .desired_width(ui.available_width() - 40.0)
                        .desired_rows(2),
                );
                if resp.changed() {
                    let t = self.chat_input.clone();
                    self.process_at_input(&t);
                }
                if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                    self.send_ai_message();
                }
                let can_send = !self.chat_input.trim().is_empty() && !self.ai_loading;
                ui.add_enabled_ui(can_send, |ui| {
                    if ui.button(RichText::new("➤").size(18.0)).clicked() {
                        self.send_ai_message();
                    }
                });
            });
            ui.label(RichText::new("Enter to send  •  Shift+Enter newline  •  @ to tag a file")
                .color(Color32::from_rgb(110, 110, 130)).size(10.0));
        });
    }
}

// ─── eframe::App ─────────────────────────────────────────────────────────────

impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // poll AI response
        if self.ai_loading {
            if let Some(rx) = &self.ai_response_receiver {
                if let Ok(reply) = rx.try_recv() {
                    self.chat_messages.push(ChatMessage { role: ChatRole::Assistant, content: reply });
                    self.ai_loading = false;
                    self.ai_response_receiver = None;
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // expire notifications
        self.notifications.retain(|n| n.created.elapsed().as_secs() < 5);

        // advance share scan timer
        if let Some(ref mut d) = self.share_dialog {
            if d.state == ShareState::Scanning {
                d.scan_timer += ctx.input(|i| i.unstable_dt);
                if d.scan_timer > 1.2 {
                    d.state = ShareState::Ready(vec![
                        "MacBook Pro (192.168.1.5)".into(),
                        "iPhone 15 Pro".into(),
                        "iPad Air".into(),
                    ]);
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(80));
            }
            if let ShareState::Transferring(_) = d.state.clone() {
                d.progress = (d.progress + ctx.input(|i| i.unstable_dt) * 0.35).min(1.0);
                if d.progress >= 1.0 {
                    let peer = if let ShareState::Transferring(p) = &d.state { p.clone() } else { "device".into() };
                    d.state = ShareState::Done(peer);
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        // keyboard shortcuts — same as original
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                self.load_directory(&self.current_path.clone());
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) { self.copy_file(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X) { self.cut_file(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) { self.paste_file(); }
            if i.key_pressed(egui::Key::Delete) { self.delete_file(); }
            if i.key_pressed(egui::Key::Backspace)
                || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
            {
                self.go_back();
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) { self.go_forward(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                self.show_hidden = !self.show_hidden;
                self.load_directory(&self.current_path.clone());
            }
            if i.key_pressed(egui::Key::Escape) {
                self.context_menu = None;
                self.at_mode = false;
            }
        });

        // panels
        self.show_top_panel(ctx);
        self.show_toolbar(ctx);
        self.show_bottom_panel(ctx);

        // Chat panel docked right — must be declared before CentralPanel
        egui::SidePanel::right("chat_panel")
            .resizable(true)
            .min_width(260.0)
            .default_width(310.0)
            .max_width(500.0)
            .show(ctx, |ui| {
                self.show_chat_sidebar(ui);
            });

        // File list takes all remaining space automatically
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref error) = self.error_message.clone() {
                ui.colored_label(Color32::RED, RichText::new(error).strong());
                ui.add_space(8.0);
            }
            self.show_file_list(ui, ctx);
        });

        // floating layers
        self.show_context_menu(ctx);
        self.show_share_dialog(ctx);
        self.show_properties_dialog(ctx);
        self.show_notifications(ctx);
    }
}

// ─── Free helpers ─────────────────────────────────────────────────────────────

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() { copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?; }
        else           { fs::copy(entry.path(), dst.join(entry.file_name()))?; }
    }
    Ok(())
}

fn fnv_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        h.wrapping_mul(0x100000001b3).wrapping_add(b as u64)
    })
}

/// Calls the Anthropic API by shelling out to `curl` — zero extra dependencies.
fn call_anthropic(history: Vec<(String, String)>) -> Result<String, String> {
    let msgs: Vec<String> = history.iter().map(|(role, content)| {
        let esc = content
            .replace('\\', "\\\\")
            .replace('"',  "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!(r#"{{"role":"{}","content":"{}"}}"#, role, esc)
    }).collect();

    let body = format!(
        r#"{{"model":"claude-haiku-4-5-20251001","max_tokens":1024,"system":"You are an intelligent file assistant embedded in a desktop file explorer. Help users manage, understand, and work with their files. Be concise and practical. Plain text only — no markdown.","messages":[{}]}}"#,
        msgs.join(",")
    );

    let out = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            "https://api.anthropic.com/v1/messages",
            "-H", "content-type: application/json",
            "-H", "anthropic-version: 2023-06-01",
            "-d", &body,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;

    let resp = String::from_utf8_lossy(&out.stdout).to_string();

    // Parse "text":"..." out of the JSON manually
    if let Some(start) = resp.find("\"text\":\"") {
        let rest = &resp[start + 8..];
        let mut result = String::new();
        let mut chars  = rest.chars().peekable();
        loop {
            match chars.next() {
                None | Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('n')  => result.push('\n'),
                    Some('t')  => result.push('\t'),
                    Some('"')  => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(c)    => { result.push('\\'); result.push(c); }
                    None       => break,
                },
                Some(c) => result.push(c),
            }
        }
        if !result.is_empty() { return Ok(result); }
    }

    // surface API error messages
    if let Some(s) = resp.find("\"message\":\"") {
        let rest = &resp[s + 11..];
        if let Some(end) = rest.find('"') {
            return Err(format!("API error: {}", &rest[..end]));
        }
    }

    Err("Could not parse API response. Check your ANTHROPIC_API_KEY env var.".into())
}

fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(format!("{}  {}", icon, label))
            .color(Color32::from_rgb(200, 205, 225)).size(13.0))
            .frame(false)
            .min_size(Vec2::new(200.0, 24.0)),
    ).clicked()
}

fn menu_item_danger(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(format!("{}  {}", icon, label))
            .color(Color32::from_rgb(230, 80, 80)).size(13.0))
            .frame(false)
            .min_size(Vec2::new(200.0, 24.0)),
    ).clicked()
}

fn show_chat_bubble(ui: &mut egui::Ui, msg: &ChatMessage) {
    match msg.role {
        ChatRole::Assistant => {
            ui.horizontal_top(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("🤖").size(15.0));
                egui::Frame::default()
                    .fill(Color32::from_rgb(35, 38, 55))
                    .corner_radius(egui::CornerRadius { nw: 2, ne: 10, sw: 10, se: 10 })
                    .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        ui.label(RichText::new(&msg.content)
                            .color(Color32::from_rgb(200, 210, 240)).size(12.5));
                    });
            });
        }
        ChatRole::User => {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add_space(4.0);
                egui::Frame::default()
                    .fill(Color32::from_rgb(30, 60, 130))
                    .corner_radius(egui::CornerRadius { nw: 10, ne: 2, sw: 10, se: 10 })
                    .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        ui.label(RichText::new(&msg.content)
                            .color(Color32::from_rgb(220, 230, 255)).size(12.5));
                    });
            });
        }
    }
}
