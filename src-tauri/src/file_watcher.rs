// 文件系统监控模块 - 专业级实现
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// 文件系统事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEventType {
    Created,
    Modified,
    Deleted,
    Renamed(String), // 包含新路径
}

/// 文件系统事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEvent {
    pub event_type: FileSystemEventType,
    pub path: String,
    pub timestamp: String,
    pub size: Option<u64>,
}

/// 文件监控器状态
pub struct FileWatcherState {
    pub watcher: Option<RecommendedWatcher>,
    pub watched_paths: Vec<String>,
    pub event_sender: Option<Sender<FileSystemEvent>>,
    pub is_running: bool,
}

impl Default for FileWatcherState {
    fn default() -> Self {
        Self {
            watcher: None,
            watched_paths: Vec::new(),
            event_sender: None,
            is_running: false,
        }
    }
}

// 全局监控器状态
lazy_static::lazy_static! {
    static ref WATCHER_STATE: Arc<Mutex<FileWatcherState>> = Arc::new(Mutex::new(FileWatcherState::default()));
}

/// 启动文件系统监控
pub fn start_file_watcher(
    paths: Vec<String>,
    callback: Box<dyn Fn(FileSystemEvent) + Send + 'static>,
) -> Result<(), String> {
    let (tx, rx): (Sender<FileSystemEvent>, Receiver<FileSystemEvent>) = channel();
    
    // 创建监控器
    let watcher_tx = tx.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    handle_notify_event(event, &watcher_tx);
                }
                Err(e) => {
                    eprintln!("文件监控错误: {:?}", e);
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )
    .map_err(|e| format!("创建文件监控器失败: {}", e))?;
    
    // 添加监控路径
    for path in &paths {
        let path_obj = Path::new(path);
        if path_obj.exists() {
            watcher
                .watch(path_obj, RecursiveMode::Recursive)
                .map_err(|e| format!("添加监控路径失败 {}: {}", path, e))?;
        }
    }
    
    // 更新状态
    {
        let mut state = WATCHER_STATE.lock().unwrap();
        state.watcher = Some(watcher);
        state.watched_paths = paths;
        state.event_sender = Some(tx);
        state.is_running = true;
    }
    
    // 启动事件处理线程
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            callback(event);
        }
    });
    
    Ok(())
}

/// 处理 notify 事件
fn handle_notify_event(event: Event, sender: &Sender<FileSystemEvent>) {
    use chrono::Utc;
    
    let timestamp = Utc::now().to_rfc3339();
    
    for path in &event.paths {
        let path_str = path.to_string_lossy().to_string();
        
        // 获取文件大小（如果存在）
        let size = if path.exists() && path.is_file() {
            std::fs::metadata(path).ok().map(|m| m.len())
        } else {
            None
        };
        
        let fs_event = match event.kind {
            notify::EventKind::Create(_) => FileSystemEvent {
                event_type: FileSystemEventType::Created,
                path: path_str,
                timestamp: timestamp.clone(),
                size,
            },
            notify::EventKind::Modify(_) => FileSystemEvent {
                event_type: FileSystemEventType::Modified,
                path: path_str,
                timestamp: timestamp.clone(),
                size,
            },
            notify::EventKind::Remove(_) => FileSystemEvent {
                event_type: FileSystemEventType::Deleted,
                path: path_str,
                timestamp: timestamp.clone(),
                size: None,
            },
            _ => continue,
        };
        
        if let Err(e) = sender.send(fs_event) {
            eprintln!("发送文件系统事件失败: {:?}", e);
            break;
        }
    }
}

/// 停止文件系统监控
pub fn stop_file_watcher() -> Result<(), String> {
    let mut state = WATCHER_STATE.lock().unwrap();
    
    if let Some(mut watcher) = state.watcher.take() {
        // 取消所有监控路径
        for path in &state.watched_paths {
            let _ = watcher.unwatch(Path::new(path));
        }
    }
    
    state.watched_paths.clear();
    state.event_sender = None;
    state.is_running = false;
    
    Ok(())
}

/// 添加监控路径
pub fn add_watch_path(path: String) -> Result<(), String> {
    let mut state = WATCHER_STATE.lock().unwrap();
    
    if let Some(ref mut watcher) = state.watcher {
        let path_obj = Path::new(&path);
        if path_obj.exists() {
            watcher
                .watch(path_obj, RecursiveMode::Recursive)
                .map_err(|e| format!("添加监控路径失败: {}", e))?;
            state.watched_paths.push(path);
        }
    }
    
    Ok(())
}

/// 移除监控路径
pub fn remove_watch_path(path: &str) -> Result<(), String> {
    let mut state = WATCHER_STATE.lock().unwrap();
    
    if let Some(ref mut watcher) = state.watcher {
        watcher
            .unwatch(Path::new(path))
            .map_err(|e| format!("移除监控路径失败: {}", e))?;
        state.watched_paths.retain(|p| p != path);
    }
    
    Ok(())
}

/// 获取监控状态
pub fn get_watcher_status() -> (bool, Vec<String>) {
    let state = WATCHER_STATE.lock().unwrap();
    (state.is_running, state.watched_paths.clone())
}
