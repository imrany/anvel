use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::thread;
use std::net::{TcpListener, TcpStream, UdpSocket};
use egui::{Color32, RichText, Vec2};

// ─── AI Config ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum AiProvider { Claude, Gemini }

#[derive(Clone)]
struct AiConfig {
    provider:  AiProvider,
    api_key:   String,
    model:     String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Claude,
            api_key:  String::new(),
            model:    "claude-haiku-4-5-20251001".into(),
        }
    }
}

impl AiConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("anvel").join("settings"))
    }

    fn load() -> Self {
        let Some(path) = Self::config_path() else { return Self::default() };
        let Ok(text) = fs::read_to_string(&path) else { return Self::default() };
        let mut cfg = Self::default();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "provider" => cfg.provider = if v.trim() == "gemini" { AiProvider::Gemini } else { AiProvider::Claude },
                    "api_key"  => cfg.api_key  = v.trim().to_string(),
                    "model"    => cfg.model     = v.trim().to_string(),
                    _ => {}
                }
            }
        }
        cfg
    }

    fn save(&self) {
        let Some(path) = Self::config_path() else { return };
        if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
        let text = format!(
            "provider={}\napi_key={}\nmodel={}\n",
            if self.provider == AiProvider::Gemini { "gemini" } else { "claude" },
            self.api_key,
            self.model,
        );
        let _ = fs::write(&path, text);
    }

    fn provider_label(&self) -> &'static str {
        match self.provider { AiProvider::Claude => "Claude", AiProvider::Gemini => "Gemini" }
    }

    fn provider_color(&self) -> Color32 {
        match self.provider {
            AiProvider::Claude => Color32::from_rgb(200, 130, 80),
            AiProvider::Gemini => Color32::from_rgb(66, 153, 225),
        }
    }

    fn default_model(&self) -> &'static str {
        match self.provider {
            AiProvider::Claude => "claude-haiku-4-5-20251001",
            AiProvider::Gemini => "gemini-2.0-flash",
        }
    }
}

// ─── Chat ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ChatMessage {
    role:    ChatRole,
    content: String,
}

#[derive(Clone, PartialEq)]
enum ChatRole { User, Assistant }

#[derive(Clone)]
struct MentionedFile {
    name:   String,
    path:   PathBuf,
    is_dir: bool,
    size:   u64,
    ext:    String,
}

// ─── LAN transfer ─────────────────────────────────────────────────────────────

const LAN_DISCOVER_PORT: u16 = 44444;
const LAN_TRANSFER_PORT: u16 = 44445;

#[derive(Clone)]
struct LanPeer {
    display: String,
    addr:    std::net::IpAddr,
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

// ─── Other UI types ───────────────────────────────────────────────────────────

struct ContextMenuState {
    pos:       egui::Pos2,
    entry_idx: usize,
}

#[derive(Clone)]
struct Notification {
    message: String,
    color:   Color32,
    created: std::time::Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum SortBy { Name, Size, Modified, Type }

#[derive(Clone, Copy, PartialEq)]
enum ViewMode { List, Details }

#[derive(Clone)]
enum FileOperation { Copy, Cut }

struct DirEntry {
    name:      String,
    path:      PathBuf,
    is_dir:    bool,
    size:      u64,
    modified:  Option<SystemTime>,
    extension: String,
}

// ─── Main struct ──────────────────────────────────────────────────────────────

pub struct FileExplorer {
    // ── file browser ─────────────────────────────────────────────────────────
    current_path:     PathBuf,
    entries:          Vec<DirEntry>,
    filtered_entries: Vec<usize>,
    selected_file:    Option<usize>,
    error_message:    Option<String>,
    search_query:     String,
    clipboard:        Option<(PathBuf, FileOperation)>,
    show_hidden:      bool,
    sort_by:          SortBy,
    view_mode:        ViewMode,
    path_history:     Vec<PathBuf>,
    history_index:    usize,
    renaming:         Option<(usize, String)>,
    properties_dialog: Option<PathBuf>,
    notifications:    Vec<Notification>,
    context_menu:     Option<ContextMenuState>,

    // ── AI chat ───────────────────────────────────────────────────────────────
    chat_messages:       Vec<ChatMessage>,
    chat_input:          String,
    ai_loading:          bool,
    ai_response_receiver: Option<std::sync::mpsc::Receiver<String>>,
    at_mode:             bool,
    at_query:            String,
    mentioned_files:     Vec<MentionedFile>,
    ai_config:           AiConfig,
    show_ai_settings:    bool,
    ai_settings_draft:   AiConfig,

    // ── LAN transfer ─────────────────────────────────────────────────────────
    lan_state:          LanTransferState,
    lan_file_path:      Option<PathBuf>,
    lan_discover_rx:    Option<std::sync::mpsc::Receiver<Vec<LanPeer>>>,
    lan_transfer_rx:    Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    lan_server_rx:      Option<std::sync::mpsc::Receiver<LanServerMsg>>,
    lan_receive_dir:    PathBuf,
}

impl Default for FileExplorer {
    fn default() -> Self {
        let home       = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let ai_config  = AiConfig::load();
        let draft      = ai_config.clone();

        // Start LAN server
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

            chat_messages: vec![ChatMessage {
                role: ChatRole::Assistant,
                content: "Hi! I'm your AI file assistant.\nType @ to mention a file, or ask me anything about your files.\n\nConfigure your AI provider with the ⚙ button above.".into(),
            }],
            chat_input: String::new(),
            ai_loading: false,
            ai_response_receiver: None,
            at_mode: false,
            at_query: String::new(),
            mentioned_files: Vec::new(),
            ai_config,
            show_ai_settings: false,
            ai_settings_draft: draft,

            lan_state: LanTransferState::Idle,
            lan_file_path: None,
            lan_discover_rx: None,
            lan_transfer_rx: None,
            lan_server_rx: Some(srv_rx),
            lan_receive_dir: home.clone(),
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
                    if !self.show_hidden && name.starts_with('.') { continue; }
                    if let Ok(metadata) = entry.metadata() {
                        let extension = entry.path()
                            .extension().and_then(|e| e.to_str())
                            .unwrap_or("").to_string();
                        self.entries.push(DirEntry {
                            name, path: entry.path(),
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
            Err(e) => { self.error_message = Some(format!("Failed to read directory: {}", e)); }
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
        if path == self.current_path { return; }
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
        if let Some(parent) = self.current_path.parent() { self.navigate_to(parent.to_path_buf()); }
    }

    fn new_folder(&mut self) {
        let mut candidate = self.current_path.join("New Folder");
        let mut n = 1u32;
        while candidate.exists() { candidate = self.current_path.join(format!("New Folder ({})", n)); n += 1; }
        let _ = fs::create_dir(&candidate);
        self.load_directory(&self.current_path.clone());
        self.push_notification("Created \"New Folder\"".into(), Color32::from_rgb(80, 180, 120));
    }

    fn open_file(path: &Path) {
        #[cfg(target_os = "linux")]   { let _ = std::process::Command::new("xdg-open").arg(path).spawn(); }
        #[cfg(target_os = "macos")]   { let _ = std::process::Command::new("open").arg(path).spawn(); }
        #[cfg(target_os = "windows")] { let _ = std::process::Command::new("cmd").args(["/C", "start", "", path.to_str().unwrap_or("")]).spawn(); }
    }

    fn copy_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    self.clipboard = Some((e.path.clone(), FileOperation::Copy));
                    self.push_notification(format!("Copied \"{}\"", e.name), Color32::from_rgb(80, 180, 120));
                }
            }
        }
    }

    fn cut_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    self.clipboard = Some((e.path.clone(), FileOperation::Cut));
                    self.push_notification(format!("Cut \"{}\"", e.name), Color32::from_rgb(240, 180, 60));
                }
            }
        }
    }

    fn paste_file(&mut self) {
        if let Some((source, operation)) = &self.clipboard.clone() {
            let dest = self.current_path.join(source.file_name().unwrap());
            match operation {
                FileOperation::Copy => {
                    if source.is_file()     { let _ = fs::copy(source, &dest); }
                    else if source.is_dir() { let _ = copy_dir_all(source, &dest); }
                }
                FileOperation::Cut => { let _ = fs::rename(source, &dest); self.clipboard = None; }
            }
            self.load_directory(&self.current_path.clone());
            self.push_notification("Pasted successfully".into(), Color32::from_rgb(80, 180, 120));
        }
    }

    fn delete_file(&mut self) {
        if let Some(idx) = self.selected_file {
            if let Some(&ei) = self.filtered_entries.get(idx) {
                if let Some(e) = self.entries.get(ei) {
                    let result = if e.is_dir { fs::remove_dir_all(&e.path) } else { fs::remove_file(&e.path) };
                    if let Err(err) = result {
                        self.error_message = Some(format!("Failed to delete: {}", err));
                    } else {
                        let name = e.name.clone();
                        self.selected_file = None;
                        self.load_directory(&self.current_path.clone());
                        self.push_notification(format!("Deleted \"{}\"", name), Color32::from_rgb(230, 80, 80));
                    }
                }
            }
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        let i  = self.selected_file?;
        let ei = self.filtered_entries.get(i)?;
        Some(self.entries.get(*ei)?.path.clone())
    }

    fn push_notification(&mut self, message: String, color: Color32) {
        self.notifications.push(Notification { message, color, created: std::time::Instant::now() });
    }

    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut ui   = 0;
        while size >= 1024.0 && ui < UNITS.len() - 1 { size /= 1024.0; ui += 1; }
        if ui == 0 { format!("{} {}", size as u64, UNITS[ui]) } else { format!("{:.2} {}", size, UNITS[ui]) }
    }

    fn format_time(time: Option<SystemTime>) -> String {
        time.and_then(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH).ok().map(|d| {
                let diff = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap().as_secs().saturating_sub(d.as_secs());
                if      diff < 60      { "Just now".into() }
                else if diff < 3600   { format!("{} min ago", diff / 60) }
                else if diff < 86400  { format!("{} hours ago", diff / 3600) }
                else if diff < 604800 { format!("{} days ago", diff / 86400) }
                else                  { format!("{} weeks ago", diff / 604800) }
            })
        }).unwrap_or_else(|| "Unknown".into())
    }

    fn get_file_icon(entry: &DirEntry) -> &'static str {
        if entry.is_dir { return "📁"; }
        match entry.extension.to_lowercase().as_str() {
            "rs"                                       => "🦀",
            "toml"                                     => "⚙️",
            "md"                                       => "📝",
            "txt"                                      => "📄",
            "pdf"                                      => "📕",
            "png"|"jpg"|"jpeg"|"gif"|"svg"|"bmp"       => "🖼️",
            "mp3"|"wav"|"ogg"|"flac"                   => "🎵",
            "mp4"|"avi"|"mkv"|"mov"                    => "🎬",
            "zip"|"tar"|"gz"|"7z"|"rar"                => "📦",
            "js"|"ts"|"jsx"|"tsx"                      => "🟨",
            "py"                                       => "🐍",
            "java"                                     => "☕",
            "cpp"|"c"|"h"                              => "⚡",
            "html"|"css"                               => "🌐",
            "json"|"xml"|"yaml"|"yml"                  => "📋",
            _                                          => "📄",
        }
    }

    fn render_breadcrumbs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Path:").color(Color32::from_rgb(100, 100, 120)).size(12.0));
            let mut components: Vec<PathBuf> = Vec::new();
            let mut current = self.current_path.as_path();
            components.push(current.to_path_buf());
            while let Some(parent) = current.parent() {
                if parent.as_os_str().is_empty() { break; }
                components.push(parent.to_path_buf());
                current = parent;
            }
            components.reverse();
            let mut nav: Option<PathBuf> = None;
            for (i, comp) in components.iter().enumerate() {
                if i > 0 { ui.label(RichText::new("/").color(Color32::from_rgb(80, 80, 100))); }
                let name = comp.file_name().and_then(|n| n.to_str()).unwrap_or_else(|| comp.to_str().unwrap_or(""));
                if ui.link(RichText::new(name).size(12.0)).clicked() { nav = Some(comp.clone()); }
            }
            if let Some(p) = nav { self.navigate_to(p); }
        });
    }
}

// ─── AI chat methods ──────────────────────────────────────────────────────────

impl FileExplorer {
    fn send_ai_message(&mut self) {
        let input = self.chat_input.trim().to_string();
        if input.is_empty() || self.ai_loading { return; }
        if self.ai_config.api_key.trim().is_empty() {
            self.chat_messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: "⚙ Please configure your API key first — click the gear icon above.".into(),
            });
            return;
        }

        let ctx_note = if self.mentioned_files.is_empty() { String::new() } else {
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

        let history: Vec<(String, String)> = self.chat_messages.iter().enumerate().map(|(i, m)| {
            let role    = if m.role == ChatRole::User { "user" } else { "assistant" }.to_string();
            let content = if i == self.chat_messages.len() - 1 { full.clone() } else { m.content.clone() };
            (role, content)
        }).collect();

        let (tx, rx) = std::sync::mpsc::channel::<String>();
        self.ai_response_receiver = Some(rx);
        let cfg = self.ai_config.clone();

        thread::spawn(move || {
            let reply = match cfg.provider {
                AiProvider::Claude => call_claude(&cfg.api_key, &cfg.model, history),
                AiProvider::Gemini => call_gemini(&cfg.api_key, &cfg.model, history),
            }.unwrap_or_else(|e| format!("Error: {}", e));
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
            .map(|(i, _)| i).take(8).collect()
    }
}

// ─── LAN transfer methods ─────────────────────────────────────────────────────

impl FileExplorer {
    fn start_lan_discover(&mut self, file_path: PathBuf) {
        self.lan_file_path = Some(file_path);
        self.lan_state     = LanTransferState::Discovering;
        let (tx, rx)       = std::sync::mpsc::channel::<Vec<LanPeer>>();
        self.lan_discover_rx = Some(rx);

        thread::spawn(move || {
            let mut peers: Vec<LanPeer> = Vec::new();

            // Bind UDP socket and broadcast discovery ping
            if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
                let _ = sock.set_broadcast(true);
                let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(200)));
                let ping = b"ANVEL_DISCOVER";
                let _ = sock.send_to(ping, ("255.255.255.255", LAN_DISCOVER_PORT));

                // Also send to local subnets
                for subnet in ["192.168.1.255", "192.168.0.255", "10.0.0.255"] {
                    let _ = sock.send_to(ping, (subnet, LAN_DISCOVER_PORT));
                }

                // Listen for responses for up to 2 seconds
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let mut buf = [0u8; 256];
                while std::time::Instant::now() < deadline {
                    if let Ok((n, addr)) = sock.recv_from(&mut buf) {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        if let Some(name) = msg.strip_prefix("ANVEL_PEER:") {
                            let peer = LanPeer {
                                display: format!("{} ({})", name.trim(), addr.ip()),
                                addr:    addr.ip(),
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

        // Simultaneously advertise ourselves
        thread::spawn(|| {
            let _ = lan_advertise_once();
        });
    }

    fn start_lan_send(&mut self, peer: LanPeer) {
        let Some(file_path) = self.lan_file_path.clone() else { return };
        let peer_name = peer.display.clone();
        self.lan_state = LanTransferState::Sending { peer_name: peer_name.clone(), progress: 0.0 };

        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        self.lan_transfer_rx = Some(rx);

        thread::spawn(move || {
            let result = lan_send_file(&file_path, peer.addr);
            let _ = tx.send(result);
        });
    }

    fn poll_lan(&mut self) {
        // Poll discovery
        if let Some(rx) = &self.lan_discover_rx {
            if let Ok(peers) = rx.try_recv() {
                self.lan_discover_rx = None;
                self.lan_state = LanTransferState::Ready(peers);
            }
        }

        // Poll transfer
        if let Some(rx) = &self.lan_transfer_rx {
            if let Ok(result) = rx.try_recv() {
                self.lan_transfer_rx = None;
                match result {
                    Ok(())   => self.lan_state = LanTransferState::Done("File sent successfully!".into()),
                    Err(e)   => self.lan_state = LanTransferState::Err(e),
                }
            }
        }

        // Poll incoming files
        let server_msgs: Vec<LanServerMsg> = if let Some(rx) = &self.lan_server_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() { msgs.push(msg); }
            msgs
        } else { Vec::new() };
        for msg in server_msgs {
            match msg {
                LanServerMsg::FileReceived { name, dest: _ } => {
                    self.push_notification(format!("📥 Received: {}", name), Color32::from_rgb(80, 200, 140));
                }
                LanServerMsg::Error(e) => {
                    self.push_notification(format!("LAN error: {}", e), Color32::from_rgb(220, 80, 80));
                }
            }
        }
    }
}

// ─── Panel rendering ──────────────────────────────────────────────────────────

impl FileExplorer {
    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(22, 22, 32))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let back_en    = self.history_index > 0;
                    let forward_en = self.history_index < self.path_history.len() - 1;

                    ui.add_enabled_ui(back_en, |ui| {
                        if ui.button(RichText::new("◀").size(15.0)).on_hover_text("Back (Alt+←)").clicked() { self.go_back(); }
                    });
                    ui.add_enabled_ui(forward_en, |ui| {
                        if ui.button(RichText::new("▶").size(15.0)).on_hover_text("Forward (Alt+→)").clicked() { self.go_forward(); }
                    });
                    if ui.button(RichText::new("⬆").size(15.0)).on_hover_text("Up").clicked()   { self.go_up(); }
                    if ui.button(RichText::new("🏠").size(15.0)).on_hover_text("Home").clicked() {
                        if let Some(home) = dirs::home_dir() { self.navigate_to(home); }
                    }
                    if ui.button(RichText::new("🔄").size(15.0)).on_hover_text("Refresh (F5)").clicked() {
                        self.load_directory(&self.current_path.clone());
                    }

                    ui.separator();

                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("🔍  Search…")
                            .desired_width(200.0),
                    );
                    if search_resp.changed() { self.apply_filter(); }

                    ui.separator();
                    if ui.button("📁  New Folder").clicked() { self.new_folder(); }
                    if ui.button(if self.show_hidden { "👁  Hide Hidden" } else { "👁  Show Hidden" }).clicked() {
                        self.show_hidden = !self.show_hidden;
                        self.load_directory(&self.current_path.clone());
                    }
                });

                ui.add_space(3.0);
                self.render_breadcrumbs(ui);
                ui.add_space(3.0);
            });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(26, 26, 36))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sort:").color(Color32::from_rgb(100, 100, 120)).size(12.0));
                    for (label, s) in [("Name", SortBy::Name), ("Size", SortBy::Size), ("Modified", SortBy::Modified), ("Type", SortBy::Type)] {
                        if ui.selectable_label(self.sort_by == s, RichText::new(label).size(12.0)).clicked() {
                            self.sort_by = s; self.sort_entries(); self.apply_filter();
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("View:").color(Color32::from_rgb(100, 100, 120)).size(12.0));
                    if ui.selectable_label(self.view_mode == ViewMode::List,    RichText::new("List").size(12.0)).clicked()    { self.view_mode = ViewMode::List; }
                    if ui.selectable_label(self.view_mode == ViewMode::Details, RichText::new("Details").size(12.0)).clicked() { self.view_mode = ViewMode::Details; }

                    if let Some((p, op)) = &self.clipboard {
                        ui.separator();
                        let label = match op { FileOperation::Copy => "📋 Clipboard: ", FileOperation::Cut => "✂️ Clipboard: " };
                        let fname = p.file_name().unwrap_or_default().to_string_lossy();
                        ui.label(RichText::new(format!("{}{}", label, fname)).color(Color32::from_rgb(100, 160, 255)).size(12.0));
                    }
                });
            });
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(18, 18, 26))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} item{}", self.filtered_entries.len(),
                        if self.filtered_entries.len() == 1 { "" } else { "s" }))
                        .color(Color32::from_rgb(100, 100, 130)).size(12.0));

                    if let Some(idx) = self.selected_file {
                        if let Some(&ei) = self.filtered_entries.get(idx) {
                            if let Some(e) = self.entries.get(ei) {
                                ui.separator();
                                ui.label(RichText::new(&e.name).color(Color32::from_rgb(130, 160, 220)).size(12.0));
                                if !e.is_dir {
                                    ui.separator();
                                    ui.label(RichText::new(Self::format_size(e.size)).color(Color32::from_rgb(100, 100, 130)).size(12.0));
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Ctrl+C/X/V  Del  F5  Right-click  @ in chat")
                            .color(Color32::from_rgb(80, 80, 100)).size(11.0));
                    });
                });
            });
    }

    // ── File list ─────────────────────────────────────────────────────────────

    fn show_file_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(ref err) = self.error_message.clone() {
            ui.colored_label(Color32::from_rgb(220, 80, 80), RichText::new(err).strong());
            ui.add_space(8.0);
        }

        if self.filtered_entries.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("📂").size(48.0));
                ui.add_space(8.0);
                ui.label(RichText::new("No files here").size(18.0).color(Color32::from_rgb(100, 100, 130)));
            });
            return;
        }

        let mut navigate_to_path: Option<PathBuf>             = None;
        let mut ctx_menu:         Option<(egui::Pos2, usize)> = None;
        let mut rename_done:      Option<(usize, String)>      = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.view_mode {
                ViewMode::List => {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 1.0);
                    for (i, &ei) in self.filtered_entries.iter().enumerate() {
                        if let Some(entry) = self.entries.get(ei) {
                            let is_sel = self.selected_file == Some(i);
                            if self.renaming.as_ref().map(|(ri, _)| *ri) == Some(i) {
                                let (_, nn) = self.renaming.as_mut().unwrap();
                                let r = ui.add(egui::TextEdit::singleline(nn).desired_width(220.0));
                                if r.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                    rename_done = self.renaming.clone();
                                }
                                if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                    rename_done = Some((i, String::new()));
                                }
                                continue;
                            }
                            let resp = ui.selectable_label(is_sel,
                                RichText::new(format!("{} {}", Self::get_file_icon(entry), entry.name)).size(14.0));
                            if resp.clicked()        { self.selected_file = Some(i); }
                            if resp.double_clicked() {
                                if entry.is_dir { navigate_to_path = Some(entry.path.clone()); }
                                else            { Self::open_file(&entry.path); }
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
                    let mut nav:    Option<PathBuf>             = None;
                    let mut cmenu:  Option<(egui::Pos2, usize)> = None;
                    let mut rndone: Option<(usize, String)>     = None;

                    TableBuilder::new(ui)
                        .striped(true)
                        .sense(egui::Sense::click())
                        .column(Column::auto().at_least(280.0))
                        .column(Column::auto().at_least(80.0))
                        .column(Column::auto().at_least(110.0))
                        .column(Column::auto().at_least(70.0))
                        .header(22.0, |mut h| {
                            h.col(|ui| { ui.strong("Name"); });
                            h.col(|ui| { ui.strong("Size"); });
                            h.col(|ui| { ui.strong("Modified"); });
                            h.col(|ui| { ui.strong("Type"); });
                        })
                        .body(|mut body| {
                            for (i, &ei) in self.filtered_entries.iter().enumerate() {
                                if let Some(entry) = self.entries.get(ei) {
                                    let is_sel = self.selected_file == Some(i);
                                    body.row(22.0, |mut row| {
                                        row.set_selected(is_sel);
                                        let (_, nr) = row.col(|ui| {
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
                                                ui.label(RichText::new(Self::format_size(entry.size)).color(Color32::from_rgb(120, 120, 150)));
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.label(RichText::new(Self::format_time(entry.modified)).color(Color32::from_rgb(120, 120, 150)));
                                        });
                                        row.col(|ui| {
                                            let t = if entry.is_dir { "Folder" } else if entry.extension.is_empty() { "File" } else { &entry.extension };
                                            ui.label(RichText::new(t).color(Color32::from_rgb(120, 120, 150)));
                                        });
                                        if nr.clicked()        { self.selected_file = Some(i); }
                                        if nr.double_clicked() {
                                            if entry.is_dir { nav = Some(entry.path.clone()); }
                                            else            { Self::open_file(&entry.path); }
                                        }
                                        if nr.secondary_clicked() {
                                            if let Some(pos) = ctx.input(|inp| inp.pointer.interact_pos()) {
                                                cmenu = Some((pos, ei));
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
        let pos = cm.pos; let ei = cm.entry_idx;
        let (epath, ename, eis_dir, esize, eext) = {
            let Some(e) = self.entries.get(ei) else { self.context_menu = None; return };
            (e.path.clone(), e.name.clone(), e.is_dir, e.size, e.extension.clone())
        };
        let mut close = false;
        let mut action: Option<String> = None;

        egui::Area::new(egui::Id::new("ctx_menu"))
            .fixed_pos(pos).order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(28, 28, 40))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 90)))
                    .corner_radius(10.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            let icon = if eis_dir { "📁" } else { Self::get_file_icon(self.entries.get(ei).unwrap()) };
                            ui.label(RichText::new(format!("{} {}", icon, ename))
                                .color(Color32::from_rgb(180, 190, 220)).size(13.0).strong());
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        if menu_item(ui, "↩️", "Open")               { action = Some("open".into());   close = true; }
                        if menu_item(ui, "✏️", "Rename")             { action = Some("rename".into()); close = true; }
                        if menu_item(ui, "📋", "Copy")               { action = Some("copy".into());   close = true; }
                        if menu_item(ui, "✂️", "Cut")                { action = Some("cut".into());    close = true; }
                        if self.clipboard.is_some() {
                            if menu_item(ui, "📌", "Paste Here")     { action = Some("paste".into());  close = true; }
                        }
                        ui.separator();
                        if menu_item(ui, "🤖", "Ask AI about this")  { action = Some("ai".into());     close = true; }
                        if menu_item(ui, "ℹ️", "Properties")         { action = Some("props".into());  close = true; }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Share via LAN").color(Color32::from_rgb(110, 110, 150)).size(11.0));
                        });
                        if menu_item(ui, "🌐", "  Send over LAN")    { action = Some("share_lan".into()); close = true; }
                        if menu_item(ui, "🔗", "  Copy Share Link")  { action = Some("share_link".into()); close = true; }
                        ui.separator();
                        if menu_item_danger(ui, "🗑️", "Delete")      { action = Some("delete".into()); close = true; }
                        ui.add_space(4.0);
                    });
                if ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(ui.min_rect()) {
                    close = true;
                }
            });

        if close { self.context_menu = None; }
        let display_idx = self.filtered_entries.iter().position(|&e| e == ei);

        if let Some(act) = action {
            match act.as_str() {
                "open"   => { if eis_dir { self.navigate_to(epath); } else { Self::open_file(&epath); } }
                "rename" => {
                    if let Some(di) = display_idx {
                        self.renaming = Some((di, ename.clone()));
                        self.selected_file = Some(di);
                    }
                }
                "copy"  => { self.clipboard = Some((epath, FileOperation::Copy)); self.push_notification(format!("Copied \"{}\"", ename), Color32::from_rgb(80, 200, 120)); }
                "cut"   => { self.clipboard = Some((epath, FileOperation::Cut));  self.push_notification(format!("Cut \"{}\"", ename), Color32::from_rgb(240, 180, 60)); }
                "paste" => { self.paste_file(); }
                "delete" => { if let Some(di) = display_idx { self.selected_file = Some(di); } self.delete_file(); }
                "ai" => {
                    self.chat_input = format!("@{} ", ename);
                    self.mentioned_files.push(MentionedFile { name: ename.clone(), path: epath, is_dir: eis_dir, size: esize, ext: eext });
                    self.push_notification(format!("Ready — type your question about \"{}\" in the chat", ename), Color32::from_rgb(100, 180, 255));
                }
                "props"      => { self.properties_dialog = Some(epath); }
                "share_lan"  => {
                    let path = epath.clone();
                    if !eis_dir {
                        self.start_lan_discover(path);
                    } else {
                        self.push_notification("LAN send: select a file (not a folder)".into(), Color32::from_rgb(220, 160, 60));
                    }
                }
                "share_link" => {
                    let link = format!("anvel://share/{:016x}", fnv_hash(&ename));
                    ctx.copy_text(link.clone());
                    self.push_notification("Share link copied to clipboard".into(), Color32::from_rgb(80, 180, 120));
                }
                _ => {}
            }
        }
    }

    // ── LAN dialog ────────────────────────────────────────────────────────────

    fn show_lan_dialog(&mut self, ctx: &egui::Context) {
        if matches!(self.lan_state, LanTransferState::Idle) { return; }

        let mut close = false;
        let mut send_to: Option<LanPeer> = None;

        egui::Window::new("🌐  LAN File Transfer")
            .collapsible(false).resizable(false).min_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgb(22, 24, 36))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 80, 140)))
                .corner_radius(12.0))
            .show(ctx, |ui| {
                if let Some(ref path) = self.lan_file_path {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("File:").color(Color32::from_rgb(120, 130, 160)).size(12.0));
                        ui.label(RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                            .strong().color(Color32::from_rgb(180, 200, 240)));
                    });
                    ui.add_space(8.0);
                }

                match &self.lan_state.clone() {
                    LanTransferState::Discovering => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Scanning local network for peers…")
                                .color(Color32::from_rgb(160, 180, 220)));
                        });
                        ui.add_space(6.0);
                        ui.label(RichText::new("Make sure the other device is running Anvel and on the same network.")
                            .color(Color32::from_rgb(100, 110, 140)).size(11.0));
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }

                    LanTransferState::Ready(peers) => {
                        if peers.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0);
                                ui.label(RichText::new("📡").size(32.0));
                                ui.label(RichText::new("No peers found on the local network.")
                                    .color(Color32::from_rgb(180, 180, 200)));
                                ui.add_space(4.0);
                                ui.label(RichText::new("Ensure the other device is running Anvel\nand connected to the same WiFi/LAN.")
                                    .color(Color32::from_rgb(100, 110, 140)).size(11.0));
                            });
                        } else {
                            ui.label(RichText::new("Select a device to send to:")
                                .color(Color32::from_rgb(130, 150, 200)).size(12.0));
                            ui.add_space(6.0);
                            for peer in peers {
                                let peer = peer.clone();
                                egui::Frame::default()
                                    .fill(Color32::from_rgb(30, 34, 50))
                                    .corner_radius(8.0)
                                    .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 })
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("💻").size(20.0));
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(&peer.display).strong().color(Color32::from_rgb(200, 210, 240)));
                                                ui.label(RichText::new(peer.addr.to_string()).color(Color32::from_rgb(100, 120, 160)).size(11.0));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button(RichText::new("Send →").color(Color32::from_rgb(80, 180, 255))).clicked() {
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
                        ui.label(RichText::new(format!("Sending to {}…", peer_name))
                            .color(Color32::from_rgb(160, 200, 240)));
                        ui.add_space(8.0);
                        ui.add(egui::ProgressBar::new(progress).animate(true).show_percentage());
                        ctx.request_repaint_after(std::time::Duration::from_millis(50));
                    }

                    LanTransferState::Done(msg) => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new("✅").size(36.0));
                            ui.label(RichText::new(msg).strong().color(Color32::from_rgb(80, 220, 130)).size(15.0));
                        });
                    }

                    LanTransferState::Err(e) => {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("❌").size(36.0));
                            ui.label(RichText::new(e).color(Color32::from_rgb(220, 80, 80)));
                        });
                    }

                    LanTransferState::Idle => {}
                }

                ui.add_space(12.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if matches!(&self.lan_state, LanTransferState::Ready(_) | LanTransferState::Err(_)) {
                        if ui.button("🔄 Rescan").clicked() {
                            if let Some(path) = self.lan_file_path.clone() {
                                self.start_lan_discover(path);
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() { close = true; }
                    });
                });
            });

        if let Some(peer) = send_to { self.start_lan_send(peer); }
        if close { self.lan_state = LanTransferState::Idle; self.lan_file_path = None; }
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
            .open(&mut open).collapsible(false).resizable(false).min_width(320.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("props").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                    ui.label(RichText::new("Name:").strong()); ui.label(&name); ui.end_row();
                    ui.label(RichText::new("Type:").strong()); ui.label(if is_dir { "Folder" } else { "File" }); ui.end_row();
                    ui.label(RichText::new("Size:").strong()); ui.label(Self::format_size(size)); ui.end_row();
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
                .fixed_pos(egui::pos2(screen.max.x - 350.0, y - 46.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(22, 24, 40, alpha))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 9, bottom: 9 })
                        .show(ui, |ui| {
                            ui.set_max_width(310.0);
                            ui.label(RichText::new(&notif.message)
                                .color(Color32::from_rgba_unmultiplied(210, 220, 245, alpha)).size(13.0));
                        });
                });
            y -= 54.0;
        }
    }

    // ── AI settings panel ─────────────────────────────────────────────────────

    fn show_ai_settings_panel(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(Color32::from_rgb(20, 22, 36))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(50, 60, 100)))
            .corner_radius(10.0)
            .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 })
            .show(ui, |ui| {
                ui.label(RichText::new("AI Provider Settings").strong().size(13.0)
                    .color(Color32::from_rgb(180, 200, 240)));
                ui.add_space(8.0);

                ui.label(RichText::new("Provider").color(Color32::from_rgb(130, 150, 200)).size(11.0));
                ui.horizontal(|ui| {
                    let is_claude = self.ai_settings_draft.provider == AiProvider::Claude;
                    if ui.selectable_label(is_claude,
                        RichText::new("Claude").color(Color32::from_rgb(200, 130, 80))).clicked()
                    {
                        self.ai_settings_draft.provider = AiProvider::Claude;
                        if self.ai_settings_draft.model == "gemini-2.0-flash"
                            || self.ai_settings_draft.model.is_empty()
                        {
                            self.ai_settings_draft.model = "claude-haiku-4-5-20251001".into();
                        }
                    }
                    if ui.selectable_label(!is_claude,
                        RichText::new("Gemini").color(Color32::from_rgb(66, 153, 225))).clicked()
                    {
                        self.ai_settings_draft.provider = AiProvider::Gemini;
                        if self.ai_settings_draft.model == "claude-haiku-4-5-20251001"
                            || self.ai_settings_draft.model.is_empty()
                        {
                            self.ai_settings_draft.model = "gemini-2.0-flash".into();
                        }
                    }
                });
                ui.add_space(6.0);

                ui.label(RichText::new("API Key").color(Color32::from_rgb(130, 150, 200)).size(11.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.ai_settings_draft.api_key)
                        .password(true)
                        .hint_text("Paste your API key here…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);

                ui.label(RichText::new("Model").color(Color32::from_rgb(130, 150, 200)).size(11.0));
                let model_hint = self.ai_settings_draft.default_model();
                ui.add(
                    egui::TextEdit::singleline(&mut self.ai_settings_draft.model)
                        .hint_text(model_hint)
                        .desired_width(f32::INFINITY),
                );

                // Quick model presets
                ui.add_space(4.0);
                let presets: &[(&str, &str)] = match self.ai_settings_draft.provider {
                    AiProvider::Claude => &[
                        ("Haiku", "claude-haiku-4-5-20251001"),
                        ("Sonnet", "claude-sonnet-4-5"),
                        ("Opus", "claude-opus-4-5"),
                    ],
                    AiProvider::Gemini => &[
                        ("Flash 2.0", "gemini-2.0-flash"),
                        ("Pro 1.5", "gemini-1.5-pro"),
                        ("Flash 1.5", "gemini-1.5-flash"),
                    ],
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Presets:").size(11.0).color(Color32::from_rgb(100, 110, 150)));
                    for &(label, model) in presets {
                        if ui.small_button(label).clicked() {
                            self.ai_settings_draft.model = model.to_string();
                        }
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Save").color(Color32::from_rgb(80, 200, 130))).clicked() {
                        self.ai_config = self.ai_settings_draft.clone();
                        self.ai_config.save();
                        self.show_ai_settings = false;
                        self.push_notification(
                            format!("AI set to {} — {}", self.ai_config.provider_label(), self.ai_config.model),
                            self.ai_config.provider_color(),
                        );
                    }
                    if ui.button("Cancel").clicked() {
                        self.ai_settings_draft = self.ai_config.clone();
                        self.show_ai_settings = false;
                    }
                });
            });
    }

    // ── Chat sidebar ──────────────────────────────────────────────────────────

    fn show_chat_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header
            egui::Frame::default()
                .fill(Color32::from_rgb(24, 26, 40))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🤖").size(22.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("AI File Assistant").strong().size(13.0));
                            ui.horizontal(|ui| {
                                let provider_color = self.ai_config.provider_color();
                                ui.label(RichText::new("●").size(10.0).color(
                                    if self.ai_config.api_key.is_empty() {
                                        Color32::from_rgb(160, 100, 60)
                                    } else {
                                        Color32::from_rgb(80, 200, 100)
                                    }
                                ));
                                ui.label(RichText::new(self.ai_config.provider_label())
                                    .size(10.0).color(provider_color));
                                if !self.ai_config.model.is_empty() {
                                    ui.label(RichText::new(format!("· {}", self.ai_config.model))
                                        .size(10.0).color(Color32::from_rgb(90, 100, 140)));
                                }
                            });
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let gear_label = if self.show_ai_settings {
                                RichText::new("✕").size(14.0).color(Color32::from_rgb(180, 80, 80))
                            } else {
                                RichText::new("⚙").size(16.0).color(Color32::from_rgb(130, 150, 200))
                            };
                            if ui.button(gear_label).on_hover_text("Configure AI provider").clicked() {
                                self.show_ai_settings = !self.show_ai_settings;
                                self.ai_settings_draft = self.ai_config.clone();
                            }
                        });
                    });
                });

            // Settings panel (inline)
            if self.show_ai_settings {
                ui.add_space(4.0);
                self.show_ai_settings_panel(ui);
                ui.add_space(4.0);
                ui.separator();
            }

            // Messages area
            let available = ui.available_height();
            let input_h   = 72.0;
            let chips_h   = if !self.mentioned_files.is_empty() { 30.0 } else { 0.0 };
            let at_h      = if self.at_mode && !self.at_candidates().is_empty() { 130.0 } else { 0.0 };
            let msg_h     = (available - input_h - chips_h - at_h - 20.0).max(40.0);

            egui::ScrollArea::vertical()
                .id_salt("chat_scroll").max_height(msg_h)
                .stick_to_bottom(true).auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for msg in &self.chat_messages {
                        show_chat_bubble(ui, msg);
                        ui.add_space(4.0);
                    }
                    if self.ai_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Thinking…").color(Color32::from_rgb(130, 130, 170)).size(12.0).italics());
                        });
                    }
                });

            ui.separator();

            // @mention autocomplete
            if self.at_mode {
                let candidates = self.at_candidates();
                if !candidates.is_empty() {
                    egui::Frame::default()
                        .fill(Color32::from_rgb(26, 28, 44))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 70, 110)))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin { left: 4, right: 4, top: 4, bottom: 4 })
                        .show(ui, |ui| {
                            ui.label(RichText::new("  Tag a file:").color(Color32::from_rgb(100, 110, 160)).size(11.0));
                            let mut select: Option<usize> = None;
                            for &idx in &candidates {
                                if let Some(e) = self.entries.get(idx) {
                                    let lbl = format!("{} {}", Self::get_file_icon(e), e.name);
                                    if ui.selectable_label(false, RichText::new(lbl).size(12.0)).clicked() {
                                        select = Some(idx);
                                    }
                                }
                            }
                            if let Some(idx) = select { self.select_mention(idx); }
                        });
                }
            }

            // File chips
            if !self.mentioned_files.is_empty() {
                let mut remove: Option<usize> = None;
                ui.horizontal_wrapped(|ui| {
                    for (i, f) in self.mentioned_files.iter().enumerate() {
                        let chip = format!("{} {} ✕", if f.is_dir { "📁" } else { "📄" }, f.name);
                        if ui.small_button(RichText::new(chip).color(Color32::from_rgb(100, 160, 255)).size(11.0)).clicked() {
                            remove = Some(i);
                        }
                    }
                });
                if let Some(i) = remove { self.mentioned_files.remove(i); }
            }

            // Input row
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut self.chat_input)
                        .hint_text("Ask about files… @ to mention")
                        .desired_width(ui.available_width() - 44.0)
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
                    if ui.button(RichText::new("➤").size(18.0)).on_hover_text("Send (Enter)").clicked() {
                        self.send_ai_message();
                    }
                });
            });
            ui.label(RichText::new("Enter send  ·  Shift+Enter newline  ·  @ tag file")
                .color(Color32::from_rgb(80, 85, 120)).size(10.0));
        });
    }
}

// ─── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark theme
        ctx.set_visuals({
            let mut v = egui::Visuals::dark();
            v.window_fill      = Color32::from_rgb(22, 24, 36);
            v.panel_fill       = Color32::from_rgb(18, 20, 30);
            v.override_text_color = Some(Color32::from_rgb(200, 210, 240));
            v
        });

        // Poll AI response
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

        // Poll LAN
        self.poll_lan();
        if matches!(self.lan_state, LanTransferState::Discovering | LanTransferState::Sending { .. }) {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        // Expire notifications
        self.notifications.retain(|n| n.created.elapsed().as_secs() < 5);

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5)                          { self.load_directory(&self.current_path.clone()); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C)       { self.copy_file(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X)       { self.cut_file(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V)       { self.paste_file(); }
            if i.key_pressed(egui::Key::Delete)                      { self.delete_file(); }
            if i.key_pressed(egui::Key::Backspace)
                || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))  { self.go_back(); }
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

        self.show_top_panel(ctx);
        self.show_toolbar(ctx);
        self.show_bottom_panel(ctx);

        egui::SidePanel::right("chat_panel")
            .resizable(true).min_width(270.0).default_width(320.0).max_width(520.0)
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(20, 22, 34))
                .inner_margin(egui::Margin { left: 8, right: 8, top: 8, bottom: 8 }))
            .show(ctx, |ui| { self.show_chat_sidebar(ui); });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(18, 20, 30)).inner_margin(8.0))
            .show(ctx, |ui| { self.show_file_list(ui, ctx); });

        self.show_context_menu(ctx);
        self.show_lan_dialog(ctx);
        self.show_properties_dialog(ctx);
        self.show_notifications(ctx);
    }
}

// ─── LAN networking ───────────────────────────────────────────────────────────

/// Background thread: listens for UDP discovery pings and advertises ourselves;
/// also listens for incoming TCP file transfers.
fn lan_receive_server(tx: std::sync::mpsc::Sender<LanServerMsg>, save_dir: PathBuf) {
    // Start a parallel thread for UDP advertising
    thread::spawn(lan_advertise_loop);

    // TCP server for receiving files
    let listener = match TcpListener::bind(("0.0.0.0", LAN_TRANSFER_PORT)) {
        Ok(l) => l,
        Err(e) => { let _ = tx.send(LanServerMsg::Error(format!("Cannot listen on port {}: {}", LAN_TRANSFER_PORT, e))); return; }
    };
    let _ = listener.set_nonblocking(false);

    for stream in listener.incoming().flatten() {
        let tx2      = tx.clone();
        let save_dir = save_dir.clone();
        thread::spawn(move || {
            match lan_receive_file(stream, &save_dir) {
                Ok((name, dest)) => { let _ = tx2.send(LanServerMsg::FileReceived { name, dest }); }
                Err(e)           => { let _ = tx2.send(LanServerMsg::Error(e)); }
            }
        });
    }
}

/// Loops forever, responding to UDP discovery pings.
fn lan_advertise_loop() {
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", LAN_DISCOVER_PORT)) else { return };
    let hostname = hostname();
    let mut buf  = [0u8; 256];
    loop {
        if let Ok((_, from)) = sock.recv_from(&mut buf) {
            let msg = String::from_utf8_lossy(&buf);
            if msg.trim_matches('\0').starts_with("ANVEL_DISCOVER") {
                let _ = sock.send_to(format!("ANVEL_PEER:{}", hostname).as_bytes(), from);
            }
        }
    }
}

/// One-shot UDP advertisement (used when starting a discovery scan).
fn lan_advertise_once() {
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) else { return };
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let hostname = hostname();
    let mut buf  = [0u8; 256];
    // Wait for a discover ping and reply once
    if let Ok((_, from)) = sock.recv_from(&mut buf) {
        let _ = sock.send_to(format!("ANVEL_PEER:{}", hostname).as_bytes(), from);
    }
}

/// Send a file over TCP to the given IP address.
fn lan_send_file(path: &Path, addr: std::net::IpAddr) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let name_bytes = name.as_bytes();

    let mut stream = TcpStream::connect((addr, LAN_TRANSFER_PORT))
        .map_err(|e| format!("Cannot connect to peer: {}", e))?;

    // Protocol: [4-byte name length LE] [name bytes] [8-byte file size LE] [file data]
    let name_len = (name_bytes.len() as u32).to_le_bytes();
    let file_len = (data.len() as u64).to_le_bytes();

    stream.write_all(&name_len).map_err(|e| e.to_string())?;
    stream.write_all(name_bytes).map_err(|e| e.to_string())?;
    stream.write_all(&file_len).map_err(|e| e.to_string())?;
    stream.write_all(&data).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    Ok(())
}

/// Receive a file from a TCP stream, save it to save_dir.
fn lan_receive_file(mut stream: TcpStream, save_dir: &Path) -> Result<(String, PathBuf), String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let name_len = u32::from_le_bytes(len_buf) as usize;

    if name_len > 4096 { return Err("Malformed packet: name too long".into()); }
    let mut name_buf = vec![0u8; name_len];
    stream.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
    let name = String::from_utf8_lossy(&name_buf).to_string();

    let mut size_buf = [0u8; 8];
    stream.read_exact(&mut size_buf).map_err(|e| e.to_string())?;
    let file_size = u64::from_le_bytes(size_buf) as usize;

    if file_size > 2 * 1024 * 1024 * 1024 { return Err("File too large (>2 GB)".into()); }
    let mut data = vec![0u8; file_size];
    stream.read_exact(&mut data).map_err(|e| e.to_string())?;

    let dest = save_dir.join(&name);
    fs::write(&dest, &data).map_err(|e| e.to_string())?;

    Ok((name, dest))
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "this-device".into())
        .trim().to_string()
}

// ─── AI API calls ─────────────────────────────────────────────────────────────

fn call_claude(api_key: &str, model: &str, history: Vec<(String, String)>) -> Result<String, String> {
    let msgs: Vec<String> = history.iter().map(|(role, content)| {
        format!(r#"{{"role":"{}","content":"{}"}}"#, role, json_escape(content))
    }).collect();

    let body = format!(
        r#"{{"model":"{}","max_tokens":1024,"system":"You are an intelligent file assistant embedded in a desktop file explorer. Help users manage, understand, and work with their files. Be concise and practical. Plain text only — no markdown.","messages":[{}]}}"#,
        model, msgs.join(",")
    );

    let out = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            "https://api.anthropic.com/v1/messages",
            "-H", "content-type: application/json",
            "-H", "anthropic-version: 2023-06-01",
            "-H", &format!("x-api-key: {}", api_key),
            "-d", &body,
        ])
        .output().map_err(|e| format!("curl error: {}", e))?;

    parse_claude_response(&String::from_utf8_lossy(&out.stdout))
}

fn parse_claude_response(resp: &str) -> Result<String, String> {
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
    if let Some(s) = resp.find("\"message\":\"") {
        let rest = &resp[s + 11..];
        if let Some(end) = rest.find('"') {
            return Err(format!("API error: {}", &rest[..end]));
        }
    }
    Err(format!("Could not parse response. Check your Claude API key.\nRaw: {}", &resp[..resp.len().min(200)]))
}

fn call_gemini(api_key: &str, model: &str, history: Vec<(String, String)>) -> Result<String, String> {
    let contents: Vec<String> = history.iter().map(|(role, content)| {
        let grole = if role == "user" { "user" } else { "model" };
        format!(r#"{{"role":"{}","parts":[{{"text":"{}"}}]}}"#, grole, json_escape(content))
    }).collect();

    let system_instruction = r#""systemInstruction":{"parts":[{"text":"You are an intelligent file assistant embedded in a desktop file explorer. Help users manage, understand, and work with their files. Be concise and practical. Plain text only — no markdown."}]}"#;

    let body = format!(
        r#"{{{}, "contents":[{}]}}"#,
        system_instruction, contents.join(",")
    );

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let out = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            &url,
            "-H", "content-type: application/json",
            "-d", &body,
        ])
        .output().map_err(|e| format!("curl error: {}", e))?;

    parse_gemini_response(&String::from_utf8_lossy(&out.stdout))
}

fn parse_gemini_response(resp: &str) -> Result<String, String> {
    // Gemini returns: "candidates":[{"content":{"parts":[{"text":"..."}]
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
    if let Some(s) = resp.find("\"message\":\"") {
        let rest = &resp[s + 11..];
        if let Some(end) = rest.find('"') {
            return Err(format!("API error: {}", &rest[..end]));
        }
    }
    Err(format!("Could not parse Gemini response. Check your API key.\nRaw: {}", &resp[..resp.len().min(200)]))
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

// ─── Free helpers ─────────────────────────────────────────────────────────────

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty    = entry.file_type()?;
        if ty.is_dir() { copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?; }
        else           { fs::copy(entry.path(), dst.join(entry.file_name()))?; }
    }
    Ok(())
}

fn fnv_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| h.wrapping_mul(0x100000001b3).wrapping_add(b as u64))
}

fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(format!("{}  {}", icon, label))
            .color(Color32::from_rgb(200, 210, 235)).size(13.0))
            .frame(false).min_size(Vec2::new(200.0, 24.0)),
    ).clicked()
}

fn menu_item_danger(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(format!("{}  {}", icon, label))
            .color(Color32::from_rgb(230, 80, 80)).size(13.0))
            .frame(false).min_size(Vec2::new(200.0, 24.0)),
    ).clicked()
}

fn show_chat_bubble(ui: &mut egui::Ui, msg: &ChatMessage) {
    match msg.role {
        ChatRole::Assistant => {
            ui.horizontal_top(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("🤖").size(14.0));
                egui::Frame::default()
                    .fill(Color32::from_rgb(30, 34, 52))
                    .corner_radius(egui::CornerRadius { nw: 2, ne: 10, sw: 10, se: 10 })
                    .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
                    .show(ui, |ui| {
                        ui.set_max_width(230.0);
                        ui.label(RichText::new(&msg.content).color(Color32::from_rgb(200, 215, 245)).size(12.5));
                    });
            });
        }
        ChatRole::User => {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add_space(4.0);
                egui::Frame::default()
                    .fill(Color32::from_rgb(28, 56, 120))
                    .corner_radius(egui::CornerRadius { nw: 10, ne: 2, sw: 10, se: 10 })
                    .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
                    .show(ui, |ui| {
                        ui.set_max_width(230.0);
                        ui.label(RichText::new(&msg.content).color(Color32::from_rgb(220, 235, 255)).size(12.5));
                    });
            });
        }
    }
}
