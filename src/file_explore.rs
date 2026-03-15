use egui::{Color32, RichText, Vec2};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::SystemTime;

// ─── Theme helpers ─────────────────────────────────────────────────────────────

fn is_dark(ctx: &egui::Context) -> bool {
    ctx.style().visuals.dark_mode
}

fn col_surface(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(18, 18, 18)
    } else {
        Color32::WHITE
    }
}

fn col_surface2(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(26, 26, 26)
    } else {
        Color32::from_rgb(245, 245, 245)
    }
}

fn col_surface3(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(32, 32, 32)
    } else {
        Color32::from_rgb(235, 235, 235)
    }
}

fn col_text(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(220, 220, 220)
    } else {
        Color32::from_rgb(20, 20, 20)
    }
}

fn col_text2(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(140, 140, 140)
    } else {
        Color32::from_rgb(100, 100, 100)
    }
}

fn col_border(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(50, 50, 50)
    } else {
        Color32::from_rgb(200, 200, 200)
    }
}

fn col_accent(ctx: &egui::Context) -> Color32 {
    if is_dark(ctx) {
        Color32::from_rgb(130, 160, 220)
    } else {
        Color32::from_rgb(40, 90, 180)
    }
}

fn col_success(_ctx: &egui::Context) -> Color32 {
    Color32::from_rgb(80, 180, 120)
}
fn col_warn(_ctx: &egui::Context) -> Color32 {
    Color32::from_rgb(210, 160, 50)
}
fn col_danger(_ctx: &egui::Context) -> Color32 {
    Color32::from_rgb(210, 70, 70)
}

// ─── LAN ──────────────────────────────────────────────────────────────────────

const LAN_DISCOVER_PORT: u16 = 44444;
const LAN_TRANSFER_PORT: u16 = 44445;

#[derive(Clone)]
struct LanPeer {
    display: String,
    addr: std::net::IpAddr,
}

#[derive(Clone)]
enum LanTransferState {
    Idle,
    Discovering,
    Ready(Vec<LanPeer>),
    Sending { peer_name: String, progress: f32 },
    Done(String),
    Err(String),
}

enum LanServerMsg {
    FileReceived { name: String, dest: PathBuf },
    Error(String),
}

// ─── UI types ─────────────────────────────────────────────────────────────────

struct ContextMenuState {
    pos: egui::Pos2,
    entry_idx: usize,
    /// Pass counter on which this menu was opened — guards against same-frame close.
    opened_frame: u64,
}

#[derive(Clone)]
struct Notification {
    message: String,
    color: Color32,
    created: std::time::Instant,
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

// ─── Main struct ──────────────────────────────────────────────────────────────

pub struct FileExplorer {
    // file browser
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
    renaming: Option<(usize, String)>,
    properties_dialog: Option<PathBuf>,
    notifications: Vec<Notification>,
    context_menu: Option<ContextMenuState>,

    // theme
    dark_mode: bool,

    // LAN
    lan_state: LanTransferState,
    lan_file_path: Option<PathBuf>,
    lan_discover_rx: Option<std::sync::mpsc::Receiver<Vec<LanPeer>>>,
    lan_transfer_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    lan_server_rx: Option<std::sync::mpsc::Receiver<LanServerMsg>>,
}

impl Default for FileExplorer {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        let (srv_tx, srv_rx) = std::sync::mpsc::channel::<LanServerMsg>();
        let recv_dir = home.clone();
        thread::spawn(move || lan_receive_server(srv_tx, recv_dir));

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
            renaming: None,
            properties_dialog: None,
            notifications: Vec::new(),
            context_menu: None,

            dark_mode: true,

            lan_state: LanTransferState::Idle,
            lan_file_path: None,
            lan_discover_rx: None,
            lan_transfer_rx: None,
            lan_server_rx: Some(srv_rx),
        };
        explorer.load_directory(&home);
        explorer
    }
}

// ─── File browser helpers ─────────────────────────────────────────────────────

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
                        let extension = entry
                            .path()
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
            let q = self.search_query.to_lowercase();
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.name.to_lowercase().contains(&q) {
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

    fn new_folder(&mut self) {
        let mut candidate = self.current_path.join("New Folder");
        let mut n = 1u32;
        while candidate.exists() {
            candidate = self.current_path.join(format!("New Folder ({})", n));
            n += 1;
        }
        let _ = fs::create_dir(&candidate);
        self.load_directory(&self.current_path.clone());
        self.push_notification(
            "Created \"New Folder\"".into(),
            Color32::from_rgb(80, 180, 120),
        );
    }

    fn open_file(path: &Path) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }

        // On Windows, use CREATE_NO_WINDOW so no console flash appears
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", path.to_str().unwrap_or("")])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }

    fn copy_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    self.clipboard = Some((e.path.clone(), FileOperation::Copy));
                    self.push_notification(
                        format!("Copied \"{}\"", e.name),
                        Color32::from_rgb(80, 180, 120),
                    );
                }
            }
        }
    }

    fn cut_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    self.clipboard = Some((e.path.clone(), FileOperation::Cut));
                    self.push_notification(
                        format!("Cut \"{}\"", e.name),
                        Color32::from_rgb(210, 160, 50),
                    );
                }
            }
        }
    }

    fn paste_file(&mut self) {
        if let Some((source, operation)) = &self.clipboard.clone() {
            let dest = self.current_path.join(source.file_name().unwrap());
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
            self.push_notification(
                "Pasted successfully".into(),
                Color32::from_rgb(80, 180, 120),
            );
        }
    }

    fn delete_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    let result = if e.is_dir {
                        fs::remove_dir_all(&e.path)
                    } else {
                        fs::remove_file(&e.path)
                    };
                    if let Err(err) = result {
                        self.error_message = Some(format!("Failed to delete: {}", err));
                    } else {
                        let name = e.name.clone();
                        self.selected_file = None;
                        self.load_directory(&self.current_path.clone());
                        self.push_notification(
                            format!("Deleted \"{}\"", name),
                            Color32::from_rgb(210, 70, 70),
                        );
                    }
                }
            }
        }
    }

    fn push_notification(&mut self, message: String, color: Color32) {
        self.notifications.push(Notification {
            message,
            color,
            created: std::time::Instant::now(),
        });
    }

    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut ui = 0usize;
        while size >= 1024.0 && ui < UNITS.len() - 1 {
            size /= 1024.0;
            ui += 1;
        }
        if ui == 0 {
            format!("{} {}", size as u64, UNITS[ui])
        } else {
            format!("{:.2} {}", size, UNITS[ui])
        }
    }

    fn format_time(time: Option<SystemTime>) -> String {
        time.and_then(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH).ok().map(|d| {
                let diff = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .saturating_sub(d.as_secs());
                if diff < 60 {
                    "Just now".into()
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
        .unwrap_or_else(|| "Unknown".into())
    }

    fn get_file_icon(entry: &DirEntry) -> &'static str {
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

    fn render_breadcrumbs(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Path:").color(col_text2(ctx)).size(12.0));
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
            let mut nav: Option<PathBuf> = None;
            for (i, comp) in components.iter().enumerate() {
                if i > 0 {
                    ui.label(RichText::new("/").color(col_text2(ctx)));
                }
                let name = comp
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| comp.to_str().unwrap_or(""));
                if ui
                    .link(RichText::new(name).size(12.0).color(col_accent(ctx)))
                    .clicked()
                {
                    nav = Some(comp.clone());
                }
            }
            if let Some(p) = nav {
                self.navigate_to(p);
            }
        });
    }
}

// ─── LAN ──────────────────────────────────────────────────────────────────────

impl FileExplorer {
    fn start_lan_discover(&mut self, file_path: PathBuf) {
        self.lan_file_path = Some(file_path);
        self.lan_state = LanTransferState::Discovering;
        let (tx, rx) = std::sync::mpsc::channel::<Vec<LanPeer>>();
        self.lan_discover_rx = Some(rx);

        thread::spawn(move || {
            let mut peers: Vec<LanPeer> = Vec::new();
            if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
                let _ = sock.set_broadcast(true);
                let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(200)));
                let ping = b"ANVEL_DISCOVER";
                let _ = sock.send_to(ping, ("255.255.255.255", LAN_DISCOVER_PORT));
                for subnet in ["192.168.1.255", "192.168.0.255", "10.0.0.255"] {
                    let _ = sock.send_to(ping, (subnet, LAN_DISCOVER_PORT));
                }
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let mut buf = [0u8; 256];
                while std::time::Instant::now() < deadline {
                    if let Ok((n, addr)) = sock.recv_from(&mut buf) {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        if let Some(name) = msg.strip_prefix("ANVEL_PEER:") {
                            let peer = LanPeer {
                                display: format!("{} ({})", name.trim(), addr.ip()),
                                addr: addr.ip(),
                            };
                            if !peers.iter().any(|p| p.addr == peer.addr) {
                                peers.push(peer);
                            }
                        }
                    }
                }
            }
            let _ = tx.send(peers);
        });

        thread::spawn(|| {
            let _ = lan_advertise_once();
        });
    }

    fn start_lan_send(&mut self, peer: LanPeer) {
        let Some(file_path) = self.lan_file_path.clone() else {
            return;
        };
        let peer_name = peer.display.clone();
        self.lan_state = LanTransferState::Sending {
            peer_name,
            progress: 0.0,
        };
        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        self.lan_transfer_rx = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(lan_send_file(&file_path, peer.addr));
        });
    }

    fn poll_lan(&mut self) {
        if let Some(rx) = &self.lan_discover_rx {
            if let Ok(peers) = rx.try_recv() {
                self.lan_discover_rx = None;
                self.lan_state = LanTransferState::Ready(peers);
            }
        }
        if let Some(rx) = &self.lan_transfer_rx {
            if let Ok(result) = rx.try_recv() {
                self.lan_transfer_rx = None;
                self.lan_state = match result {
                    Ok(()) => LanTransferState::Done("File sent successfully!".into()),
                    Err(e) => LanTransferState::Err(e),
                };
            }
        }
        let server_msgs: Vec<LanServerMsg> = if let Some(rx) = &self.lan_server_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            Vec::new()
        };
        for msg in server_msgs {
            match msg {
                LanServerMsg::FileReceived { name, dest } => {
                    // Show the folder where the file was saved
                    let folder = dest
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "home".to_string());
                    self.push_notification(
                        format!("📥 Received \"{}\" → {}", name, folder),
                        Color32::from_rgb(80, 180, 120),
                    );
                    // Refresh the file list if the file landed in the current folder
                    if dest.parent().map_or(false, |p| p == self.current_path) {
                        self.load_directory(&self.current_path.clone());
                    }
                }
                LanServerMsg::Error(e) => self
                    .push_notification(format!("LAN error: {}", e), Color32::from_rgb(210, 70, 70)),
            }
        }
    }
}

// ─── Panel rendering ──────────────────────────────────────────────────────────

impl FileExplorer {
    fn show_top_panel(&mut self, ctx: &egui::Context) {
        let bg = col_surface2(ctx);
        let txt = col_text(ctx);

        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 6,
                bottom: 6,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let back_en = self.history_index > 0;
                    let forward_en = self.history_index < self.path_history.len() - 1;

                    ui.add_enabled_ui(back_en, |ui| {
                        if ui
                            .button(RichText::new("◀").size(15.0).color(txt))
                            .on_hover_text("Back (Alt+←)")
                            .clicked()
                        {
                            self.go_back();
                        }
                    });
                    ui.add_enabled_ui(forward_en, |ui| {
                        if ui
                            .button(RichText::new("▶").size(15.0).color(txt))
                            .on_hover_text("Forward (Alt+→)")
                            .clicked()
                        {
                            self.go_forward();
                        }
                    });
                    if ui
                        .button(RichText::new("⬆").size(15.0).color(txt))
                        .on_hover_text("Up")
                        .clicked()
                    {
                        self.go_up();
                    }
                    if ui
                        .button(RichText::new("🏠").size(15.0))
                        .on_hover_text("Home")
                        .clicked()
                    {
                        if let Some(home) = dirs::home_dir() {
                            self.navigate_to(home);
                        }
                    }
                    if ui
                        .button(RichText::new("🔄").size(15.0))
                        .on_hover_text("Refresh (F5)")
                        .clicked()
                    {
                        self.load_directory(&self.current_path.clone());
                    }

                    ui.separator();

                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("🔍  Search…")
                            .desired_width(200.0),
                    );
                    if search_resp.changed() {
                        self.apply_filter();
                    }

                    ui.separator();
                    if ui.button("📁  New Folder").clicked() {
                        self.new_folder();
                    }
                    if ui
                        .button(if self.show_hidden {
                            "👁  Hide Hidden"
                        } else {
                            "👁  Show Hidden"
                        })
                        .clicked()
                    {
                        self.show_hidden = !self.show_hidden;
                        self.load_directory(&self.current_path.clone());
                    }
                });

                ui.add_space(3.0);
                self.render_breadcrumbs(ui, ctx);
                ui.add_space(3.0);
            });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        let bg = col_surface3(ctx);
        let t2 = col_text2(ctx);

        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sort:").color(t2).size(12.0));
                    for (label, s) in [
                        ("Name", SortBy::Name),
                        ("Size", SortBy::Size),
                        ("Modified", SortBy::Modified),
                        ("Type", SortBy::Type),
                    ] {
                        if ui
                            .selectable_label(self.sort_by == s, RichText::new(label).size(12.0))
                            .clicked()
                        {
                            self.sort_by = s;
                            self.sort_entries();
                            self.apply_filter();
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("View:").color(t2).size(12.0));
                    if ui
                        .selectable_label(
                            self.view_mode == ViewMode::List,
                            RichText::new("List").size(12.0),
                        )
                        .clicked()
                    {
                        self.view_mode = ViewMode::List;
                    }
                    if ui
                        .selectable_label(
                            self.view_mode == ViewMode::Details,
                            RichText::new("Details").size(12.0),
                        )
                        .clicked()
                    {
                        self.view_mode = ViewMode::Details;
                    }
                    if let Some((p, op)) = &self.clipboard {
                        ui.separator();
                        let label = match op {
                            FileOperation::Copy => "📋 Clipboard: ",
                            FileOperation::Cut => "✂️ Clipboard: ",
                        };
                        let fname = p.file_name().unwrap_or_default().to_string_lossy();
                        ui.label(
                            RichText::new(format!("{}{}", label, fname))
                                .color(col_accent(ctx))
                                .size(12.0),
                        );
                    }
                });
            });
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        let bg = col_surface(ctx);
        let t2 = col_text2(ctx);
        let ac = col_accent(ctx);

        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} item{}",
                            self.filtered_entries.len(),
                            if self.filtered_entries.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ))
                        .color(t2)
                        .size(12.0),
                    );

                    if let Some(idx) = self.selected_file {
                        if let Some(&ei) = self.filtered_entries.get(idx) {
                            if let Some(e) = self.entries.get(ei) {
                                ui.separator();
                                ui.label(RichText::new(&e.name).color(ac).size(12.0));
                                if !e.is_dir {
                                    ui.separator();
                                    ui.label(
                                        RichText::new(Self::format_size(e.size))
                                            .color(t2)
                                            .size(12.0),
                                    );
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("Ctrl+C/X/V  Del  F5  Right-click  @ in chat")
                                .color(t2)
                                .size(11.0),
                        );
                    });
                });
            });
    }

    // ── File list ─────────────────────────────────────────────────────────────

    fn show_file_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(ref err) = self.error_message.clone() {
            ui.colored_label(col_danger(ctx), RichText::new(err).strong());
            ui.add_space(8.0);
        }

        if self.filtered_entries.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("📂").size(48.0));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("No files here")
                        .size(18.0)
                        .color(col_text2(ctx)),
                );
            });
            return;
        }

        let mut navigate_to_path: Option<PathBuf> = None;
        let mut ctx_menu: Option<(egui::Pos2, usize)> = None;
        let mut rename_done: Option<(usize, String)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.view_mode {
                    ViewMode::List => {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 1.0);
                        for (i, &ei) in self.filtered_entries.iter().enumerate() {
                            if let Some(entry) = self.entries.get(ei) {
                                let is_sel = self.selected_file == Some(i);

                                if self.renaming.as_ref().map(|(ri, _)| *ri) == Some(i) {
                                    let (_, nn) = self.renaming.as_mut().unwrap();
                                    let r =
                                        ui.add(egui::TextEdit::singleline(nn).desired_width(220.0));
                                    if r.lost_focus()
                                        || ui.input(|inp| inp.key_pressed(egui::Key::Enter))
                                    {
                                        rename_done = self.renaming.clone();
                                    }
                                    if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                        rename_done = Some((i, String::new()));
                                    }
                                    continue;
                                }

                                // FIX: Button::selectable replaces deprecated SelectableLabel
                                let label_text =
                                    format!("{} {}", Self::get_file_icon(entry), entry.name);
                                let resp = ui.add_sized(
                                    [ui.available_width(), 24.0],
                                    egui::Button::selectable(
                                        is_sel,
                                        RichText::new(label_text).size(14.0),
                                    ),
                                );
                                if resp.clicked() {
                                    self.selected_file = Some(i);
                                }
                                if resp.double_clicked() {
                                    if entry.is_dir {
                                        navigate_to_path = Some(entry.path.clone());
                                    } else {
                                        Self::open_file(&entry.path);
                                    }
                                }
                                if resp.secondary_clicked() {
                                    if let Some(pos) = ctx.input(|inp| inp.pointer.interact_pos()) {
                                        ctx_menu = Some((pos, ei));
                                        self.selected_file = Some(i);
                                    }
                                }
                            }
                        }
                    }

                    ViewMode::Details => {
                        use egui_extras::{Column, TableBuilder};
                        let mut nav: Option<PathBuf> = None;
                        let mut cmenu: Option<(egui::Pos2, usize)> = None;
                        let mut rndone: Option<(usize, String)> = None;
                        let t2 = col_text2(ctx);

                        TableBuilder::new(ui)
                            .striped(true)
                            .sense(egui::Sense::click())
                            .column(Column::remainder().at_least(200.0))
                            .column(Column::auto().at_least(80.0))
                            .column(Column::auto().at_least(110.0))
                            .column(Column::auto().at_least(60.0))
                            .header(22.0, |mut h| {
                                h.col(|ui| {
                                    ui.strong("Name");
                                });
                                h.col(|ui| {
                                    ui.strong("Size");
                                });
                                h.col(|ui| {
                                    ui.strong("Modified");
                                });
                                h.col(|ui| {
                                    ui.strong("Type");
                                });
                            })
                            .body(|mut body| {
                                for (i, &ei) in self.filtered_entries.iter().enumerate() {
                                    if let Some(entry) = self.entries.get(ei) {
                                        let is_sel = self.selected_file == Some(i);
                                        // Capture interactions via label responses; body.row()
                                        // returns () in egui 0.33 so we can't call .clicked() on it.
                                        let mut row_click = false;
                                        let mut row_dbl = false;
                                        let mut row_rclick = false;
                                        let mut row_rect = egui::Rect::NOTHING;

                                        body.row(24.0, |mut row| {
                                            row.set_selected(is_sel);
                                            row.col(|ui| {
                                                if self.renaming.as_ref().map(|(ri, _)| *ri)
                                                    == Some(i)
                                                {
                                                    let (_, nn) = self.renaming.as_mut().unwrap();
                                                    let r = ui.add(
                                                        egui::TextEdit::singleline(nn)
                                                            .desired_width(200.0),
                                                    );
                                                    if r.lost_focus()
                                                        || ui.input(|inp| {
                                                            inp.key_pressed(egui::Key::Enter)
                                                        })
                                                    {
                                                        rndone = self.renaming.clone();
                                                    }
                                                } else {
                                                    let r = ui.label(RichText::new(format!(
                                                        "{} {}",
                                                        Self::get_file_icon(entry),
                                                        entry.name
                                                    )));
                                                    row_rect = r.rect;
                                                    if r.clicked() {
                                                        row_click = true;
                                                    }
                                                    if r.double_clicked() {
                                                        row_dbl = true;
                                                    }
                                                    if r.secondary_clicked() {
                                                        row_rclick = true;
                                                    }
                                                }
                                            });
                                            row.col(|ui| {
                                                if !entry.is_dir {
                                                    ui.label(
                                                        RichText::new(Self::format_size(
                                                            entry.size,
                                                        ))
                                                        .color(t2),
                                                    );
                                                }
                                            });
                                            row.col(|ui| {
                                                ui.label(
                                                    RichText::new(Self::format_time(
                                                        entry.modified,
                                                    ))
                                                    .color(t2),
                                                );
                                            });
                                            row.col(|ui| {
                                                let t = if entry.is_dir {
                                                    "Folder"
                                                } else if entry.extension.is_empty() {
                                                    "File"
                                                } else {
                                                    &entry.extension
                                                };
                                                ui.label(RichText::new(t).color(t2));
                                            });
                                        });

                                        if row_click {
                                            self.selected_file = Some(i);
                                        }
                                        if row_dbl {
                                            if entry.is_dir {
                                                nav = Some(entry.path.clone());
                                            } else {
                                                Self::open_file(&entry.path);
                                            }
                                        }
                                        if row_rclick {
                                            let pos = ctx
                                                .input(|inp| inp.pointer.interact_pos())
                                                .unwrap_or(row_rect.left_bottom());
                                            cmenu = Some((pos, ei));
                                            self.selected_file = Some(i);
                                        }
                                    }
                                }
                            });

                        if let Some(p) = nav {
                            navigate_to_path = Some(p);
                        }
                        if let Some(c) = cmenu {
                            ctx_menu = Some(c);
                        }
                        if let Some(r) = rndone {
                            rename_done = Some(r);
                        }
                    }
                }
            });

        if let Some(path) = navigate_to_path {
            self.navigate_to(path);
        }
        if let Some((pos, idx)) = ctx_menu {
            self.context_menu = Some(ContextMenuState {
                pos,
                entry_idx: idx,
                // FIX: cumulative_pass_nr() replaces the removed frame_nr()
                opened_frame: ctx.cumulative_pass_nr(),
            });
        }
        if let Some((i, new_name)) = rename_done {
            if !new_name.is_empty() {
                if let Some(&ei) = self.filtered_entries.get(i) {
                    if let Some(entry) = self.entries.get(ei) {
                        let new_path = entry.path.parent().unwrap().join(&new_name);
                        let _ = fs::rename(&entry.path, &new_path);
                    }
                }
            }
            self.renaming = None;
            self.load_directory(&self.current_path.clone());
        }
    }

    // ── Context menu ──────────────────────────────────────────────────────────

    fn show_context_menu(&mut self, ctx: &egui::Context) {
        let Some(ref cm) = self.context_menu else {
            return;
        };
        let pos = cm.pos;
        let ei = cm.entry_idx;
        let opened_frame = cm.opened_frame;

        let (epath, ename, eis_dir, _esize, _eext) = {
            let Some(e) = self.entries.get(ei) else {
                self.context_menu = None;
                return;
            };
            (
                e.path.clone(),
                e.name.clone(),
                e.is_dir,
                e.size,
                e.extension.clone(),
            )
        };

        let mut close = false;
        let mut action: Option<String> = None;

        let bg = col_surface2(ctx);
        let border = col_border(ctx);
        let txt = col_text(ctx);

        let area_resp = egui::Area::new(egui::Id::new("ctx_menu"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.0, border))
                    .corner_radius(10.0)
                    .shadow(egui::Shadow {
                        offset: [2, 6],
                        blur: 14,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(egui::Margin {
                        left: 4,
                        right: 4,
                        top: 6,
                        bottom: 6,
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            let icon = if eis_dir {
                                "📁"
                            } else {
                                Self::get_file_icon(self.entries.get(ei).unwrap())
                            };
                            ui.label(
                                RichText::new(format!("{} {}", icon, ename))
                                    .color(txt)
                                    .size(13.0)
                                    .strong(),
                            );
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        if menu_item(ui, "↩️", "Open", ctx) {
                            action = Some("open".into());
                            close = true;
                        }
                        if menu_item(ui, "✏️", "Rename", ctx) {
                            action = Some("rename".into());
                            close = true;
                        }
                        if menu_item(ui, "📋", "Copy", ctx) {
                            action = Some("copy".into());
                            close = true;
                        }
                        if menu_item(ui, "✂️", "Cut", ctx) {
                            action = Some("cut".into());
                            close = true;
                        }
                        if self.clipboard.is_some() {
                            if menu_item(ui, "📌", "Paste Here", ctx) {
                                action = Some("paste".into());
                                close = true;
                            }
                        }
                        ui.separator();
                        if menu_item(ui, "ℹ️", "Properties", ctx) {
                            action = Some("props".into());
                            close = true;
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Share via LAN")
                                    .color(col_text2(ctx))
                                    .size(11.0),
                            );
                        });
                        if menu_item(ui, "🌐", "  Send over LAN", ctx) {
                            action = Some("share_lan".into());
                            close = true;
                        }
                        if menu_item(ui, "🔗", "  Copy Share Link", ctx) {
                            action = Some("share_link".into());
                            close = true;
                        }
                        ui.separator();
                        if menu_item_danger(ui, "🗑️", "Delete") {
                            action = Some("delete".into());
                            close = true;
                        }
                        ui.add_space(4.0);
                    });
            });

        // FIX: use cumulative_pass_nr() instead of the removed frame_nr().
        // Guard: only close on click-outside after the pass that opened this menu,
        // so the right-click that spawned it doesn't immediately close it.
        if ctx.cumulative_pass_nr() > opened_frame {
            let menu_rect = area_resp.response.rect;
            let clicked_outside = ctx
                .input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked())
                && !menu_rect.contains(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default()));
            if clicked_outside {
                close = true;
            }
        }

        if close {
            self.context_menu = None;
        }
        let display_idx = self.filtered_entries.iter().position(|&e| e == ei);

        if let Some(act) = action {
            match act.as_str() {
                "open" => {
                    if eis_dir {
                        self.navigate_to(epath);
                    } else {
                        Self::open_file(&epath);
                    }
                }
                "rename" => {
                    if let Some(di) = display_idx {
                        self.renaming = Some((di, ename.clone()));
                        self.selected_file = Some(di);
                    }
                }
                "copy" => {
                    self.clipboard = Some((epath, FileOperation::Copy));
                    self.push_notification(
                        format!("Copied \"{}\"", ename),
                        Color32::from_rgb(80, 180, 120),
                    );
                }
                "cut" => {
                    self.clipboard = Some((epath, FileOperation::Cut));
                    self.push_notification(
                        format!("Cut \"{}\"", ename),
                        Color32::from_rgb(210, 160, 50),
                    );
                }
                "paste" => {
                    self.paste_file();
                }
                "delete" => {
                    if let Some(di) = display_idx {
                        self.selected_file = Some(di);
                    }
                    self.delete_file();
                }
                "props" => {
                    self.properties_dialog = Some(epath);
                }
                "share_lan" => {
                    if !eis_dir {
                        self.start_lan_discover(epath);
                    } else {
                        self.push_notification(
                            "LAN send: select a file, not a folder".into(),
                            col_warn(ctx),
                        );
                    }
                }
                "share_link" => {
                    let link = format!("anvel://share/{:016x}", fnv_hash(&ename));
                    ctx.copy_text(link);
                    self.push_notification(
                        "Share link copied to clipboard".into(),
                        col_success(ctx),
                    );
                }
                _ => {}
            }
        }
    }

    // ── LAN dialog ────────────────────────────────────────────────────────────

    fn show_lan_dialog(&mut self, ctx: &egui::Context) {
        if matches!(self.lan_state, LanTransferState::Idle) {
            return;
        }

        let mut close = false;
        let mut send_to: Option<LanPeer> = None;

        let bg = col_surface2(ctx);
        let border = col_border(ctx);
        let txt = col_text(ctx);
        let t2 = col_text2(ctx);
        let ac = col_accent(ctx);

        egui::Window::new("🌐  LAN File Transfer")
            .collapsible(false).resizable(false).min_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, border))
                .corner_radius(12.0))
            .show(ctx, |ui| {
                if let Some(ref path) = self.lan_file_path {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("File:").color(t2).size(12.0));
                        ui.label(RichText::new(path.file_name().unwrap_or_default().to_string_lossy()).strong().color(txt));
                    });
                    ui.add_space(8.0);
                }

                match &self.lan_state.clone() {
                    LanTransferState::Discovering => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Scanning local network for peers…").color(txt));
                        });
                        ui.add_space(6.0);
                        ui.label(RichText::new("Make sure the other device is running Anvel and on the same network.")
                            .color(t2).size(11.0));
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    LanTransferState::Ready(peers) => {
                        if peers.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("📡").size(32.0));
                                ui.label(RichText::new("No peers found on the local network.").color(txt));
                                ui.label(RichText::new("Ensure the other device is running Anvel on the same WiFi/LAN.")
                                    .color(t2).size(11.0));
                            });
                        } else {
                            ui.label(RichText::new("Select a device to send to:").color(t2).size(12.0));
                            ui.add_space(6.0);
                            for peer in peers {
                                let peer = peer.clone();
                                egui::Frame::new()
                                    .fill(col_surface3(ctx))
                                    .corner_radius(8.0)
                                    .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 })
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("💻").size(20.0));
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(&peer.display).strong().color(txt));
                                                ui.label(RichText::new(peer.addr.to_string()).color(t2).size(11.0));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button(RichText::new("Send →").color(ac)).clicked() {
                                                    send_to = Some(peer.clone());
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        }
                    }
                    LanTransferState::Sending { peer_name, progress } => {
                        let progress = *progress;
                        ui.label(RichText::new(format!("Sending to {}…", peer_name)).color(txt));
                        ui.add_space(8.0);
                        ui.add(egui::ProgressBar::new(progress).animate(true).show_percentage());
                        ctx.request_repaint_after(std::time::Duration::from_millis(50));
                    }
                    LanTransferState::Done(msg) => {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("✅").size(36.0));
                            ui.label(RichText::new(msg).strong().color(col_success(ctx)).size(15.0));
                        });
                    }
                    LanTransferState::Err(e) => {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("❌").size(36.0));
                            ui.label(RichText::new(e).color(col_danger(ctx)));
                        });
                    }
                    LanTransferState::Idle => {}
                }

                ui.add_space(12.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if matches!(&self.lan_state, LanTransferState::Ready(_) | LanTransferState::Err(_)) {
                        if ui.button("🔄 Rescan").clicked() {
                            if let Some(path) = self.lan_file_path.clone() { self.start_lan_discover(path); }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() { close = true; }
                    });
                });
            });

        if let Some(peer) = send_to {
            self.start_lan_send(peer);
        }
        if close {
            self.lan_state = LanTransferState::Idle;
            self.lan_file_path = None;
        }
    }

    // ── Properties dialog ─────────────────────────────────────────────────────

    fn show_properties_dialog(&mut self, ctx: &egui::Context) {
        let Some(ref path) = self.properties_dialog.clone() else {
            return;
        };
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let md = fs::metadata(path).ok();
        let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
        let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let mtime = md.as_ref().and_then(|m| m.modified().ok());
        let mut open = true;

        egui::Window::new(format!("Properties — {}", name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .min_width(320.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("props")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Name:").strong());
                        ui.label(&name);
                        ui.end_row();
                        ui.label(RichText::new("Type:").strong());
                        ui.label(if is_dir { "Folder" } else { "File" });
                        ui.end_row();
                        ui.label(RichText::new("Size:").strong());
                        ui.label(Self::format_size(size));
                        ui.end_row();
                        ui.label(RichText::new("Modified:").strong());
                        ui.label(Self::format_time(mtime));
                        ui.end_row();
                        ui.label(RichText::new("Path:").strong());
                        ui.label(
                            RichText::new(path.to_string_lossy())
                                .size(11.0)
                                .color(col_text2(ctx)),
                        );
                        ui.end_row();
                    });
            });
        if !open {
            self.properties_dialog = None;
        }
    }

    // ── Notifications ─────────────────────────────────────────────────────────

    fn show_notifications(&self, ctx: &egui::Context) {
        if self.notifications.is_empty() {
            return;
        }
        let screen = ctx.viewport_rect();
        let mut y = screen.max.y - 16.0;
        let bg = col_surface2(ctx);
        let fg = col_text(ctx);

        for notif in self.notifications.iter().rev() {
            let age = notif.created.elapsed().as_secs_f32();
            let alpha = ((5.0 - age).clamp(0.0, 1.0) * 255.0) as u8;
            let c = notif.color;

            egui::Area::new(egui::Id::new(format!("notif_{:p}", notif)))
                .fixed_pos(egui::pos2(screen.max.x - 350.0, y - 46.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(
                            bg.r(),
                            bg.g(),
                            bg.b(),
                            alpha,
                        ))
                        .stroke(egui::Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                        ))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin {
                            left: 14,
                            right: 14,
                            top: 9,
                            bottom: 9,
                        })
                        .show(ui, |ui| {
                            ui.set_max_width(310.0);
                            ui.label(
                                RichText::new(&notif.message)
                                    .color(Color32::from_rgba_unmultiplied(
                                        fg.r(),
                                        fg.g(),
                                        fg.b(),
                                        alpha,
                                    ))
                                    .size(13.0),
                            );
                        });
                });
            y -= 54.0;
        }
    }
}

// ─── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark or light visuals each frame
        ctx.set_visuals(if self.dark_mode {
            let mut v = egui::Visuals::dark();
            v.window_fill = Color32::from_rgb(24, 24, 24);
            v.panel_fill = Color32::from_rgb(18, 18, 18);
            v.faint_bg_color = Color32::from_rgb(26, 26, 26);
            v.extreme_bg_color = Color32::from_rgb(12, 12, 12);
            v.override_text_color = Some(Color32::from_rgb(215, 215, 215));
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 28, 28);
            v.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 60));
            v.widgets.inactive.bg_fill = Color32::from_rgb(36, 36, 36);
            v.widgets.hovered.bg_fill = Color32::from_rgb(48, 48, 48);
            v.widgets.active.bg_fill = Color32::from_rgb(60, 60, 60);
            v.selection.bg_fill = Color32::from_rgb(55, 55, 75);
            v
        } else {
            let mut v = egui::Visuals::light();
            v.window_fill = Color32::WHITE;
            v.panel_fill = Color32::from_rgb(248, 248, 248);
            v.faint_bg_color = Color32::from_rgb(242, 242, 242);
            v.extreme_bg_color = Color32::WHITE;
            v.override_text_color = Some(Color32::from_rgb(20, 20, 20));
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(240, 240, 240);
            v.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 200));
            v.widgets.inactive.bg_fill = Color32::from_rgb(232, 232, 232);
            v.widgets.hovered.bg_fill = Color32::from_rgb(220, 220, 220);
            v.widgets.active.bg_fill = Color32::from_rgb(200, 200, 200);
            v.selection.bg_fill = Color32::from_rgb(190, 210, 245);
            v
        });

        // Poll LAN
        self.poll_lan();
        if matches!(
            self.lan_state,
            LanTransferState::Discovering | LanTransferState::Sending { .. }
        ) {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        // Expire notifications
        self.notifications
            .retain(|n| n.created.elapsed().as_secs() < 5);

        // Keyboard shortcuts
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
            if i.key_pressed(egui::Key::Backspace)
                || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
            {
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

        self.show_top_panel(ctx);
        self.show_toolbar(ctx);
        self.show_bottom_panel(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(col_surface(ctx))
                    .inner_margin(egui::Margin::same(6)),
            )
            .show(ctx, |ui| {
                self.show_file_list(ui, ctx);
            });

        self.show_context_menu(ctx);
        self.show_lan_dialog(ctx);
        self.show_properties_dialog(ctx);
        self.show_notifications(ctx);
    }
}

// ─── LAN networking ───────────────────────────────────────────────────────────

fn lan_receive_server(tx: std::sync::mpsc::Sender<LanServerMsg>, default_save_dir: PathBuf) {
    thread::spawn(lan_advertise_loop);

    let listener = match TcpListener::bind(("0.0.0.0", LAN_TRANSFER_PORT)) {
        Ok(l) => l,
        Err(e) => {
            let _ = tx.send(LanServerMsg::Error(format!(
                "Cannot listen on port {}: {}",
                LAN_TRANSFER_PORT, e
            )));
            return;
        }
    };

    for stream in listener.incoming().flatten() {
        let tx2 = tx.clone();
        // Always save to the default_save_dir (home). The UI layer then
        // optionally reloads the directory when it sees the FileReceived message.
        let save_dir = default_save_dir.clone();
        thread::spawn(move || match lan_receive_file(stream, &save_dir) {
            Ok((name, dest)) => {
                let _ = tx2.send(LanServerMsg::FileReceived { name, dest });
            }
            Err(e) => {
                let _ = tx2.send(LanServerMsg::Error(e));
            }
        });
    }
}

fn lan_advertise_loop() {
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", LAN_DISCOVER_PORT)) else {
        return;
    };
    let hostname = hostname();
    let mut buf = [0u8; 256];
    loop {
        if let Ok((_, from)) = sock.recv_from(&mut buf) {
            let msg = String::from_utf8_lossy(&buf);
            if msg.trim_matches('\0').starts_with("ANVEL_DISCOVER") {
                let _ = sock.send_to(format!("ANVEL_PEER:{}", hostname).as_bytes(), from);
            }
        }
    }
}

fn lan_advertise_once() {
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return;
    };
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let hostname = hostname();
    let mut buf = [0u8; 256];
    if let Ok((_, from)) = sock.recv_from(&mut buf) {
        let _ = sock.send_to(format!("ANVEL_PEER:{}", hostname).as_bytes(), from);
    }
}

fn lan_send_file(path: &Path, addr: std::net::IpAddr) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let name_bytes = name.as_bytes();

    let mut stream = TcpStream::connect((addr, LAN_TRANSFER_PORT))
        .map_err(|e| format!("Cannot connect to peer: {}", e))?;

    stream
        .write_all(&(name_bytes.len() as u32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(name_bytes).map_err(|e| e.to_string())?;
    stream
        .write_all(&(data.len() as u64).to_le_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&data).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn lan_receive_file(mut stream: TcpStream, save_dir: &Path) -> Result<(String, PathBuf), String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let name_len = u32::from_le_bytes(len_buf) as usize;
    if name_len > 4096 {
        return Err("Malformed packet: name too long".into());
    }

    let mut name_buf = vec![0u8; name_len];
    stream
        .read_exact(&mut name_buf)
        .map_err(|e| e.to_string())?;
    let name = String::from_utf8_lossy(&name_buf).to_string();

    let mut size_buf = [0u8; 8];
    stream
        .read_exact(&mut size_buf)
        .map_err(|e| e.to_string())?;
    let file_size = u64::from_le_bytes(size_buf) as usize;
    if file_size > 2 * 1024 * 1024 * 1024 {
        return Err("File too large (>2 GB)".into());
    }

    let mut data = vec![0u8; file_size];
    stream.read_exact(&mut data).map_err(|e| e.to_string())?;

    let dest = save_dir.join(&name);
    fs::write(&dest, &data).map_err(|e| e.to_string())?;
    Ok((name, dest))
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "this-device".into())
        .trim()
        .to_string()
}

// ─── Free helpers ─────────────────────────────────────────────────────────────

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn fnv_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        h.wrapping_mul(0x100000001b3).wrapping_add(b as u64)
    })
}

fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str, ctx: &egui::Context) -> bool {
    ui.add(
        egui::Button::new(
            RichText::new(format!("{}  {}", icon, label))
                .color(col_text(ctx))
                .size(13.0),
        )
        .frame(false)
        .min_size(Vec2::new(200.0, 24.0)),
    )
    .clicked()
}

fn menu_item_danger(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    ui.add(
        egui::Button::new(
            RichText::new(format!("{}  {}", icon, label))
                .color(Color32::from_rgb(210, 70, 70))
                .size(13.0),
        )
        .frame(false)
        .min_size(Vec2::new(200.0, 24.0)),
    )
    .clicked()
}
