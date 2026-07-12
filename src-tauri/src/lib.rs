// SpaceTree 空间树 - 企业级完整实现 v3.40.0

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Condvar};
use chrono::prelude::*;
use tauri::Emitter;

// 文件系统监控模块
mod file_watcher;
use file_watcher::FileSystemEvent;

// 企业级扫描器模块
mod scanner;
pub use scanner::{EnterpriseScanner, EnterpriseScanConfig, StreamingScanner, ScanStatistics};

// 日志
use log::{info, warn};

// 随机数 - 使用 rand::random 函数

// ========== 全局状态 ==========

lazy_static::lazy_static! {
    static ref SCAN_CONTROL: Arc<(Mutex<ScanControlState>, Condvar)> = Arc::new((
        Mutex::new(ScanControlState::Running),
        Condvar::new(),
    ));
    
    static ref SCAN_STATE: Mutex<ScanState> = Mutex::new(ScanState::default());
}

// ========== 数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub hash: Option<String>,
    pub hash_algorithm: String,
    pub created_at: String,
    pub modified_at: String,
    pub file_extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub files: Vec<FileInfo>,
    pub total_size: u64,
    pub wasted_space: u64,
}

// 重复文件分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateClassification {
    pub complete_duplicates: Vec<DuplicateGroup>,  // 完全相同（文件名与哈希都相同）
    pub name_duplicates: Vec<DuplicateGroup>,      // 名称相同（文件名相同，哈希不同）
    pub content_duplicates: Vec<DuplicateGroup>,   // 内容相同（哈希相同，文件名不同）
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanState {
    pub should_stop: bool,
    pub is_paused: bool,
    pub total_files: usize,
    pub processed_files: usize,
    pub current_file: String,
    pub scanned_directories: usize,
    pub current_directory: String,
    pub recent_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub current_file: String,
    pub percentage: f64,
    pub is_paused: bool,
    pub is_stopped: bool,
    pub scanned_directories: usize,
    pub current_directory: String,
    pub recent_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanControlState {
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    XxHash3,      // 默认 - 极速非加密哈希
    XxHash64,     // 64位版本，更好的兼容性
    Blake3,       // 现代安全哈希 - 速度与安全的最佳平衡
    Sha256,       // 标准安全哈希
    Sha512,       // 高安全级别哈希
    Md5,          // 遗留兼容（已弃用，不推荐）
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::XxHash3
    }
}

impl Serialize for HashAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            HashAlgorithm::XxHash3 => "xxhash3",
            HashAlgorithm::XxHash64 => "xxhash64",
            HashAlgorithm::Blake3 => "blake3",
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha512 => "sha512",
            HashAlgorithm::Md5 => "md5",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for HashAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "xxhash3" => Ok(HashAlgorithm::XxHash3),
            "xxhash64" => Ok(HashAlgorithm::XxHash64),
            "blake3" => Ok(HashAlgorithm::Blake3),
            "sha256" => Ok(HashAlgorithm::Sha256),
            "sha512" => Ok(HashAlgorithm::Sha512),
            "md5" => Ok(HashAlgorithm::Md5),
            _ => Err(serde::de::Error::custom(format!("unknown hash algorithm: {}", s))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default)]
    pub max_depth: i32,
    #[serde(default)]
    pub include_extensions: Vec<String>,
    #[serde(default)]
    pub exclude_extensions: Vec<String>,
    pub exclude_hidden: bool,
    pub exclude_system: bool,
    #[serde(rename = "min_size")]
    pub min_file_size: u64,
    #[serde(rename = "max_size")]
    pub max_file_size: u64,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    pub hash_algorithm: HashAlgorithm,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            include_extensions: vec![],
            exclude_extensions: vec![],
            exclude_hidden: true,
            exclude_system: true,
            min_file_size: 0,
            max_file_size: 10 * 1024 * 1024 * 1024,
            exclude_patterns: vec![],
            hash_algorithm: HashAlgorithm::XxHash3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepStrategy {
    pub strategy_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResumeResult {
    pub success: bool,
    pub is_paused: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailResult {
    pub path: String,
    pub thumbnail_base64: Option<String>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarImageGroup {
    pub similarity: f64,
    pub files: Vec<FileInfo>,
    pub hash_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSimilarityConfig {
    pub algorithm: String,
    pub threshold: u32,
}

impl Default for ImageSimilarityConfig {
    fn default() -> Self {
        Self {
            algorithm: "phash".to_string(),
            threshold: 10,
        }
    }
}

// ========== 扫描历史数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryItem {
    pub id: i64,
    pub scan_time: String,
    pub directories: Vec<String>,
    pub total_files: i64,
    pub duplicate_groups: i64,
    pub duplicate_files: i64,
    pub wasted_space: i64,
    pub duration_seconds: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryResult {
    pub history: Vec<ScanHistoryItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryQueryParams {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
    pub min_files: Option<i64>,
    pub max_files: Option<i64>,
    pub search_keyword: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl Default for ScanHistoryQueryParams {
    fn default() -> Self {
        Self {
            start_date: None,
            end_date: None,
            status: None,
            min_files: None,
            max_files: None,
            search_keyword: None,
            page: 1,
            page_size: 20,
            sort_by: Some("scan_time".to_string()),
            sort_order: Some("DESC".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryStatistics {
    pub total_scans: i64,
    pub completed_scans: i64,
    pub interrupted_scans: i64,
    pub total_files_scanned: i64,
    pub total_duplicates_found: i64,
    pub total_wasted_space: i64,
    pub average_scan_duration: i64,
    pub scan_frequency_by_day: Vec<DayScanCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayScanCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherStatus {
    pub is_running: bool,
    pub watched_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_scan: bool,
    pub scan_interval: i32,
    pub exclude_patterns: Vec<String>,
    pub min_file_size: i64,
    pub hash_algorithm: String,
    pub enable_file_watcher: bool,
    pub default_scan_paths: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            language: "zh-CN".to_string(),
            auto_scan: false,
            scan_interval: 24,
            exclude_patterns: vec![],
            min_file_size: 0,
            hash_algorithm: "xxhash3".to_string(),
            enable_file_watcher: true,
            default_scan_paths: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSettingsRequest {
    pub settings: AppSettings,
    pub db_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub file_extensions: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: usize,
    pub page_size: usize,
    pub sort_by: String,
    pub sort_order: String,
    pub get_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub files: Vec<FileInfo>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

// ========== 分析功能数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateDistribution {
    pub by_size: Vec<SizeRangeGroup>,
    pub by_type: Vec<FileTypeGroup>,
    pub by_directory: Vec<DirectoryGroup>,
    pub summary: DistributionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRangeGroup {
    pub range: String,
    pub min_size: u64,
    pub max_size: u64,
    pub file_count: i32,
    pub total_size: u64,
    pub group_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeGroup {
    pub extension: String,
    pub file_count: i32,
    pub group_count: i32,
    pub total_size: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub directory: String,
    pub file_count: i32,
    pub group_count: i32,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionSummary {
    pub total_files: i64,
    pub total_groups: i64,
    pub total_wasted_space: u64,
    pub average_group_size: f64,
}

// ========== 合规检查数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub report_type: String,
    pub generated_at: String,
    pub summary: ComplianceSummary,
    pub findings: Vec<ComplianceFinding>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_files_reviewed: usize,
    pub total_duplicates_found: usize,
    pub total_space_occupied: u64,
    pub risk_level: String,
    pub compliance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub finding_id: String,
    pub category: String,
    pub severity: String,
    pub description: String,
    pub affected_files: Vec<String>,
    pub remediation: String,
}

// ========== 日志审计数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub id: i64,
    pub timestamp: String,
    pub operation_type: String,
    pub target_files: Vec<String>,
    pub source_path: Option<String>,
    pub destination_path: Option<String>,
    pub status: String,
    pub message: Option<String>,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryParams {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub operation_type: Option<String>,
    pub status: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryResult {
    pub logs: Vec<OperationLog>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub severity: String,
    pub user: String,
    pub resource: String,
    pub action: String,
    pub details: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryParams {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    pub events: Vec<AuditLog>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

// ========== 回收站数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleBinItem {
    pub id: String,
    pub original_path: String,
    pub deleted_at: String,
    pub size: u64,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleBinList {
    pub items: Vec<RecycleBinItem>,
    pub total_count: usize,
    pub total_size: u64,
}

// ========== 空文件扫描数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyFileScanResult {
    pub empty_files: Vec<FileInfo>,
    pub total_count: usize,
    pub total_size: u64,
}

// ========== 重复文件夹数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFolderResult {
    pub duplicate_folders: Vec<DuplicateFolderGroup>,
    pub total_groups: usize,
    pub total_wasted_space: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFolderGroup {
    pub folder_paths: Vec<String>,
    pub file_count: usize,
    pub total_size: u64,
    pub similarity: f64,
}

// ========== 安全工具函数 ==========

fn is_path_safe(path: &str, allowed_roots: &[String]) -> bool {
    // 首先清理路径
    let sanitized = sanitize_path(path);
    let path_buf = PathBuf::from(&sanitized);
    
    let canonical = match path_buf.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    for root in allowed_roots {
        let root_path = PathBuf::from(root);
        if let Ok(root_canonical) = root_path.canonicalize() {
            if canonical.starts_with(&root_canonical) {
                return true;
            }
        }
    }
    
    if allowed_roots.is_empty() {
        let path_lower = sanitized.to_lowercase();
        let dangerous_paths = [
            "c:\\windows",
            "c:\\program files",
            "c:\\programdata",
            "/etc", "/usr", "/bin", "/sbin", "/lib", "/sys", "/proc",
        ];
        
        for dangerous in &dangerous_paths {
            if path_lower.starts_with(dangerous) {
                return false;
            }
        }
        return true;
    }
    
    false
}

fn sanitize_path(path: &str) -> String {
    path.replace("..", "")
        .replace("//", "/")
        .replace("\\\\", "\\")
        .trim_start_matches('/')
        .trim_start_matches('\\')
        .to_string()
}

#[cfg(windows)]
fn is_file_locked(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    use std::fs::OpenOptions;
    
    let result = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path);
    
    match result {
        Ok(_) => false,
        Err(_) => true,
    }
}

#[cfg(unix)]
fn is_file_locked(_path: &Path) -> bool {
    false
}

// ========== 日志记录函数 ==========

fn log_operation(
    conn: &Connection,
    operation_type: &str,
    target_files: &[String],
    source_path: Option<&str>,
    destination_path: Option<&str>,
    status: &str,
    message: Option<&str>,
) -> Result<(), String> {
    let target_files_str = target_files.join(",");
    conn.execute(
        "INSERT INTO operation_logs (operation_type, target_files, source_path, destination_path, status, message, user) 
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        (
            operation_type,
            target_files_str,
            source_path,
            destination_path,
            status,
            message,
            "system",
        ),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn log_audit_event(
    conn: &Connection,
    event_type: &str,
    severity: &str,
    resource: &str,
    action: &str,
    details: &str,
    result: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO audit_logs (event_type, severity, user, resource, action, details, result) 
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        (event_type, severity, "system", resource, action, details, result),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ========== 哈希算法 ==========

fn compute_file_hash(path: &Path, algorithm: HashAlgorithm) -> Result<String, String> {
    match algorithm {
        HashAlgorithm::XxHash3 => compute_xxhash3(path),
        HashAlgorithm::XxHash64 => compute_xxhash64(path),
        HashAlgorithm::Blake3 => compute_blake3(path),
        HashAlgorithm::Sha256 => compute_sha256(path),
        HashAlgorithm::Sha512 => compute_sha512(path),
        HashAlgorithm::Md5 => compute_md5(path),
    }
}

fn compute_xxhash3(path: &Path) -> Result<String, String> {
    use xxhash_rust::xxh3::Xxh3;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Xxh3::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:016x}", hasher.digest()))
}

fn compute_xxhash64(path: &Path) -> Result<String, String> {
    use xxhash_rust::xxh64::Xxh64;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Xxh64::new(0); // seed = 0
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:016x}", hasher.digest()))
}

fn compute_blake3(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn compute_md5(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = md5::Context::new();
    let mut buffer = vec![0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.consume(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.compute()))
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    use sha2::{Sha256, Digest};

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn compute_sha512(path: &Path) -> Result<String, String> {
    use sha2::{Sha512, Digest};

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha512::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

// ========== 图像处理 ==========

fn compute_image_perceptual_hash(path: &Path, algorithm: &str) -> Result<String, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    
    match algorithm {
        "ahash" => compute_ahash(&img),
        "dhash" => compute_dhash(&img),
        "phash" => compute_phash(&img),
        _ => compute_phash(&img),
    }
}

fn compute_ahash(img: &image::DynamicImage) -> Result<String, String> {
    use image::imageops::FilterType;
    
    let resized = img.resize_exact(8, 8, FilterType::Lanczos3);
    let gray = resized.to_luma8();
    
    let pixels: Vec<u8> = gray.pixels().map(|p| p[0]).collect();
    let avg: u8 = (pixels.iter().map(|&p| p as u32).sum::<u32>() / pixels.len() as u32) as u8;
    
    let mut hash = 0u64;
    for (i, &pixel) in pixels.iter().enumerate() {
        if pixel >= avg {
            hash |= 1 << i;
        }
    }
    
    Ok(format!("{:016x}", hash))
}

fn compute_dhash(img: &image::DynamicImage) -> Result<String, String> {
    use image::imageops::FilterType;
    
    let resized = img.resize_exact(9, 8, FilterType::Lanczos3);
    let gray = resized.to_luma8();
    
    let mut hash = 0u64;
    let mut bit_pos = 0;
    
    for y in 0..8 {
        for x in 0..8 {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1 << bit_pos;
            }
            bit_pos += 1;
        }
    }
    
    Ok(format!("{:016x}", hash))
}

lazy_static::lazy_static! {
    static ref DCT_MATRIX: [[f64; 32]; 8] = {
        let mut matrix = [[0.0; 32]; 8];
        for u in 0..8 {
            let c = if u == 0 {
                (1.0 / 2.0f64.sqrt()) * (2.0 / 32.0f64).sqrt()
            } else {
                1.0 * (2.0 / 32.0f64).sqrt()
            };
            for x in 0..32 {
                matrix[u][x] = c * (((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI) / 64.0).cos();
            }
        }
        matrix
    };
}

fn compute_phash(img: &image::DynamicImage) -> Result<String, String> {
    use image::imageops::FilterType;
    
    let resized = img.resize_exact(32, 32, FilterType::Lanczos3);
    let gray = resized.to_luma8();
    
    let mut pixels = [0.0; 1024];
    for (i, p) in gray.pixels().enumerate() {
        if i < 1024 {
            pixels[i] = p[0] as f64;
        }
    }
    
    // 第一阶段变换：Y = T * X 
    // T 大小为 8x32, X 大小为 32x32 (按行存储，所以 pixels[x * 32 + y] 为第 x 行、第 y 列的元素)
    // Y[u, y] = sum_x ( T[u, x] * X[x, y] )
    let mut y_matrix = [0.0; 256]; // 8x32
    for u in 0..8 {
        for y in 0..32 {
            let mut sum = 0.0;
            for x in 0..32 {
                sum += DCT_MATRIX[u][x] * pixels[x * 32 + y];
            }
            y_matrix[u * 32 + y] = sum;
        }
    }
    
    // 第二阶段变换：D = Y * T^T
    // D[u, v] = sum_y ( Y[u, y] * T[v, y] )
    let mut dct_coeffs = [0.0; 64]; // 8x8
    for u in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0;
            for y in 0..32 {
                sum += y_matrix[u * 32 + y] * DCT_MATRIX[v][y];
            }
            dct_coeffs[u * 8 + v] = sum;
        }
    }
    
    let avg = dct_coeffs.iter().skip(1).sum::<f64>() / 63.0;
    let mut hash = 0u64;
    for (i, &coeff) in dct_coeffs.iter().enumerate().skip(1) {
        if coeff > avg {
            hash |= 1 << (i - 1);
        }
    }
    
    Ok(format!("{:016x}", hash))
}

#[allow(dead_code)]
fn compute_hash_distance(hash1: &str, hash2: &str) -> u32 {
    if hash1.len() != hash2.len() {
        return u32::MAX;
    }
    
    let h1 = u64::from_str_radix(hash1, 16).unwrap_or(0);
    let h2 = u64::from_str_radix(hash2, 16).unwrap_or(0);
    
    let xor = h1 ^ h2;
    xor.count_ones()
}

fn generate_thumbnail(path: &Path, max_width: u32, max_height: u32) -> Result<(String, u32, u32), String> {
    use image::imageops::FilterType;
    use image::GenericImageView;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    
    // 验证尺寸限制
    const MAX_THUMBNAIL_SIZE: u32 = 4096;
    const MIN_THUMBNAIL_SIZE: u32 = 1;
    
    if max_width == 0 || max_height == 0 {
        return Err("缩略图尺寸不能为0".to_string());
    }
    
    if max_width > MAX_THUMBNAIL_SIZE || max_height > MAX_THUMBNAIL_SIZE {
        return Err(format!("缩略图尺寸不能超过 {}x{}", MAX_THUMBNAIL_SIZE, MAX_THUMBNAIL_SIZE));
    }
    
    let img = image::open(path).map_err(|e| e.to_string())?;
    
    let (orig_width, orig_height) = img.dimensions();
    
    // 如果原图尺寸小于最小缩略图尺寸，直接返回原图
    if orig_width < MIN_THUMBNAIL_SIZE || orig_height < MIN_THUMBNAIL_SIZE {
        return Err("原图尺寸太小".to_string());
    }
    
    let scale_x = max_width as f32 / orig_width as f32;
    let scale_y = max_height as f32 / orig_height as f32;
    let scale = scale_x.min(scale_y).min(1.0);
    
    let new_width = (orig_width as f32 * scale).max(MIN_THUMBNAIL_SIZE as f32) as u32;
    let new_height = (orig_height as f32 * scale).max(MIN_THUMBNAIL_SIZE as f32) as u32;
    
    let thumbnail = img.resize(new_width, new_height, FilterType::Lanczos3);
    
    let mut buffer = Vec::new();
    thumbnail.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    
    // 检查生成的缩略图大小（防止内存攻击）
    const MAX_BASE64_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if buffer.len() > MAX_BASE64_SIZE {
        return Err("生成的缩略图太大".to_string());
    }
    
    let base64_string = BASE64.encode(&buffer);
    
    Ok((base64_string, new_width, new_height))
}

// ========== 扫描控制 ==========

fn check_pause_with_condvar() -> bool {
    let (lock, cvar) = &**SCAN_CONTROL;
    let mut state = match lock.lock() {
        Ok(guard) => guard,
        Err(_) => return false, // 如果锁被污染，继续扫描
    };
    
    loop {
        match *state {
            ScanControlState::Stopped => return true,
            ScanControlState::Running => return false,
            ScanControlState::Paused => {
                state = match cvar.wait(state) {
                    Ok(guard) => guard,
                    Err(_) => return false, // 等待失败，继续扫描
                };
            }
        }
    }
}

fn set_scan_control(control: ScanControlState) {
    let (lock, cvar) = &**SCAN_CONTROL;
    if let Ok(mut state) = lock.lock() {
        *state = control;
        cvar.notify_all();
    }
}

fn reset_scan_state() {
    if let Ok(mut scan_state) = SCAN_STATE.lock() {
        *scan_state = ScanState::default();
    }
    set_scan_control(ScanControlState::Running);
}

// ========== 数据库 ==========

fn init_database(db_path: &str) -> Result<Connection, String> {
    // 确保数据库所在目录存在
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建数据库目录失败: {}", e))?;
        }
    }
    
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // 启用 WAL 模式和优化设置
    conn.execute("PRAGMA journal_mode = WAL", []).ok();
    conn.execute("PRAGMA synchronous = NORMAL", []).ok();
    
    // 自适应表结构升级（Volume GUID 卷感知感知支持）
    conn.execute("ALTER TABLE files ADD COLUMN volume_guid TEXT", []).ok();
    conn.execute("ALTER TABLE files ADD COLUMN relative_path TEXT", []).ok();
    
    // 主文件表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE NOT NULL,
            filename TEXT NOT NULL,
            size INTEGER NOT NULL,
            hash TEXT,
            hash_algorithm TEXT DEFAULT 'xxhash3',
            created_at TEXT,
            modified_at TEXT,
            file_extension TEXT,
            scanned_at TEXT DEFAULT CURRENT_TIMESTAMP,
            volume_guid TEXT,
            relative_path TEXT
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 应用设置表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 回收站表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recycle_bin (
            id TEXT PRIMARY KEY,
            original_path TEXT NOT NULL,
            deleted_at TEXT NOT NULL,
            size INTEGER NOT NULL,
            file_name TEXT NOT NULL
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 操作日志表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT DEFAULT CURRENT_TIMESTAMP,
            operation_type TEXT NOT NULL,
            target_files TEXT NOT NULL,
            source_path TEXT,
            destination_path TEXT,
            status TEXT NOT NULL,
            message TEXT,
            user TEXT DEFAULT 'system'
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 审计日志表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT DEFAULT CURRENT_TIMESTAMP,
            event_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            user TEXT NOT NULL,
            resource TEXT NOT NULL,
            action TEXT NOT NULL,
            details TEXT NOT NULL,
            result TEXT NOT NULL
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 图像哈希缓存表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS image_hashes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE NOT NULL,
            ahash TEXT,
            dhash TEXT,
            phash TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 缩略图缓存表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS thumbnails (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE NOT NULL,
            thumbnail_base64 TEXT NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 扫描历史表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scan_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_time TEXT DEFAULT CURRENT_TIMESTAMP,
            directories TEXT NOT NULL,
            total_files INTEGER NOT NULL DEFAULT 0,
            duplicate_groups INTEGER NOT NULL DEFAULT 0,
            duplicate_files INTEGER NOT NULL DEFAULT 0,
            wasted_space INTEGER NOT NULL DEFAULT 0,
            duration_seconds INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'completed'
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    // 创建索引
    conn.execute("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_files_size ON files(size)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_files_extension ON files(file_extension)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_operation_logs_timestamp ON operation_logs(timestamp)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_operation_logs_type ON operation_logs(operation_type)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_image_hashes_path ON image_hashes(path)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_thumbnails_path ON thumbnails(path)", [])
        .map_err(|e| e.to_string())?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_scan_history_time ON scan_history(scan_time)", [])
        .map_err(|e| e.to_string())?;
    
    info!("数据库初始化完成: {}", db_path);
    Ok(conn)
}

#[tauri::command(rename_all = "snake_case")]
fn init_database_cmd(db_path: String) -> Result<(), String> {
    info!("初始化数据库命令: {}", db_path);
    
    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }
    
    // 确保数据库所在目录存在
    if let Some(parent) = Path::new(&db_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建数据库目录失败: {}", e))?;
        }
    }
    
    // 初始化数据库（创建表结构）
    init_database(&db_path)?;
    
    info!("数据库初始化命令完成: {}", db_path);
    Ok(())
}

// ========== Tauri 命令 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalScanConfig {
    pub directories: Vec<String>,
    pub scan_config: ScanConfig,
    pub use_file_watcher: bool,
    pub detect_moves: bool,
    pub detect_renames: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalScanResult {
    pub new_files: Vec<FileInfo>,
    pub modified_files: Vec<FileInfo>,
    pub deleted_files: Vec<String>,
    pub moved_files: Vec<(String, String)>, // (old_path, new_path)
    pub renamed_files: Vec<(String, String)>, // (old_path, new_path)
    pub scan_time_ms: u64,
}

#[tauri::command(rename_all = "snake_case")]
async fn incremental_scan(db_path: String, config: IncrementalScanConfig) -> Result<IncrementalScanResult, String> {
    use std::time::Instant;

    info!("开始增量扫描: {:?}", config.directories);
    let start_time = Instant::now();

    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    if config.directories.is_empty() {
        return Err("未提供扫描目录".to_string());
    }

    // 重置扫描状态
    reset_scan_state();
    
    // 使用 spawn_blocking 在后台线程执行耗时操作，避免阻塞 UI
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut conn = init_database(&db_path)?;
        
        // 获取数据库中已有的文件记录
        let existing_files: HashMap<String, (String, u64)> = {
            let mut stmt = conn.prepare(
                "SELECT path, hash, size FROM files WHERE path IS NOT NULL"
            ).map_err(|e| e.to_string())?;
            
            let files: Vec<(String, String, u64)> = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u64,
                ))
            }).map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
            
            files.into_iter().map(|(p, h, s)| (p, (h, s))).collect()
        };
        
        // 计算总文件数
        let total_files: usize = config.directories.iter()
            .filter(|dir| Path::new(dir).exists())
            .flat_map(|dir| walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()))
            .filter(|e| e.path().is_file())
            .count();
        
        {
            if let Ok(mut state) = SCAN_STATE.lock() {
                state.total_files = total_files;
            }
        }
        
        let mut new_files: Vec<FileInfo> = Vec::new();
        let mut modified_files: Vec<FileInfo> = Vec::new();
        let mut current_files: HashSet<String> = HashSet::new();
        let mut processed = 0;
        
        // 扫描目录
        for dir in &config.directories {
            if !Path::new(dir).exists() {
                warn!("目录不存在，跳过: {}", dir);
                continue;
            }
            
            {
                if let Ok(mut state) = SCAN_STATE.lock() {
                    state.current_directory = dir.clone();
                    state.scanned_directories += 1;
                }
            }
            
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if check_pause_with_condvar() {
                    return Err("扫描已取消".to_string());
                }
                
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                
                let path_str = path.to_string_lossy().to_string();
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                current_files.insert(path_str.clone());
                processed += 1;
                
                // 更新扫描状态
                {
                    if let Ok(mut state) = SCAN_STATE.lock() {
                        state.current_file = file_name.clone();
                        state.processed_files = processed;
                        // 保持最近扫描的20个文件
                        state.recent_files.insert(0, file_name);
                        if state.recent_files.len() > 20 {
                            state.recent_files.truncate(20);
                        }
                    }
                }
                
                // 检查文件是否需要处理
                if let Some((old_hash, old_size)) = existing_files.get(&path_str) {
                    // 文件已存在，检查是否修改
                    let metadata = match fs::metadata(path) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    
                    let new_size = metadata.len();
                    if new_size != *old_size {
                        // 文件大小改变，需要重新计算哈希
                        match compute_file_hash(path, config.scan_config.hash_algorithm) {
                            Ok(new_hash) => {
                                if new_hash != *old_hash {
                                    // 文件已修改
                                    let file_info = FileInfo {
                                        id: 0,
                                        path: path_str.clone(),
                                        filename: path.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                        size: new_size,
                                        hash: Some(new_hash.clone()),
                                        hash_algorithm: format!("{:?}", config.scan_config.hash_algorithm).to_lowercase(),
                                        created_at: metadata.created().ok()
                                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                            .map(|d| d.as_secs().to_string())
                                            .unwrap_or_default(),
                                        modified_at: metadata.modified().ok()
                                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                            .map(|d| d.as_secs().to_string())
                                            .unwrap_or_default(),
                                        file_extension: path.extension()
                                            .map(|e| e.to_string_lossy().to_string().to_lowercase())
                                            .unwrap_or_default(),
                                    };
                                    
                                    // 更新数据库
                                    let _ = conn.execute(
                                        "UPDATE files SET hash = ?, size = ?, modified_at = ? WHERE path = ?",
                                        rusqlite::params![new_hash, new_size as i64, 
                                            file_info.modified_at.clone(), 
                                            &path_str],
                                    );
                                    
                                    modified_files.push(file_info);
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                } else {
                    // 新文件
                    match process_single_file(path, &config.scan_config, &mut conn) {
                        Ok(Some(file_info)) => {
                            new_files.push(file_info);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // 检测已删除的文件
        let deleted_files: Vec<String> = existing_files.keys()
            .filter(|p| !current_files.contains(*p))
            .cloned()
            .collect();
        
        // 从数据库中删除不存在的文件记录
        for path in &deleted_files {
            let _ = conn.execute("DELETE FROM files WHERE path = ?", [path]);
        }
        
        let scan_time_ms = start_time.elapsed().as_millis() as u64;
        
        info!(
            "增量扫描完成: {} 新文件, {} 修改文件, {} 删除文件, 耗时 {}ms",
            new_files.len(),
            modified_files.len(),
            deleted_files.len(),
            scan_time_ms
        );
        
        Ok(IncrementalScanResult {
            new_files,
            modified_files,
            deleted_files,
            moved_files: Vec::new(),
            renamed_files: Vec::new(),
            scan_time_ms,
        })
    }).await.map_err(|e| format!("增量扫描任务失败: {}", e))?;
    
    result
}

// 处理单个文件（用于增量扫描）
fn process_single_file(
    path: &Path,
    config: &ScanConfig,
    conn: &mut Connection,
) -> Result<Option<FileInfo>, String> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };

    let size = metadata.len();
    // max_file_size 为 0 表示无限制
    let max_size_limit = if config.max_file_size == 0 {
        u64::MAX
    } else {
        config.max_file_size
    };
    if size < config.min_file_size || size > max_size_limit {
        return Ok(None);
    }

    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    if config.exclude_hidden && filename.starts_with('.') {
        return Ok(None);
    }

    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let hash = compute_file_hash(path, config.hash_algorithm).ok();
    let hash_algorithm = format!("{:?}", config.hash_algorithm).to_lowercase();

    let modified_at = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
        .flatten()
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let created_at = metadata.created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
        .flatten()
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();

    let path_str = path.to_string_lossy().to_string();

    conn.execute(
        "INSERT OR REPLACE INTO files
         (path, filename, size, hash, hash_algorithm, modified_at, created_at, file_extension, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &path_str,
            filename,
            size as i64,
            hash.as_ref(),
            &hash_algorithm,
            &modified_at,
            &created_at,
            &extension,
            &Utc::now().to_rfc3339(),
        ),
    ).map_err(|e| e.to_string())?;

    Ok(Some(FileInfo {
        id: 0,
        path: path_str,
        filename: filename.to_string(),
        size,
        hash,
        hash_algorithm,
        created_at,
        modified_at,
        file_extension: extension,
    }))
}

#[tauri::command(rename_all = "snake_case")]
async fn scan_directories(directories: Vec<String>, db_path: String, config: Option<ScanConfig>, allowed_roots: Option<Vec<String>>) -> Result<(), String> {
    info!("开始扫描目录: {:?}, 数据库: {}", directories, db_path);

    // 检查空目录列表
    if directories.is_empty() {
        return Err("未提供扫描目录".to_string());
    }

    // 检查数据库路径
    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    let config = config.unwrap_or_default();
    let roots = allowed_roots.unwrap_or_default();

    reset_scan_state();

    for dir in &directories {
        if dir.is_empty() {
            return Err("目录路径不能为空".to_string());
        }
        if !is_path_safe(dir, &roots) {
            return Err(format!("路径不安全: {}", dir));
        }
        // 检查目录是否存在
        if !Path::new(dir).exists() {
            return Err(format!("目录不存在: {}", dir));
        }
        if !Path::new(dir).is_dir() {
            return Err(format!("路径不是目录: {}", dir));
        }
    }

    // 在后台线程中执行扫描，避免阻塞主线程
    let db_path_clone = db_path.clone();
    let directories_clone = directories.clone();
    let config_clone = config.clone();
    let roots_clone = roots.clone();
    
    // 使用线程执行扫描，并等待其完成
    let handle = std::thread::spawn(move || {
        let mut conn = match init_database(&db_path_clone) {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("初始化数据库失败: {}", e);
                return Err::<(), String>(format!("初始化数据库失败: {}", e));
            }
        };

        for dir in directories_clone {
            if let Err(e) = scan_single_directory(&dir, &config_clone, &roots_clone, &mut conn) {
                warn!("扫描目录失败 {}: {}", dir, e);
                // 继续扫描其他目录，而不是直接返回错误
            }
        }
        
        info!("扫描完成");
        Ok::<(), String>(())
    });
    
    // 等待扫描线程完成
    match handle.join() {
        Ok(result) => result?,
        Err(_) => return Err("扫描线程执行失败".to_string()),
    }
    
    Ok(())
}

fn scan_single_directory(
    dir: &str,
    config: &ScanConfig,
    allowed_roots: &[String],
    conn: &mut Connection,
) -> Result<(), String> {
    use walkdir::WalkDir;
    
    info!("开始扫描目录: {}", dir);
    
    // 更新当前扫描的目录
    {
        if let Ok(mut state) = SCAN_STATE.lock() {
            state.current_directory = dir.to_string();
            state.scanned_directories += 1;
        }
    }
    
    // ============ Windows 极速 USN 扫描后端集成 ============
    #[cfg(windows)]
    {
        if is_admin() && dir.len() >= 2 && dir.chars().nth(1) == Some(':') {
            info!("环境符合 NTFS USN 索引加速要求，启动 MFT/USN 驱动级极速枚举...");
            match enumerate_files_via_usn(dir) {
                Ok(usn_files) => {
                    info!("USN 极速枚举成功，获取到 {} 个文件", usn_files.len());
                    {
                        if let Ok(mut state) = SCAN_STATE.lock() {
                            state.total_files += usn_files.len();
                            state.processed_files += usn_files.len();
                            if !usn_files.is_empty() {
                                state.current_file = usn_files.last().unwrap().filename.clone();
                            }
                        }
                    }
                    // 批量装入数据库
                    save_files_to_database(conn, &usn_files)?;
                    info!("USN 数据库批插入完成！扫描成功结束。");
                    return Ok(());
                }
                Err(e) => {
                    warn!("USN 扫描失败: {}, 降级回退至普通 WalkDir 并行扫描", e);
                }
            }
        }
    }
    
    // ============ Fallback 降级：企业级高性能并行流水线扫描 ============
    info!("启动高性能并发流水线 WalkDir 扫描: {}", dir);

    #[derive(Clone)]
    struct FileRecord {
        path_str: String,
        filename: String,
        size: i64,
        hash: Option<String>,
        hash_algorithm: String,
        modified_at: String,
        extension: String,
        volume_guid: Option<String>,
        relative_path: String,
    }

    fn process_file_item(path: &Path, config: &ScanConfig, allowed_roots: &[String]) -> Option<FileRecord> {
        let path_str = path.to_string_lossy().to_string();
        if !is_path_safe(&path_str, allowed_roots) {
            return None;
        }
        
        let metadata = std::fs::metadata(path).ok()?;
        let size = metadata.len();
        if size < config.min_file_size {
            return None;
        }
        if config.max_file_size > 0 && size > config.max_file_size {
            return None;
        }
        
        let filename = path.file_name()?.to_str()?.to_string();
        if config.exclude_hidden && filename.starts_with('.') {
            return None;
        }
        
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
            
        let hash = match compute_file_hash(path, config.hash_algorithm) {
            Ok(h) => Some(h),
            Err(_) => None
        };
        let hash_algorithm = format!("{:?}", config.hash_algorithm).to_lowercase();
        
        let modified_at = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
            .flatten()
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
            
        let mut volume_guid = None;
        let mut relative_path = filename.clone();
        
        #[cfg(windows)]
        {
            if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
                let drive_prefix = &path_str[..3];
                let drive_w = drive_prefix.replace("/", "\\");
                volume_guid = get_volume_guid_for_path(&drive_w);
                relative_path = path_str[3..].to_string();
            }
        }
        
        Some(FileRecord {
            path_str,
            filename,
            size: size as i64,
            hash,
            hash_algorithm,
            modified_at,
            extension,
            volume_guid,
            relative_path,
        })
    }

    let config_arc = std::sync::Arc::new(config.clone());
    let allowed_roots_arc = std::sync::Arc::new(allowed_roots.to_vec());
    let (tx, rx) = crossbeam::channel::unbounded::<Vec<FileRecord>>();
    
    // 收集一级子目录与一级直接文件
    let mut sub_dirs = Vec::new();
    let mut root_files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry_opt in entries {
            if let Ok(entry) = entry_opt {
                let path = entry.path();
                if path.is_dir() {
                    sub_dirs.push(path);
                } else if path.is_file() {
                    root_files.push(path);
                }
            }
        }
    }
    
    // 启动后台并行扫描线程，利用 rayon 线程池榨干多核性能
    let tx_clone = tx.clone();
    let config_t = config_arc.clone();
    let roots_t = allowed_roots_arc.clone();
    
    std::thread::spawn(move || {
        use rayon::prelude::*;
        
        // 并行遍历各子目录
        sub_dirs.par_iter().for_each(|sub_path| {
            let mut batch = Vec::new();
            for entry in WalkDir::new(sub_path)
                .max_depth(config_t.max_depth as usize)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if check_pause_with_condvar() {
                    break;
                }
                
                if !entry.path().is_file() {
                    continue;
                }
                
                if let Some(record) = process_file_item(entry.path(), &config_t, &roots_t) {
                    batch.push(record);
                    if batch.len() >= 300 {
                        let _ = tx_clone.send(batch.clone());
                        batch.clear();
                    }
                }
            }
            if !batch.is_empty() {
                let _ = tx_clone.send(batch);
            }
        });
        
        // 扫描根目录下的一级直接文件
        let mut root_batch = Vec::new();
        for path in root_files {
            if let Some(record) = process_file_item(&path, &config_t, &roots_t) {
                root_batch.push(record);
            }
        }
        if !root_batch.is_empty() {
            let _ = tx_clone.send(root_batch);
        }
    });
    
    drop(tx); // 必须在主线程释放发送端，rx 迭代器才能在全部子线程执行完后退出

    // 主线程消费端：使用 Connection 单线程批量写入事务，达成近乎无锁的吞吐量
    let mut processed = 0;
    let mut inserted = 0;
    
    // 开启初始事务
    let mut tx_db = conn.transaction().map_err(|e| format!("数据库开启事务失败: {}", e))?;
    
    while let Ok(batch) = rx.recv() {
        if check_pause_with_condvar() {
            return Err("扫描已取消".to_string());
        }

        for record in batch {
            processed += 1;
            
            // 实时向前端汇报状态
            {
                if let Ok(mut state) = SCAN_STATE.lock() {
                    state.current_file = record.filename.clone();
                    state.processed_files = processed;
                    state.total_files = processed + 20; // 滚动预估总数
                    state.recent_files.insert(0, record.filename.clone());
                    if state.recent_files.len() > 20 {
                        state.recent_files.truncate(20);
                    }
                }
            }
            
            if let Err(e) = tx_db.execute(
                "INSERT OR REPLACE INTO files 
                 (path, filename, size, hash, hash_algorithm, modified_at, file_extension, scanned_at, volume_guid, relative_path)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &record.path_str,
                    &record.filename,
                    record.size,
                    record.hash.as_ref(),
                    &record.hash_algorithm,
                    &record.modified_at,
                    &record.extension,
                    &Utc::now().to_rfc3339(),
                    record.volume_guid.as_ref(),
                    &record.relative_path,
                ),
            ) {
                warn!("写入数据库失败 {}: {}", record.path_str, e);
            } else {
                inserted += 1;
            }
        }
        
        // 周期性提交事务以释放锁，防止单一事务锁死大盘
        if processed % 3000 == 0 {
            tx_db.commit().map_err(|e| format!("事务阶段性提交失败: {}", e))?;
            tx_db = conn.transaction().map_err(|e| format!("事务重新创建失败: {}", e))?;
        }
    }
    
    tx_db.commit().map_err(|e| format!("最终事务提交失败: {}", e))?;
    
    // 最终对齐确切状态
    {
        if let Ok(mut state) = SCAN_STATE.lock() {
            state.total_files = processed;
            state.processed_files = processed;
        }
    }

    info!("高性能并发 WalkDir 扫描完成，共扫描 {} 个文件，成功录入数据库 {} 条记录", processed, inserted);
    Ok(())
}

#[tauri::command]
fn get_scan_progress() -> ScanProgress {
    let (total_files, processed_files, current_file, scanned_directories, current_directory, recent_files) = if let Ok(state) = SCAN_STATE.lock() {
        (state.total_files, state.processed_files, state.current_file.clone(), state.scanned_directories, state.current_directory.clone(), state.recent_files.clone())
    } else {
        (0, 0, String::new(), 0, String::new(), Vec::new())
    };
    
    let (control_state, _) = &**SCAN_CONTROL;
    let control = if let Ok(state) = control_state.lock() {
        *state
    } else {
        ScanControlState::Running
    };
    
    let percentage = if total_files > 0 {
        (processed_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    
    ScanProgress {
        total_files,
        processed_files,
        current_file,
        percentage,
        is_paused: control == ScanControlState::Paused,
        is_stopped: control == ScanControlState::Stopped,
        scanned_directories,
        current_directory,
        recent_files,
    }
}

#[tauri::command]
fn pause_scan() -> PauseResumeResult {
    set_scan_control(ScanControlState::Paused);
    info!("扫描已暂停");
    PauseResumeResult {
        success: true,
        is_paused: true,
        message: "扫描已暂停".to_string(),
    }
}

#[tauri::command]
fn resume_scan() -> PauseResumeResult {
    set_scan_control(ScanControlState::Running);
    info!("扫描已恢复");
    PauseResumeResult {
        success: true,
        is_paused: false,
        message: "扫描已恢复".to_string(),
    }
}

#[tauri::command]
fn stop_scan() -> PauseResumeResult {
    set_scan_control(ScanControlState::Stopped);
    info!("扫描已停止");
    PauseResumeResult {
        success: true,
        is_paused: false,
        message: "扫描已停止".to_string(),
    }
}

#[tauri::command(rename_all = "snake_case")]
fn find_duplicates(db_path: String) -> Result<DuplicateClassification, String> {
    info!("开始查找重复文件, 数据库路径: {}", db_path);

    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    let conn = init_database(&db_path)?;
    info!("数据库连接成功");
    
    // 1. 查找完全相同（文件名与哈希都相同）
    let complete_duplicates = find_complete_duplicates(&conn)?;
    info!("完全相同: {} 组", complete_duplicates.len());
    
    // 2. 查找名称相同（文件名相同，哈希不同）
    let name_duplicates = find_name_duplicates(&conn)?;
    info!("名称相同: {} 组", name_duplicates.len());
    
    // 3. 查找内容相同（哈希相同，文件名不同）
    let content_duplicates = find_content_duplicates(&conn)?;
    info!("内容相同: {} 组", content_duplicates.len());
    
    info!("找到完全相同: {} 组, 名称相同: {} 组, 内容相同: {} 组", 
          complete_duplicates.len(), name_duplicates.len(), content_duplicates.len());
    
    let result = DuplicateClassification {
        complete_duplicates,
        name_duplicates,
        content_duplicates,
    };
    
    // 记录返回的 JSON 数据
    match serde_json::to_string(&result) {
        Ok(json) => info!("返回的 JSON: {}", json),
        Err(e) => warn!("序列化 JSON 失败: {}", e),
    }
    
    Ok(result)
}

// 查找完全相同（文件名与哈希都相同）
fn find_complete_duplicates(conn: &Connection) -> Result<Vec<DuplicateGroup>, String> {
    // 使用 COLLATE NOCASE 使文件名比较不区分大小写（Windows 兼容）
    let mut stmt = conn.prepare(
        "SELECT filename, hash, COUNT(*) as count 
         FROM files 
         WHERE hash IS NOT NULL 
         GROUP BY filename COLLATE NOCASE, hash 
         HAVING count > 1"
    ).map_err(|e| e.to_string())?;
    
    let groups_data: Vec<(String, String, i64)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    info!("找到 {} 组可能的完全相同文件", groups_data.len());
    
    let mut groups = Vec::new();
    
    for (filename, hash, count) in groups_data {
        if check_pause_with_condvar() {
            break;
        }
        
        info!("检查: {} (哈希: {}, 计数: {})", filename, hash, count);
        
        let mut stmt = conn.prepare(
            "SELECT id, path, filename, size, hash, hash_algorithm, 
                    created_at, modified_at, file_extension 
             FROM files WHERE filename = ? COLLATE NOCASE AND hash = ?"
        ).map_err(|e| e.to_string())?;
        
        let files: Vec<FileInfo> = stmt.query_map([&filename, &hash], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        
        info!("  找到 {} 个文件", files.len());
        
        if files.len() > 1 {
            let total_size: u64 = files.iter().map(|f| f.size).sum();
            let wasted_space = total_size.saturating_sub(files[0].size);
            
            groups.push(DuplicateGroup {
                hash: hash.clone(),
                files,
                total_size,
                wasted_space,
            });
        }
    }
    
    info!("总共找到 {} 组完全相同文件", groups.len());
    Ok(groups)
}

// 查找名称相同但哈希不同
fn find_name_duplicates(conn: &Connection) -> Result<Vec<DuplicateGroup>, String> {
    let mut stmt = conn.prepare(
        "SELECT filename, COUNT(*) as count, COUNT(DISTINCT hash) as hash_count
         FROM files 
         WHERE hash IS NOT NULL
         GROUP BY filename COLLATE NOCASE
         HAVING count > 1 AND hash_count > 1"
    ).map_err(|e| e.to_string())?;
    
    let name_groups: Vec<(String, i64, i64)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    info!("找到 {} 组可能的名称相同文件", name_groups.len());
    
    let mut groups = Vec::new();
    
    for (filename, count, hash_count) in name_groups {
        if check_pause_with_condvar() {
            break;
        }
        
        info!("检查名称相同: {} (文件数: {}, 哈希数: {})", filename, count, hash_count);
        
        let mut stmt = conn.prepare(
            "SELECT id, path, filename, size, hash, hash_algorithm, 
                    created_at, modified_at, file_extension 
             FROM files WHERE filename = ? COLLATE NOCASE"
        ).map_err(|e| e.to_string())?;
        
        let files: Vec<FileInfo> = stmt.query_map([&filename], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        
        info!("  找到 {} 个文件", files.len());
        
        if files.len() > 1 {
            // 检查是否真的有不同哈希（避免误报）
            let unique_hashes: std::collections::HashSet<_> = files.iter().filter_map(|f| f.hash.as_ref()).collect();
            if unique_hashes.len() > 1 {
                let total_size: u64 = files.iter().map(|f| f.size).sum();
                let wasted_space = total_size.saturating_sub(files[0].size);
                
                let hash = format!("name:{}", filename);
                groups.push(DuplicateGroup {
                    hash,
                    files,
                    total_size,
                    wasted_space,
                });
            }
        }
    }
    
    Ok(groups)
}

// 查找内容相同但名称不同（哈希相同但文件名不同）
fn find_content_duplicates(conn: &Connection) -> Result<Vec<DuplicateGroup>, String> {
    let mut stmt = conn.prepare(
        "SELECT hash, COUNT(*) as count, COUNT(DISTINCT filename COLLATE NOCASE) as name_count
         FROM files 
         WHERE hash IS NOT NULL 
         GROUP BY hash 
         HAVING count > 1 AND name_count > 1"
    ).map_err(|e| e.to_string())?;
    
    let hash_groups: Vec<(String, i64, i64)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    info!("找到 {} 组可能的内容相同文件", hash_groups.len());
    
    let mut groups = Vec::new();
    
    for (hash, _count, _name_count) in hash_groups {
        if check_pause_with_condvar() {
            break;
        }
        
        let mut stmt = conn.prepare(
            "SELECT id, path, filename, size, hash, hash_algorithm, 
                    created_at, modified_at, file_extension 
             FROM files WHERE hash = ?"
        ).map_err(|e| e.to_string())?;
        
        let files: Vec<FileInfo> = stmt.query_map([&hash], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        
        if files.len() > 1 {
            let unique_names: std::collections::HashSet<String> = files.iter()
                .map(|f| f.filename.to_lowercase())
                .collect();
            if unique_names.len() > 1 {
                let total_size: u64 = files.iter().map(|f| f.size).sum();
                let wasted_space = total_size.saturating_sub(files[0].size);
                
                groups.push(DuplicateGroup {
                    hash: hash.clone(),
                    files,
                    total_size,
                    wasted_space,
                });
            }
        }
    }
    
    info!("总共找到 {} 组内容相同文件", groups.len());
    Ok(groups)
}

// 重复检测规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DuplicateRule {
    #[serde(rename = "hash")]
    Hash,           // 哈希相同（完全重复）
    #[serde(rename = "name")]
    Name,           // 文件名相同（可能内容不同）
    #[serde(rename = "size")]
    Size,           // 文件大小相同
    #[serde(rename = "name_size")]
    NameSize,       // 文件名和大小都相同
    #[serde(rename = "partial")]
    Partial,        // 部分匹配（匹配前若干字节）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateSearchResult {
    pub rule: DuplicateRule,
    pub groups: Vec<DuplicateGroup>,
    pub total_groups: usize,
    pub total_files: usize,
    pub total_wasted_space: u64,
}

#[tauri::command(rename_all = "snake_case")]
fn find_duplicates_advanced(
    db_path: String,
    rules: Vec<DuplicateRule>,
) -> Result<Vec<DuplicateSearchResult>, String> {
    info!("开始高级重复文件检测, 数据库路径: {}, 规则数: {}", db_path, rules.len());

    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    let conn = init_database(&db_path)?;
    let mut results = Vec::new();

    for rule in rules {
        let result = match rule {
            DuplicateRule::Hash => find_duplicates_by_hash(&conn)?,
            DuplicateRule::Name => find_duplicates_by_name(&conn)?,
            DuplicateRule::Size => find_duplicates_by_size(&conn)?,
            DuplicateRule::NameSize => find_duplicates_by_name_size(&conn)?,
            DuplicateRule::Partial => find_duplicates_by_partial(&conn, 102400)?, // 默认前100KB
        };
        results.push(result);
    }

    Ok(results)
}

// 提取所有有实际记录的文件列表（学术级优化：单次全表拉取辅助函数）
fn fetch_all_valid_files(conn: &Connection) -> Result<Vec<FileInfo>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size, hash, hash_algorithm, 
                created_at, modified_at, file_extension 
         FROM files"
    ).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map([], |row| {
        Ok(FileInfo {
            id: row.get(0)?,
            path: row.get(1)?,
            filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            size: row.get::<_, i64>(3)? as u64,
            hash: row.get(4)?,
            hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
            created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
            modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    Ok(files)
}

// 按哈希查找重复（完全重复）- 学术级优化：单次查询全表 + 内存分组
fn find_duplicates_by_hash(conn: &Connection) -> Result<DuplicateSearchResult, String> {
    info!("按哈希查找重复文件(单次全表内存分组优化)");
    let all_files = fetch_all_valid_files(conn)?;
    
    let mut groups_map: std::collections::HashMap<String, Vec<FileInfo>> = std::collections::HashMap::new();
    for file in all_files {
        if let Some(ref hash) = file.hash {
            if !hash.is_empty() {
                groups_map.entry(hash.clone()).or_default().push(file);
            }
        }
    }
    
    let mut groups = Vec::new();
    let mut total_files = 0;
    let mut total_wasted = 0u64;
    
    for (hash, group_files) in groups_map {
        if group_files.len() > 1 {
            let total_size: u64 = group_files.iter().map(|f| f.size).sum();
            let wasted_space = total_size.saturating_sub(group_files[0].size);
            total_files += group_files.len();
            total_wasted += wasted_space;
            
            groups.push(DuplicateGroup {
                hash: hash.clone(),
                files: group_files,
                total_size,
                wasted_space,
            });
        }
    }
    
    let total_groups = groups.len();
    info!("哈希重复: {} 组, {} 文件", total_groups, total_files);
    Ok(DuplicateSearchResult {
        rule: DuplicateRule::Hash,
        groups,
        total_groups,
        total_files,
        total_wasted_space: total_wasted,
    })
}

// 按文件名查找重复 - 学术级优化：单次查询全表 + 内存分组
fn find_duplicates_by_name(conn: &Connection) -> Result<DuplicateSearchResult, String> {
    info!("按文件名查找重复文件(单次全表内存分组优化)");
    let all_files = fetch_all_valid_files(conn)?;
    
    let mut groups_map: std::collections::HashMap<String, Vec<FileInfo>> = std::collections::HashMap::new();
    for file in all_files {
        let name_lower = file.filename.to_lowercase();
        if !name_lower.is_empty() {
            groups_map.entry(name_lower).or_default().push(file);
        }
    }
    
    let mut groups = Vec::new();
    let mut total_files = 0;
    let mut total_wasted = 0u64;
    
    for (name_lower, mut group_files) in groups_map {
        group_files.sort_by_key(|f| f.id);
        
        if group_files.len() > 1 {
            let unique_hashes: std::collections::HashSet<_> = group_files.iter().filter_map(|f| f.hash.as_ref()).collect();
            if unique_hashes.len() > 1 {
                let total_size: u64 = group_files.iter().map(|f| f.size).sum();
                let wasted_space = total_size.saturating_sub(group_files[0].size);
                total_files += group_files.len();
                total_wasted += wasted_space;
                
                let hash = format!("name:{}", name_lower);
                groups.push(DuplicateGroup {
                    hash,
                    files: group_files,
                    total_size,
                    wasted_space,
                });
            }
        }
    }
    
    let total_groups = groups.len();
    info!("文件名重复: {} 组, {} 文件", total_groups, total_files);
    Ok(DuplicateSearchResult {
        rule: DuplicateRule::Name,
        groups,
        total_groups,
        total_files,
        total_wasted_space: total_wasted,
    })
}

// 按大小查找重复 - 学术级优化：单次查询全表 + 内存分组
fn find_duplicates_by_size(conn: &Connection) -> Result<DuplicateSearchResult, String> {
    info!("按大小查找重复文件(单次全表内存分组优化)");
    let all_files = fetch_all_valid_files(conn)?;
    
    let mut groups_map: std::collections::HashMap<u64, Vec<FileInfo>> = std::collections::HashMap::new();
    for file in all_files {
        if file.size > 0 {
            groups_map.entry(file.size).or_default().push(file);
        }
    }
    
    let mut groups = Vec::new();
    let mut total_files = 0;
    let mut total_wasted = 0u64;
    
    for (size, mut group_files) in groups_map {
        group_files.sort_by_key(|f| f.id);
        
        if group_files.len() > 1 {
            let total_size: u64 = group_files.iter().map(|f| f.size).sum();
            let wasted_space = total_size.saturating_sub(group_files[0].size);
            total_files += group_files.len();
            total_wasted += wasted_space;
            
            let hash = format!("size:{}", size);
            groups.push(DuplicateGroup {
                hash,
                files: group_files,
                total_size,
                wasted_space,
            });
        }
    }
    
    let total_groups = groups.len();
    info!("大小重复: {} 组, {} 文件", total_groups, total_files);
    Ok(DuplicateSearchResult {
        rule: DuplicateRule::Size,
        groups,
        total_groups,
        total_files,
        total_wasted_space: total_wasted,
    })
}

// 按文件名和大小查找重复 - 学术级优化：单次查询全表 + 内存分组
fn find_duplicates_by_name_size(conn: &Connection) -> Result<DuplicateSearchResult, String> {
    info!("按文件名和大小查找重复文件(单次全表内存分组优化)");
    let all_files = fetch_all_valid_files(conn)?;
    
    let mut groups_map: std::collections::HashMap<(String, u64), Vec<FileInfo>> = std::collections::HashMap::new();
    for file in all_files {
        let name_lower = file.filename.to_lowercase();
        if !name_lower.is_empty() && file.size > 0 {
            groups_map.entry((name_lower, file.size)).or_default().push(file);
        }
    }
    
    let mut groups = Vec::new();
    let mut total_files = 0;
    let mut total_wasted = 0u64;
    
    for ((name_lower, size), mut group_files) in groups_map {
        group_files.sort_by_key(|f| f.id);
        
        if group_files.len() > 1 {
            let total_size: u64 = group_files.iter().map(|f| f.size).sum();
            let wasted_space = total_size.saturating_sub(group_files[0].size);
            total_files += group_files.len();
            total_wasted += wasted_space;
            
            let hash = format!("namesize:{}:{}", name_lower, size);
            groups.push(DuplicateGroup {
                hash,
                files: group_files,
                total_size,
                wasted_space,
            });
        }
    }
    
    let total_groups = groups.len();
    info!("文件名和大小重复: {} 组, {} 文件", total_groups, total_files);
    Ok(DuplicateSearchResult {
        rule: DuplicateRule::NameSize,
        groups,
        total_groups,
        total_files,
        total_wasted_space: total_wasted,
    })
}

// 流式计算文件前段字节的散列值（部分哈希提取）
fn compute_file_partial_bytes_hash(path: &str, partial_size: u64) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};
    use xxhash_rust::xxh3::Xxh3;
    
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let file_len = metadata.len();
    
    let mut hasher = Xxh3::new();
    
    // 如果文件长度小于 partial_size * 3，则直接进行流式全量读取哈希
    if file_len <= partial_size * 3 {
        let mut buffer = vec![0u8; 8192];
        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
    } else {
        // 多点采样匹配（头部，中部，尾部），限制最大单采样块为 16384 字节，在大采样设置下保证性能
        let chunk_size = partial_size.min(16384);
        let mut buffer = vec![0u8; chunk_size as usize];
        
        // 1. 采样头部
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        hasher.update(&buffer[..bytes_read]);
        
        // 2. 采样中部
        let middle_offset = file_len / 2 - chunk_size / 2;
        file.seek(SeekFrom::Start(middle_offset)).map_err(|e| e.to_string())?;
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        hasher.update(&buffer[..bytes_read]);
        
        // 3. 采样尾部
        let tail_offset = file_len - chunk_size;
        file.seek(SeekFrom::Start(tail_offset)).map_err(|e| e.to_string())?;
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:016x}", hasher.digest()))
}

// 商业级部分匹配（Partial Match）查重核心算法 - 并行指纹计算
fn find_duplicates_by_partial(conn: &Connection, partial_size: u64) -> Result<DuplicateSearchResult, String> {
    info!("按文件部分内容查找重复文件, 比对前 {} 字节", partial_size);
    
    let all_files = fetch_all_valid_files(conn)?;
    let files: Vec<FileInfo> = all_files.into_iter().filter(|f| f.size > 0).collect();
    
    use rayon::prelude::*;
    let file_partial_hashes: Vec<(FileInfo, String)> = files.par_iter().map(|file| {
        let p_hash = compute_file_partial_bytes_hash(&file.path, partial_size).unwrap_or_default();
        (file.clone(), p_hash)
    }).filter(|(_, h)| !h.is_empty()).collect();
    
    // 学术诚信级碰撞隔离：将 file.size 拼接进 key，绝不允许大小不同的文件被判定为重复
    let mut groups_map: std::collections::HashMap<String, Vec<FileInfo>> = std::collections::HashMap::new();
    for (file, p_hash) in file_partial_hashes {
        let group_key = format!("{}:{}", file.size, p_hash);
        groups_map.entry(group_key).or_default().push(file);
    }
    
    let mut groups = Vec::new();
    let mut total_files = 0;
    let mut total_wasted = 0u64;
    
    for (group_key, mut group_files) in groups_map {
        group_files.sort_by_key(|f| f.id);
        
        if group_files.len() > 1 {
            let total_size: u64 = group_files.iter().map(|f| f.size).sum();
            let wasted_space = total_size.saturating_sub(group_files[0].size);
            total_files += group_files.len();
            total_wasted += wasted_space;
            
            // 拆分出 p_hash 部分用于 hash 字段显示
            let parts: Vec<&str> = group_key.split(':').collect();
            let p_hash = parts.get(1).unwrap_or(&"");
            
            groups.push(DuplicateGroup {
                hash: format!("partial:{}:{}", p_hash, partial_size),
                files: group_files,
                total_size,
                wasted_space,
            });
        }
    }
    
    let total_groups = groups.len();
    info!("部分匹配重复: {} 组, {} 文件", total_groups, total_files);
    Ok(DuplicateSearchResult {
        rule: DuplicateRule::Partial,
        groups,
        total_groups,
        total_files,
        total_wasted_space: total_wasted,
    })
}

#[tauri::command(rename_all = "snake_case")]
fn find_duplicates_partial(db_path: String, partial_size: Option<u64>) -> Result<DuplicateSearchResult, String> {
    info!("通过 Tauri 外部接口调用部分匹配查重");
    let conn = init_database(&db_path)?;
    let size = partial_size.unwrap_or(102400); // 默认 100KB
    find_duplicates_by_partial(&conn, size)
}

#[tauri::command(rename_all = "snake_case")]
fn verify_file_hashes_match(keep_path: String, delete_paths: Vec<String>) -> Result<bool, String> {
    use std::path::Path;
    
    let keep_p = Path::new(&keep_path);
    if !keep_p.exists() {
        return Err(format!("保留的文件不存在: {}", keep_path));
    }
    
    let keep_hash = compute_file_hash(keep_p, HashAlgorithm::XxHash3)?;
    
    for del_path in delete_paths {
        let del_p = Path::new(&del_path);
        if !del_p.exists() {
            return Err(format!("待删除的文件不存在: {}", del_path));
        }
        
        let del_hash = compute_file_hash(del_p, HashAlgorithm::XxHash3)?;
        if keep_hash != del_hash {
            return Ok(false);
        }
    }
    
    Ok(true)
}


// ============ MFT/USN 驱动级 Windows USN 极速枚举与卷感知模块 ============

#[cfg(windows)]
#[repr(C, packed(4))]
#[allow(non_camel_case_types, dead_code)]
struct USN_RECORD_V2 {
    record_length: u32,
    major_version: u16,
    minor_version: u16,
    file_reference_number: u64,
    parent_file_reference_number: u64,
    usn: i64,
    time_stamp: i64,
    reason: u32,
    reason_info: u32,
    source_info: u32,
    security_id: u32,
    file_attributes: u32,
    file_name_length: u16,
    file_name_offset: u16,
    file_name: [u16; 1],
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct MFT_ENUM_DATA_V0 {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct USN_JOURNAL_DATA_V0 {
    usn_journal_id: u64,
    first_usn: i64,
    next_usn: i64,
    lowest_valid_usn: i64,
    max_usn: i64,
    maximum_size: u64,
    allocation_delta: u64,
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct CREATE_USN_JOURNAL_DATA {
    maximum_size: u64,
    allocation_delta: u64,
}

// 纯内核 API 绑定链接，免除对 windows-sys 缺失 feature 的依赖
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: isize,
    ) -> isize;
    
    fn CloseHandle(hObject: isize) -> i32;
    
    fn DeviceIoControl(
        hDevice: isize,
        dwIoControlCode: u32,
        lpInBuffer: *const std::ffi::c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut std::ffi::c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;
}

// 获取指定路径的卷 GUID
#[cfg(windows)]
fn get_volume_guid_for_path(path: &str) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    
    let path_w: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let mut volume_path_w = vec![0u16; 1024];
    
    // 获取根挂载盘符路径
    let res = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW(
            path_w.as_ptr(),
            volume_path_w.as_mut_ptr(),
            1024
        )
    };
    
    if res == 0 {
        return None;
    }
    
    let mut guid_w = vec![0u16; 50];
    let res = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW(
            volume_path_w.as_ptr(),
            guid_w.as_mut_ptr(),
            50
        )
    };
    
    if res == 0 {
        return None;
    }
    
    let len = guid_w.iter().position(|&x| x == 0).unwrap_or(50);
    Some(String::from_utf16_lossy(&guid_w[..len]))
}

#[cfg(not(windows))]
fn get_volume_guid_for_path(_path: &str) -> Option<String> {
    None
}

// 检查并动态重映射挂载盘符（卷感知热重定位）
#[tauri::command(rename_all = "snake_case")]
fn check_and_remap_volumes(db_path: String) -> Result<serde_json::Value, String> {
    info!("执行盘符感知检测与动态映射");
    let conn = init_database(&db_path)?;
    
    // 查询库中已记录的所有卷 GUID
    let mut stmt = conn.prepare("SELECT DISTINCT volume_guid FROM files WHERE volume_guid IS NOT NULL")
        .map_err(|e| e.to_string())?;
    
    let db_guids: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        
    let mut remapped_count = 0;
    let mut remap_details = Vec::new();
    
    #[cfg(windows)]
    {
        for drive_letter in b'A'..=b'Z' {
            let drive_str = format!("{}:\\", drive_letter as char);
            if let Some(current_guid) = get_volume_guid_for_path(&drive_str) {
                if db_guids.contains(&current_guid) {
                    let mut stmt = conn.prepare("SELECT path FROM files WHERE volume_guid = ? LIMIT 1")
                        .map_err(|e| e.to_string())?;
                    
                    let last_path: Option<String> = stmt.query_row([&current_guid], |row| row.get(0)).ok();
                    if let Some(path) = last_path {
                        let last_drive_prefix = if path.len() >= 3 { &path[..3] } else { "" };
                        let current_drive_prefix = &drive_str[..3];
                        
                        if !last_drive_prefix.is_empty() && last_drive_prefix.to_lowercase() != current_drive_prefix.to_lowercase() {
                            info!("检测到磁盘卷 GUID: {} 漂移: {} -> {}", current_guid, last_drive_prefix, current_drive_prefix);
                            
                            let old_pattern = format!("{}%", last_drive_prefix);
                            let affected: usize = conn.execute(
                                "UPDATE files SET path = ?1 || SUBSTR(path, 4) WHERE volume_guid = ?2 AND path LIKE ?3",
                                &[current_drive_prefix, &current_guid, &old_pattern]
                            ).map_err(|e| e.to_string())?;
                            
                            remapped_count += affected;
                            remap_details.push(format!("盘符 {} 映射更新至 {}, 影响文件数: {}", last_drive_prefix, current_drive_prefix, affected));
                        }
                    }
                }
            }
        }
    }
    
    Ok(serde_json::json!({
        "success": true,
        "remapped_files": remapped_count,
        "details": remap_details
    }))
}

// 检查是否具备管理员权限
#[cfg(windows)]
fn is_admin() -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    let volume_name: Vec<u16> = OsStr::new(r"\\.\PhysicalDrive0").encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            volume_name.as_ptr(),
            0x80000000, // GENERIC_READ
            1, // FILE_SHARE_READ
            std::ptr::null(),
            3, // OPEN_EXISTING
            0,
            0
        )
    };
    if handle != -1 {
        unsafe { CloseHandle(handle) };
        true
    } else {
        false
    }
}

#[cfg(not(windows))]
fn is_admin() -> bool {
    false
}

// Windows 底层 USN 日志极速文件枚举实现（MFT/USN 驱动级核心实现）
#[cfg(windows)]
fn enumerate_files_via_usn(scan_root: &str) -> Result<Vec<FileInfo>, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use std::collections::HashMap;
    
    info!("启动 Windows 内核 USN 日志枚举，扫描路径: {}", scan_root);
    
    let mount_point = if scan_root.len() >= 3 { &scan_root[..3] } else { scan_root };
    let _volume_guid = get_volume_guid_for_path(mount_point).unwrap_or_default();
    
    let volume_device = format!(r"\\.\{}", &mount_point[..2]);
    let device_w: Vec<u16> = OsStr::new(&volume_device).encode_wide().chain(Some(0)).collect();
    
    let handle = unsafe {
        CreateFileW(
            device_w.as_ptr(),
            0x80000000, // GENERIC_READ
            1 | 2, // FILE_SHARE_READ | FILE_SHARE_WRITE
            std::ptr::null(),
            3, // OPEN_EXISTING
            0x02000000, // FILE_FLAG_BACKUP_SEMANTICS
            0
        )
    };
    
    if handle == -1 {
        return Err("无法获取物理磁盘卷句柄，请尝试以管理员身份运行。".to_string());
    }
    
    let mut journal_data = USN_JOURNAL_DATA_V0 {
        usn_journal_id: 0, first_usn: 0, next_usn: 0, lowest_valid_usn: 0, max_usn: 0, maximum_size: 0, allocation_delta: 0
    };
    let mut bytes_returned = 0;
    
    let res = unsafe {
        DeviceIoControl(
            handle,
            0x000900f4, // FSCTL_QUERY_USN_JOURNAL
            std::ptr::null(),
            0,
            &mut journal_data as *mut _ as *mut _,
            std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut()
        )
    };
    
    if res == 0 {
        info!("未发现活动 USN 日志，试图初始化新 Journal...");
        let create_data = CREATE_USN_JOURNAL_DATA {
            maximum_size: 32 * 1024 * 1024,
            allocation_delta: 8 * 1024 * 1024,
        };
        let mut temp_bytes = 0;
        let create_res = unsafe {
            DeviceIoControl(
                handle,
                0x000900e7, // FSCTL_CREATE_USN_JOURNAL
                &create_data as *const _ as *const _,
                std::mem::size_of::<CREATE_USN_JOURNAL_DATA>() as u32,
                std::ptr::null_mut(),
                0,
                &mut temp_bytes,
                std::ptr::null_mut()
            )
        };
        if create_res == 0 {
            unsafe { CloseHandle(handle) };
            return Err("创建 USN 日志失败。".to_string());
        }
        
        unsafe {
            DeviceIoControl(
                handle,
                0x000900f4,
                std::ptr::null(),
                0,
                &mut journal_data as *mut _ as *mut _,
                std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut()
            );
        }
    }
    
    let mut mft_enum_data = MFT_ENUM_DATA_V0 {
        start_file_reference_number: 0,
        low_usn: 0,
        high_usn: journal_data.next_usn,
    };
    
    let mut file_map: HashMap<u64, (String, u64)> = HashMap::with_capacity(100000);
    let mut raw_files: Vec<(u64, u64, String, u64, String)> = Vec::with_capacity(100000);
    let mut buffer = vec![0u8; 65536];
    
    loop {
        let mut bytes_ret = 0;
        let res = unsafe {
            DeviceIoControl(
                handle,
                0x000900b3, // FSCTL_ENUM_USN_DATA
                &mft_enum_data as *const _ as *const _,
                std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_ret,
                std::ptr::null_mut()
            )
        };
        
        if res == 0 || bytes_ret < 8 {
            break;
        }
        
        let next_start = u64::from_ne_bytes(buffer[0..8].try_into().unwrap());
        mft_enum_data.start_file_reference_number = next_start;
        
        let mut offset = 8;
        while offset < bytes_ret as usize {
            let record = unsafe { &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2) };
            if record.record_length == 0 {
                break;
            }
            
            let name_ptr = unsafe { buffer.as_ptr().add(offset + record.file_name_offset as usize) } as *const u16;
            let name_len = (record.file_name_length / 2) as usize;
            let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
            let filename = String::from_utf16_lossy(name_slice);
            
            let file_ref = record.file_reference_number;
            let parent_ref = record.parent_file_reference_number;
            let is_dir = (record.file_attributes & 0x00000010) != 0;
            
            if is_dir {
                file_map.insert(file_ref, (filename, parent_ref));
            } else {
                let ext = Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                raw_files.push((file_ref, parent_ref, filename, 0u64, ext));
            }
            
            offset += record.record_length as usize;
        }
        
        if next_start == 0 {
            break;
        }
    }
    
    unsafe { CloseHandle(handle) };
    
    info!("构建 MFT 目录树并生成绝对路径, 总文件数: {}", raw_files.len());
    let file_map_arc = Arc::new(file_map);
    
    use rayon::prelude::*;
    let now_str = chrono::Utc::now().to_rfc3339();
    
    let scan_root_cleaned = scan_root.replace("/", "\\");
    let scan_root_lower = scan_root_cleaned.to_lowercase();
    let drive_prefix = &mount_point[..2];
    
    let result_files: Vec<FileInfo> = raw_files.par_iter().map(|(_ref_num, parent_ref, filename, _size, ext)| {
        let mut path_parts = Vec::new();
        let mut current_parent = *parent_ref;
        
        while let Some((parent_name, next_parent)) = file_map_arc.get(&current_parent) {
            path_parts.push(parent_name.clone());
            current_parent = *next_parent;
            if current_parent == 0 || path_parts.len() > 50 {
                break;
            }
        }
        
        path_parts.reverse();
        let mut relative_dir = path_parts.join("\\");
        if !relative_dir.is_empty() {
            relative_dir.push_str("\\");
        }
        
        let full_path = format!("{}\\{}{}", drive_prefix, relative_dir, filename);
        
        FileInfo {
            id: 0,
            path: full_path,
            filename: filename.clone(),
            size: 0u64,
            hash: None,
            hash_algorithm: "xxhash3".to_string(),
            created_at: now_str.clone(),
            modified_at: now_str.clone(),
            file_extension: ext.clone(),
        }
    })
    .filter(|f| f.path.to_lowercase().starts_with(&scan_root_lower))
    .map(|mut f| {
        f.path = f.path.replace("\\", "/");
        FileInfo {
            id: 0,
            path: f.path.clone(),
            filename: f.filename.clone(),
            size: 0,
            hash: None,
            hash_algorithm: "xxhash3".to_string(),
            created_at: f.created_at.clone(),
            modified_at: f.modified_at.clone(),
            file_extension: f.file_extension.clone(),
        }
    })
    .collect();
    
    Ok(result_files)
}

#[cfg(not(windows))]
fn enumerate_files_via_usn(_scan_root: &str) -> Result<Vec<FileInfo>, String> {
    Err("USN 快速枚举仅支持 Windows 系统。".to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn check_database_has_files(db_path: String) -> Result<bool, String> {
    info!("检查数据库是否有文件: {}", db_path);
    
    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }
    
    let conn = init_database(&db_path)?;
    
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .unwrap_or(0);
    
    let hash_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM files WHERE hash IS NOT NULL", [], |row| row.get(0))
        .unwrap_or(0);
    
    let duplicate_hash_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (
                SELECT hash FROM files 
                WHERE hash IS NOT NULL 
                GROUP BY hash 
                HAVING COUNT(*) > 1
            )", 
            [], 
            |row| row.get(0)
        )
        .unwrap_or(0);
    
    info!("数据库中文件数量: {}, 有哈希值: {}, 重复哈希: {}", count, hash_count, duplicate_hash_count);
    Ok(count > 0)
}
// 调试命令：获取数据库中的样本数据
#[tauri::command(rename_all = "snake_case")]
fn debug_database_files(db_path: String) -> Result<String, String> {
    info!("调试数据库文件: {}", db_path);
    
    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }
    
    let conn = init_database(&db_path)?;
    
    // 获取总文件数
    let total_files: i64 = conn
        .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .unwrap_or(0);
    
    // 获取有哈希的文件数
    let with_hash: i64 = conn
        .query_row("SELECT COUNT(*) FROM files WHERE hash IS NOT NULL", [], |row| row.get(0))
        .unwrap_or(0);
    
    // 获取无哈希的文件数
    let without_hash: i64 = conn
        .query_row("SELECT COUNT(*) FROM files WHERE hash IS NULL", [], |row| row.get(0))
        .unwrap_or(0);
    
    // 获取样本文件（前10个）
    let mut stmt = conn.prepare(
        "SELECT path, filename, hash, size FROM files LIMIT 10"
    ).map_err(|e| e.to_string())?;
    
    let files: Vec<(String, String, Option<String>, i64)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
        ))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 查找有相同哈希的文件（按哈希分组）
    let mut stmt = conn.prepare(
        "SELECT hash, COUNT(*) as cnt, GROUP_CONCAT(filename, ' | ') as names 
         FROM files 
         WHERE hash IS NOT NULL 
         GROUP BY hash 
         HAVING cnt > 1 
         LIMIT 5"
    ).map_err(|e| e.to_string())?;
    
    let duplicates: Vec<(String, i64, String)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 查找文件名相同但哈希不同的文件
    let mut stmt = conn.prepare(
        "SELECT filename, COUNT(*) as cnt, COUNT(DISTINCT hash) as hash_cnt,
                GROUP_CONCAT(DISTINCT hash, ' | ') as hashes
         FROM files 
         WHERE hash IS NOT NULL 
         GROUP BY filename COLLATE NOCASE 
         HAVING cnt > 1 AND hash_cnt > 1 
         LIMIT 5"
    ).map_err(|e| e.to_string())?;
    
    let name_dups: Vec<(String, i64, i64, String)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    let result = format!(
        "=== 数据库统计 ===\n总文件数: {}\n有哈希: {}\n无哈希: {}\n\n=== 样本文件（前10个）===\n{}\n\n=== 相同哈希的文件（前5组）===\n{}\n\n=== 文件名相同但哈希不同（前5组）===\n{}",
        total_files,
        with_hash,
        without_hash,
        if files.is_empty() {
            "  (无文件)".to_string()
        } else {
            files.iter().map(|(p, f, h, s)| {
                format!("  {} | 哈希={} | 大小={} | 路径={}", 
                    f, 
                    h.as_ref().map(|x| &x[..16.min(x.len())]).unwrap_or("NULL"),
                    s,
                    p
                )
            }).collect::<Vec<_>>().join("\n")
        },
        if duplicates.is_empty() {
            "  (无重复哈希)".to_string()
        } else {
            duplicates.iter().map(|(h, c, n)| {
                format!("  哈希: {}... | 数量: {} | 文件: {}", &h[..16.min(h.len())], c, n)
            }).collect::<Vec<_>>().join("\n")
        },
        if name_dups.is_empty() {
            "  (无名称相同但哈希不同的文件)".to_string()
        } else {
            name_dups.iter().map(|(f, c, hc, hs)| {
                format!("  文件名: {} | 文件数: {} | 哈希数: {} | 哈希: {}", f, c, hc, 
                    hs.split(" | ").map(|h| &h[..8.min(h.len())]).collect::<Vec<_>>().join(", "))
            }).collect::<Vec<_>>().join("\n")
        }
    );
    
    info!("调试结果:\n{}", result);
    Ok(result)
}

#[tauri::command(rename_all = "snake_case")]
fn delete_file(
    path: String,
    db_path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<(), String> {
    info!("删除文件: {}", path);
    
    let roots = allowed_roots.unwrap_or_default();
    
    if !is_path_safe(&path, &roots) {
        let conn = init_database(&db_path)?;
        let _ = log_operation(&conn, "DELETE", &[path.clone()], None, None, "FAILED", Some("路径不安全"));
        return Err("路径不安全".to_string());
    }
    
    if is_file_locked(Path::new(&path)) {
        let conn = init_database(&db_path)?;
        let _ = log_operation(&conn, "DELETE", &[path.clone()], None, None, "FAILED", Some("文件被锁定"));
        return Err("文件被锁定".to_string());
    }
    
    // 先删除数据库记录，确保数据一致性
    let conn = init_database(&db_path)?;
    conn.execute("DELETE FROM files WHERE path = ?", [&path])
        .map_err(|e| {
            let _ = log_operation(&conn, "DELETE", &[path.clone()], None, None, "FAILED", Some(&format!("数据库删除失败: {}", e)));
            format!("数据库删除失败: {}", e)
        })?;
    
    // 再删除物理文件
    if let Err(e) = std::fs::remove_file(&path) {
        // 物理文件删除失败，记录警告但数据库已删除
        warn!("物理文件删除失败 {}: {}，但数据库记录已删除", path, e);
        let _ = log_operation(&conn, "DELETE", &[path.clone()], None, None, "PARTIAL", Some(&format!("物理文件删除失败: {}", e)));
    } else {
        // 记录操作日志
        let _ = log_operation(&conn, "DELETE", &[path.clone()], None, None, "SUCCESS", None);
        
        // 记录审计日志
        let _ = log_audit_event(&conn, "FILE_DELETED", "INFO", &path, "DELETE", "文件已删除", "SUCCESS");
    }
    
    info!("文件删除成功: {}", path);
    Ok(())
}

fn check_same_volume(p1: &std::path::Path, p2: &std::path::Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        // 物理盘卷前缀盘符校验 (Windows 平台稳定版)
        let abs1 = std::fs::canonicalize(p1).map_err(|e| format!("无法规范化路径 {}: {}", p1.display(), e))?;
        let abs2 = std::fs::canonicalize(p2).map_err(|e| format!("无法规范化路径 {}: {}", p2.display(), e))?;
        
        let prefix1 = abs1.to_string_lossy();
        let prefix2 = abs2.to_string_lossy();
        
        let get_drive = |path_str: &str| -> String {
            if path_str.starts_with(r"\\?\") {
                path_str.chars().skip(4).take(2).collect::<String>()
            } else {
                path_str.chars().take(2).collect::<String>()
            }
        };

        let d1 = get_drive(&prefix1);
        let d2 = get_drive(&prefix2);

        if d1.is_empty() || d2.is_empty() {
            Err("无法提取文件系统驱动器盘符前缀，无法安全执行硬链接判定".to_string())
        } else {
            Ok(d1.eq_ignore_ascii_case(&d2))
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::MetadataExt;
        let m1 = std::fs::metadata(p1).map_err(|e| format!("读取源文件元数据失败 {}: {}", p1.display(), e))?;
        let m2 = std::fs::metadata(p2).map_err(|e| format!("读取待链接文件元数据失败 {}: {}", p2.display(), e))?;
        Ok(m1.dev() == m2.dev())
    }
}

#[tauri::command(rename_all = "snake_case")]
fn replace_files_with_hardlinks(
    keep_path: String,
    replace_paths: Vec<String>,
    db_path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<usize, String> {
    info!("执行硬链接去重替换。源文件: {}, 待链接项: {} 个", keep_path, replace_paths.len());
    
    let roots = allowed_roots.unwrap_or_default();
    let keep_p = Path::new(&keep_path);
    
    if !keep_p.exists() {
        return Err(format!("源文件不存在: {}", keep_path));
    }
    
    if !is_path_safe(&keep_path, &roots) {
        return Err("源文件路径安全校验失败".to_string());
    }

    let conn = init_database(&db_path)?;
    let mut success_count = 0;
    
    for path in replace_paths {
        let rep_p = Path::new(&path);
        
        if !rep_p.exists() {
            warn!("待链接替换的文件物理不存在，跳过: {}", path);
            continue;
        }
        
        if !is_path_safe(&path, &roots) {
            warn!("路径不安全，拦截链接替换: {}", path);
            continue;
        }
        
        // 1. 物理卷校验 (硬链接必须在同一分区)
        match check_same_volume(keep_p, rep_p) {
            Ok(is_same) => {
                if !is_same {
                    let err_msg = format!("物理盘卷ID不一致，无法建立硬链接: {} 与 {}", keep_path, path);
                    let _ = log_operation(&conn, "HARDLINK", &[path.clone()], None, None, "FAILED", Some(&err_msg));
                    return Err(err_msg);
                }
            }
            Err(e) => {
                let _ = log_operation(&conn, "HARDLINK", &[path.clone()], None, None, "FAILED", Some(&e));
                return Err(e);
            }
        }
        
        // 2. 事务型防灾链接建立逻辑 (备份 -> 链接 -> 确认或回滚)
        let temp_bak_path = format!("{}.dfh_temp", path);
        let temp_p = Path::new(&temp_bak_path);
        
        // Step A: 备份重命名
        if let Err(e) = std::fs::rename(rep_p, temp_p) {
            let err_msg = format!("建立硬链接事务重命名备份失败: {}，原因: {}", path, e);
            let _ = log_operation(&conn, "HARDLINK", &[path.clone()], None, None, "FAILED", Some(&err_msg));
            return Err(err_msg);
        }
        
        // Step B: 创建物理硬链接
        if let Err(e) = std::fs::hard_link(keep_p, rep_p) {
            // 失败，执行事务性回滚复原
            if let Err(rollback_err) = std::fs::rename(temp_p, rep_p) {
                log::error!("重大系统错误：硬链接失败且备份回滚失败！文件: {}，原因: {}", path, rollback_err);
            }
            let err_msg = format!("创建物理硬链接失败: {}，原因: {}。系统已安全自动回滚。", path, e);
            let _ = log_operation(&conn, "HARDLINK", &[path.clone()], None, None, "FAILED", Some(&err_msg));
            return Err(err_msg);
        }
        
        // Step C: 成功，清除备份文件
        if let Err(e) = std::fs::remove_file(temp_p) {
            warn!("清除硬链接事务备份文件失败 {}: {} (链接本身已建立)", temp_bak_path, e);
        }
        
        // 3. 数据库数据流清理 (对齐去重结果)
        let _ = conn.execute("DELETE FROM files WHERE path = ?", [&path]);
        let _ = log_operation(&conn, "HARDLINK", &[path.clone()], None, None, "SUCCESS", None);
        let _ = log_audit_event(&conn, "HARDLINK_CREATED", "INFO", &path, "HARDLINK", &format!("已成功替换为指向 {} 的硬链接", keep_path), "SUCCESS");
        
        success_count += 1;
    }
    
    Ok(success_count)
}


#[tauri::command]
fn smart_select_duplicates(
    groups: Vec<DuplicateGroup>,
    strategy: KeepStrategy,
) -> Result<Vec<Vec<bool>>, String> {
    info!("智能选择策略: {}", strategy.strategy_type);
    
    // 验证策略类型
    let valid_strategies = ["newest", "oldest", "shortest", "longest"];
    if !valid_strategies.contains(&strategy.strategy_type.as_str()) {
        return Err(format!("无效的策略类型: {}", strategy.strategy_type));
    }
    
    let mut selections = Vec::new();
    
    for group in groups {
        let mut group_selection = vec![false; group.files.len()];
        
        if group.files.is_empty() {
            selections.push(group_selection);
            continue;
        }
        
        let keep_index = match strategy.strategy_type.as_str() {
            "newest" => {
                group.files.iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.modified_at.cmp(&b.modified_at))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            "oldest" => {
                group.files.iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.modified_at.cmp(&b.modified_at))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            "shortest" => {
                group.files.iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.path.len().cmp(&b.path.len()))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            "longest" => {
                group.files.iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.path.len().cmp(&b.path.len()))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            _ => 0,
        };
        
        for (i, selected) in group_selection.iter_mut().enumerate() {
            *selected = i != keep_index;
        }
        
        selections.push(group_selection);
    }
    
    Ok(selections)
}

#[tauri::command(rename_all = "snake_case")]
fn batch_delete_files(
    paths: Vec<String>,
    db_path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<usize, String> {
    info!("批量删除 {} 个文件", paths.len());
    
    let roots = allowed_roots.unwrap_or_default();
    let mut deleted_count = 0;
    let mut failed_paths: Vec<String> = Vec::new();
    
    let conn = init_database(&db_path)?;
    
    for path in &paths {
        if !is_path_safe(path, &roots) {
            warn!("跳过不安全路径: {}", path);
            failed_paths.push(format!("{} (路径不安全)", path));
            continue;
        }
        
        if is_file_locked(Path::new(path)) {
            warn!("文件被锁定: {}", path);
            failed_paths.push(format!("{} (文件被锁定)", path));
            continue;
        }
        
        // 先删除数据库记录
        if let Err(e) = conn.execute("DELETE FROM files WHERE path = ?", [path]) {
            warn!("数据库删除失败 {}: {}", path, e);
            failed_paths.push(format!("{} (数据库删除失败: {})", path, e));
            continue;
        }
        
        // 再删除物理文件
        if let Err(e) = std::fs::remove_file(path) {
            warn!("物理文件删除失败 {}: {}，但数据库记录已删除", path, e);
            failed_paths.push(format!("{} (物理文件删除失败: {})", path, e));
        }
        
        deleted_count += 1;
    }
    
    // 记录批量操作日志
    if deleted_count > 0 {
        let _ = log_operation(&conn, "BATCH_DELETE", &paths, None, None, "SUCCESS", 
            Some(&format!("成功删除 {}/{} 个文件", deleted_count, paths.len())));
    }
    
    if !failed_paths.is_empty() {
        warn!("批量删除部分失败: {:?}", failed_paths);
    }
    
    info!("成功删除 {} 个文件", deleted_count);
    Ok(deleted_count)
}

// ========== BK-Tree 图像指纹索引 ==========

struct BKNode {
    hash: u64,
    index: usize,
    children: std::collections::HashMap<u32, BKNode>,
}

impl BKNode {
    fn new(hash: u64, index: usize) -> Self {
        Self {
            hash,
            index,
            children: std::collections::HashMap::new(),
        }
    }
}

struct BKTree {
    root: Option<BKNode>,
}

impl BKTree {
    fn new() -> Self {
        Self { root: None }
    }

    fn insert(&mut self, hash: u64, index: usize) {
        if let Some(ref mut root) = self.root {
            Self::insert_node(root, hash, index);
        } else {
            self.root = Some(BKNode::new(hash, index));
        }
    }

    fn insert_node(parent: &mut BKNode, hash: u64, index: usize) {
        let dist = (parent.hash ^ hash).count_ones();
        if parent.children.contains_key(&dist) {
            if let Some(child) = parent.children.get_mut(&dist) {
                Self::insert_node(child, hash, index);
            }
        } else {
            parent.children.insert(dist, BKNode::new(hash, index));
        }
    }

    fn search(&self, query_hash: u64, threshold: u32) -> Vec<(usize, u32)> {
        let mut results = Vec::new();
        if let Some(ref root) = self.root {
            Self::search_node(root, query_hash, threshold, &mut results);
        }
        results
    }

    fn search_node(node: &BKNode, query_hash: u64, threshold: u32, results: &mut Vec<(usize, u32)>) {
        let dist = (node.hash ^ query_hash).count_ones();
        if dist <= threshold {
            results.push((node.index, dist));
        }

        let min_dist = if dist > threshold { dist - threshold } else { 0 };
        let max_dist = dist + threshold;

        for (&child_dist, child_node) in &node.children {
            if child_dist >= min_dist && child_dist <= max_dist {
                Self::search_node(child_node, query_hash, threshold, results);
            }
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
fn find_similar_images(db_path: String, config: ImageSimilarityConfig, allowed_roots: Option<Vec<String>>) -> Result<Vec<SimilarImageGroup>, String> {
    info!("查找相似图像");

    // 验证算法类型
    let valid_algorithms = ["ahash", "dhash", "phash"];
    if !valid_algorithms.contains(&config.algorithm.as_str()) {
        return Err(format!("无效的算法类型: {}，支持的算法: {:?}", config.algorithm, valid_algorithms));
    }

    // 验证阈值范围（0-64，因为是64位哈希）
    if config.threshold > 64 {
        return Err(format!("阈值必须在 0-64 之间，当前: {}", config.threshold));
    }

    let conn = init_database(&db_path)?;
    let roots = allowed_roots.unwrap_or_default();
    
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size, created_at, modified_at, file_extension
         FROM files 
         WHERE file_extension IN ('jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff')"
    ).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map([], |row| {
        Ok(FileInfo {
            id: row.get(0)?,
            path: row.get(1)?,
            filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            size: row.get::<_, i64>(3)? as u64,
            hash: None,
            hash_algorithm: "phash".to_string(),
            created_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            modified_at: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            file_extension: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .filter(|f| is_path_safe(&f.path, &roots))
    .collect();
    
    let mut file_hashes: Vec<(FileInfo, String)> = Vec::new();
    for file in files {
        if check_pause_with_condvar() {
            break;
        }

        let path = Path::new(&file.path);
        if let Ok(hash) = compute_image_perceptual_hash(path, &config.algorithm) {
            file_hashes.push((file, hash));
        }
    }
    
    // 构建 BK-Tree 索引
    let mut bk_tree = BKTree::new();
    for (idx, (_, hash_str)) in file_hashes.iter().enumerate() {
        let hash_val = u64::from_str_radix(hash_str, 16).unwrap_or(0);
        bk_tree.insert(hash_val, idx);
    }
    
    let mut groups: Vec<SimilarImageGroup> = Vec::new();
    let mut processed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    
    for i in 0..file_hashes.len() {
        if processed.contains(&i) {
            continue;
        }
        
        let (ref file1, ref hash1) = file_hashes[i];
        let hash_val1 = u64::from_str_radix(hash1, 16).unwrap_or(0);
        
        // 利用 BK-Tree 近邻剪枝查询，复杂度由 O(N) 降低到 O(log N)
        let matches = bk_tree.search(hash_val1, config.threshold);
        
        let mut similar_files = vec![file1.clone()];
        let mut hash_values = vec![hash1.clone()];
        
        for (idx, _) in matches {
            if idx == i || processed.contains(&idx) {
                continue;
            }
            let (ref file_match, ref hash_match) = file_hashes[idx];
            similar_files.push(file_match.clone());
            hash_values.push(hash_match.clone());
            processed.insert(idx);
        }

        if similar_files.len() > 1 {
            groups.push(SimilarImageGroup {
                similarity: 1.0 - (config.threshold as f64 / 64.0),
                files: similar_files,
                hash_values,
            });
        }
        
        processed.insert(i);
    }
    
    info!("找到 {} 个相似图像组", groups.len());
    Ok(groups)
}

#[tauri::command]
fn generate_thumbnail_command(
    path: String,
    max_width: u32,
    max_height: u32,
    allowed_roots: Option<Vec<String>>,
) -> Result<ThumbnailResult, String> {
    let roots = allowed_roots.unwrap_or_default();
    
    if !is_path_safe(&path, &roots) {
        return Err("路径不安全".to_string());
    }
    
    let path_obj = Path::new(&path);
    let (base64_data, width, height) = generate_thumbnail(path_obj, max_width, max_height)?;
    
    Ok(ThumbnailResult {
        path,
        thumbnail_base64: Some(base64_data),
        mime_type: "image/png".to_string(),
        width,
        height,
    })
}

#[tauri::command]
fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = Path::new(&path).parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn read_file_content(
    path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<String, String> {
    let roots = allowed_roots.unwrap_or_default();
    
    if !is_path_safe(&path, &roots) {
        return Err("路径不安全".to_string());
    }
    
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(
    path: String,
    content: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<(), String> {
    let roots = allowed_roots.unwrap_or_default();
    
    if !is_path_safe(&path, &roots) {
        return Err("路径不安全".to_string());
    }
    
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
fn export_duplicates(groups: Vec<DuplicateGroup>, format: String, export_path: String) -> Result<(), String> {
    info!("导出重复文件，格式: {}", format);

    // 检查导出路径
    if export_path.is_empty() {
        return Err("导出路径不能为空".to_string());
    }

    // 检查导出目录是否存在
    if let Some(parent) = Path::new(&export_path).parent() {
        if !parent.exists() {
            return Err(format!("导出目录不存在: {}", parent.display()));
        }
    }

    // 验证格式
    let valid_formats = ["csv", "md"];
    let format_lower = format.to_lowercase();
    if !valid_formats.contains(&format_lower.as_str()) {
        return Err(format!("不支持的格式: {}，支持的格式: {:?}", format, valid_formats));
    }

    match format_lower.as_str() {
        "csv" => export_duplicates_csv(&groups, &export_path),
        "md" => export_duplicates_markdown(&groups, &export_path),
        _ => Err(format!("不支持的格式: {}", format)),
    }
}

fn export_duplicates_csv(groups: &[DuplicateGroup], path: &str) -> Result<(), String> {
    let mut writer = csv::Writer::from_path(path).map_err(|e| e.to_string())?;
    
    writer.write_record(&["Hash", "File Path", "File Name", "Size", "Modified At"])
        .map_err(|e| e.to_string())?;
    
    for group in groups {
        for file in &group.files {
            writer.write_record(&[
                &group.hash,
                &file.path,
                &file.filename,
                &file.size.to_string(),
                &file.modified_at,
            ]).map_err(|e| e.to_string())?;
        }
    }
    
    writer.flush().map_err(|e| e.to_string())?;
    info!("CSV 导出完成: {}", path);
    Ok(())
}

fn export_duplicates_markdown(groups: &[DuplicateGroup], path: &str) -> Result<(), String> {
    let mut content = String::new();
    content.push_str("# 重复文件报告\n\n");
    content.push_str(&format!("生成时间: {}\n\n", Utc::now().to_rfc3339()));
    content.push_str(&format!("重复文件组数: {}\n\n", groups.len()));
    
    let total_wasted: u64 = groups.iter().map(|g| g.wasted_space).sum();
    content.push_str(&format!("可释放空间: {} bytes\n\n", total_wasted));
    
    for (i, group) in groups.iter().enumerate() {
        content.push_str(&format!("## 组 {} - Hash: {}\n\n", i + 1, group.hash));
        content.push_str(&format!("- 文件数: {}\n", group.files.len()));
        content.push_str(&format!("- 总大小: {} bytes\n", group.total_size));
        content.push_str(&format!("- 浪费空间: {} bytes\n\n", group.wasted_space));
        
        content.push_str("### 文件列表\n\n");
        for file in &group.files {
            content.push_str(&format!("- `{}` ({} bytes)\n", file.path, file.size));
        }
        content.push_str("\n");
    }
    
    std::fs::write(path, content).map_err(|e| e.to_string())?;
    info!("Markdown 导出完成: {}", path);
    Ok(())
}

#[tauri::command]
fn save_settings(request: SaveSettingsRequest) -> Result<(), String> {
    let conn = init_database(&request.db_path)?;
    let settings_json = serde_json::to_string(&request.settings).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)",
        ("app_settings", &settings_json, &Utc::now().to_rfc3339()),
    ).map_err(|e| e.to_string())?;
    
    info!("设置已保存");
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
fn load_settings(db_path: String) -> Result<AppSettings, String> {
    let conn = init_database(&db_path)?;
    
    let settings: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?",
        ["app_settings"],
        |row| row.get(0),
    );
    
    match settings {
        Ok(json) => {
            let settings: AppSettings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            Ok(settings)
        }
        Err(_) => Ok(AppSettings::default()),
    }
}

#[tauri::command]
fn start_file_watcher_command(
    paths: Vec<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    for path in &paths {
        if !Path::new(path).exists() {
            return Err(format!("路径不存在: {}", path));
        }
    }
    
    file_watcher::start_file_watcher(
        paths,
        Box::new(move |event: FileSystemEvent| {
            let _ = app_handle.emit("file-system-event", event);
        }),
    )
}

#[tauri::command]
fn stop_file_watcher_command() -> Result<(), String> {
    file_watcher::stop_file_watcher()
}

#[tauri::command]
fn get_file_watcher_status() -> FileWatcherStatus {
    let (is_running, watched_paths) = file_watcher::get_watcher_status();
    FileWatcherStatus {
        is_running,
        watched_paths,
    }
}

#[tauri::command]
fn add_watch_path_command(path: String) -> Result<(), String> {
    file_watcher::add_watch_path(path)
}

#[tauri::command]
fn remove_watch_path_command(path: String) -> Result<(), String> {
    file_watcher::remove_watch_path(&path)
}

#[tauri::command(rename_all = "snake_case")]
fn search_files(
    db_path: String,
    params: SearchParams,
) -> Result<SearchResult, String> {
    let conn = init_database(&db_path)?;
    
    let mut query = String::from(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
         FROM files WHERE 1=1"
    );
    let mut query_params: Vec<String> = Vec::new();
    
    if !params.query.is_empty() {
        query.push_str(" AND (filename LIKE ? OR path LIKE ?)");
        let pattern = format!("%{}%", params.query);
        query_params.push(pattern.clone());
        query_params.push(pattern);
    }
    
    if !params.file_extensions.is_empty() {
        let placeholders: Vec<String> = params.file_extensions.iter().map(|_| "?".to_string()).collect();
        query.push_str(&format!(" AND file_extension IN ({})", placeholders.join(",")));
        query_params.extend(params.file_extensions);
    }
    
    if let Some(min_size) = params.min_size {
        query.push_str(" AND size >= ?");
        query_params.push(min_size.to_string());
    }
    
    if let Some(max_size) = params.max_size {
        query.push_str(" AND size <= ?");
        query_params.push(max_size.to_string());
    }
    
    if let Some(start_date) = params.start_date {
        query.push_str(" AND modified_at >= ?");
        query_params.push(start_date);
    }
    
    if let Some(end_date) = params.end_date {
        query.push_str(" AND modified_at <= ?");
        query_params.push(end_date);
    }
    
    // 白名单验证排序字段，防止 SQL 注入
    let allowed_sort_columns = ["filename", "path", "size", "modified_at", "created_at", "file_extension"];
    let allowed_orders = ["ASC", "DESC", "asc", "desc"];
    
    let sort_by = if allowed_sort_columns.contains(&params.sort_by.as_str()) {
        &params.sort_by
    } else {
        "filename"
    };
    
    let sort_order = if allowed_orders.contains(&params.sort_order.as_str()) {
        params.sort_order.to_uppercase()
    } else {
        "ASC".to_string()
    };
    
    query.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));
    query.push_str(" LIMIT ? OFFSET ?");
    query_params.push(params.page_size.to_string());
    query_params.push((params.page * params.page_size).to_string());
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map(
        rusqlite::params_from_iter(query_params.iter()),
        |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    let count_query = query.split("LIMIT").next().unwrap_or("").to_string();
    let count_query = count_query.replace(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension",
        "SELECT COUNT(*)",
    );
    
    let total: i64 = conn.query_row(
        &count_query,
        rusqlite::params_from_iter(query_params[..query_params.len()-2].iter()),
        |row| row.get(0)
    ).unwrap_or(0);
    
    Ok(SearchResult {
        files,
        total: total as usize,
        page: params.page,
        page_size: params.page_size,
    })
}

// ========== 目录树功能 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub file_count: usize,
    pub children: Vec<DirectoryNode>,
}

#[tauri::command(rename_all = "snake_case")]
fn get_directory_tree(db_path: String, root_path: Option<String>) -> Result<Vec<DirectoryNode>, String> {
    info!("获取目录树, 数据库: {}, 根路径: {:?}", db_path, root_path);
    
    let conn = init_database(&db_path)?;
    
    // 构建目录树
    let mut root_nodes: Vec<DirectoryNode> = Vec::new();
    let mut path_to_node: HashMap<String, (usize, usize)> = HashMap::new(); // (root_index, node_index)
    
    // 查询所有文件路径
    let mut stmt = conn.prepare(
        "SELECT DISTINCT path FROM files ORDER BY path"
    ).map_err(|e| e.to_string())?;
    
    let paths: Vec<String> = stmt.query_map([], |row| {
        row.get::<_, String>(0)
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    for file_path in paths {
        // 过滤指定根路径
        if let Some(ref root) = root_path {
            if !file_path.starts_with(root) {
                continue;
            }
        }
        
        // 分割路径
        let path_parts: Vec<&str> = file_path.split(|c| c == '/' || c == '\\').filter(|s| !s.is_empty()).collect();
        
        let mut current_path = String::new();
        let mut parent_key = String::new();
        
        for (i, part) in path_parts.iter().enumerate() {
            // 构建当前路径
            if current_path.is_empty() {
                current_path = part.to_string();
            } else {
                current_path = format!("{}/ {}", current_path, part);
            }
            
            let is_file = i == path_parts.len() - 1;
            let node_key = current_path.clone();
            
            // 检查节点是否已存在
            if !path_to_node.contains_key(&node_key) {
                let node = DirectoryNode {
                    name: part.to_string(),
                    path: current_path.clone(),
                    is_directory: !is_file,
                    size: 0,
                    file_count: if is_file { 1 } else { 0 },
                    children: Vec::new(),
                };
                
                if parent_key.is_empty() {
                    // 根节点
                    path_to_node.insert(node_key.clone(), (root_nodes.len(), 0));
                    root_nodes.push(node);
                } else {
                    // 子节点
                    if let Some(&(root_idx, _)) = path_to_node.get(&parent_key) {
                        let parent = &mut root_nodes[root_idx];
                        let child_idx = parent.children.len();
                        path_to_node.insert(node_key.clone(), (root_idx, child_idx));
                        parent.children.push(node);
                    }
                }
            } else if is_file {
                // 文件已存在，增加计数
                if let Some(&(root_idx, _)) = path_to_node.get(&node_key) {
                    root_nodes[root_idx].file_count += 1;
                }
            }
            
            parent_key = node_key;
        }
    }
    
    info!("目录树构建完成, 共 {} 个根节点", root_nodes.len());
    Ok(root_nodes)
}

#[tauri::command(rename_all = "snake_case")]
fn get_files_in_directory(
    db_path: String,
    directory_path: String,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<(Vec<FileInfo>, usize), String> {
    info!("获取目录内文件: {}, 目录: {}", db_path, directory_path);
    
    let conn = init_database(&db_path)?;
    
    let page = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(100);
    let offset = page * page_size;
    
    // 查询目录内的文件
    let mut query = String::from(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
         FROM files 
         WHERE path LIKE ? || '%'"
    );
    
    // 排除子目录中的文件（只获取当前目录的直接文件）
    query.push_str(" AND path NOT LIKE ? || '%/%' AND path NOT LIKE ? || '%\\%'");
    
    query.push_str(" ORDER BY filename LIMIT ? OFFSET ?");
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map(
        rusqlite::params![&directory_path, &directory_path, &directory_path, page_size as i64, offset as i64],
        |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 查询总数
    let count_query = "SELECT COUNT(*) FROM files WHERE path LIKE ? || '%' AND path NOT LIKE ? || '%/%' AND path NOT LIKE ? || '%\\%'";
    let total: i64 = conn.query_row(
        count_query,
        rusqlite::params![&directory_path, &directory_path, &directory_path],
        |row| row.get(0)
    ).unwrap_or(0);
    
    info!("找到 {} 个文件, 总数: {}", files.len(), total);
    Ok((files, total as usize))
}

#[tauri::command(rename_all = "snake_case")]
fn get_all_files(
    db_path: String,
) -> Result<Vec<FileInfo>, String> {
    info!("获取所有文件: {}", db_path);
    
    let conn = init_database(&db_path)?;
    
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
         FROM files 
         ORDER BY path, filename"
    ).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map(
        [],
        |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    info!("找到 {} 个文件", files.len());
    Ok(files)
}

#[tauri::command(rename_all = "snake_case")]
fn search_in_directory(
    db_path: String,
    directory_path: String,
    query: String,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<(Vec<FileInfo>, usize), String> {
    info!("在目录中搜索: {}, 目录: {}, 查询: {}", db_path, directory_path, query);
    
    let conn = init_database(&db_path)?;
    
    let page = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(100);
    let offset = page * page_size;
    
    let pattern = format!("%{}%", query);
    
    // 查询目录内匹配的文件
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
         FROM files 
         WHERE path LIKE ? || '%' 
         AND (filename LIKE ? OR path LIKE ?)
         ORDER BY filename LIMIT ? OFFSET ?"
    ).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map(
        rusqlite::params![&directory_path, &pattern, &pattern, page_size as i64, offset as i64],
        |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 查询总数
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE path LIKE ? || '%' AND (filename LIKE ? OR path LIKE ?)",
        rusqlite::params![&directory_path, &pattern, &pattern],
        |row| row.get(0)
    ).unwrap_or(0);
    
    info!("搜索完成, 找到 {} 个文件, 总数: {}", files.len(), total);
    Ok((files, total as usize))
}

// ========== 图片打包功能 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDirectoryInfo {
    pub directory_path: String,
    pub image_count: usize,
    pub total_size: u64,
    pub image_files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageArchiveReport {
    pub directories: Vec<ImageDirectoryInfo>,
    pub total_directories: usize,
    pub total_images: usize,
    pub total_size: u64,
}

#[tauri::command(rename_all = "snake_case")]
fn analyze_image_directories(
    db_path: String,
    min_image_count: Option<usize>,
    image_extensions: Option<Vec<String>>,
) -> Result<ImageArchiveReport, String> {
    let min_count = min_image_count.unwrap_or(10);
    let extensions = image_extensions.unwrap_or_else(|| {
        vec!["jpg".to_string(), "jpeg".to_string(), "png".to_string(), "gif".to_string(), "bmp".to_string(), "webp".to_string(), "svg".to_string()]
    });
    
    info!("分析图片密集目录, 数据库: {}, 最小图片数: {}", db_path, min_count);
    
    let conn = init_database(&db_path)?;
    
    // 查询所有图片文件
    let placeholders: Vec<String> = extensions.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
         FROM files 
         WHERE file_extension IN ({})
         ORDER BY path",
        placeholders.join(",")
    );
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let files: Vec<FileInfo> = stmt.query_map(
        rusqlite::params_from_iter(extensions.iter()),
        |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 按目录分组
    let mut dir_map: HashMap<String, Vec<FileInfo>> = HashMap::new();
    for file in files {
        let dir_path = Path::new(&file.path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        
        dir_map.entry(dir_path).or_default().push(file);
    }
    
    // 筛选出图片数量超过阈值的目录
    let mut directories: Vec<ImageDirectoryInfo> = dir_map
        .into_iter()
        .filter(|(_, files)| files.len() >= min_count)
        .map(|(dir_path, files)| {
            let total_size: u64 = files.iter().map(|f| f.size).sum();
            ImageDirectoryInfo {
                directory_path: dir_path,
                image_count: files.len(),
                total_size,
                image_files: files,
            }
        })
        .collect();
    
    // 按图片数量排序
    directories.sort_by(|a, b| b.image_count.cmp(&a.image_count));
    
    let total_images: usize = directories.iter().map(|d| d.image_count).sum();
    let total_size: u64 = directories.iter().map(|d| d.total_size).sum();
    
    info!("找到 {} 个图片密集目录, 共 {} 张图片", directories.len(), total_images);
    
    Ok(ImageArchiveReport {
        total_directories: directories.len(),
        total_images,
        total_size,
        directories,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveResult {
    pub archive_path: String,
    pub file_count: usize,
    pub total_size: u64,
    pub compression_ratio: f64,
}

#[tauri::command(rename_all = "snake_case")]
async fn create_archive(
    db_path: String,
    source_directory: String,
    output_path: String,
    archive_format: String, // "zip" 或 "rar"
    compression_level: Option<u32>,
) -> Result<ArchiveResult, String> {
    use std::io::Write;
    
    info!("创建压缩文件: {} -> {}, 格式: {}", source_directory, output_path, archive_format);
    
    let compression = compression_level.unwrap_or(6);
    let format = archive_format.to_lowercase();
    
    // 收集要压缩的文件
    let files_to_archive: Vec<FileInfo> = {
        let conn = init_database(&db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension 
             FROM files 
             WHERE path LIKE ? || '%'
             ORDER BY path"
        ).map_err(|e| e.to_string())?;
        
        let files: Vec<FileInfo> = stmt.query_map([&source_directory], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                size: row.get::<_, i64>(3)? as u64,
                hash: row.get(4)?,
                hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
                created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        files
    };
    
    if files_to_archive.is_empty() {
        return Err("没有找到要压缩的文件".to_string());
    }
    
    let total_size: u64 = files_to_archive.iter().map(|f| f.size).sum();
    let file_count = files_to_archive.len();
    
    // 创建压缩文件
    if format == "zip" {
        // 使用 zip crate 创建 ZIP 文件
        let file = std::fs::File::create(&output_path).map_err(|e| format!("创建文件失败: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(compression as i64));
        
        for file_info in &files_to_archive {
            let file_path = Path::new(&file_info.path);
            if file_path.exists() {
                let file_name = file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                
                zip.start_file(&file_name, options).map_err(|e| format!("添加文件失败: {}", e))?;
                
                let mut f = std::fs::File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).map_err(|e| format!("读取文件失败: {}", e))?;
                zip.write_all(&buffer).map_err(|e| format!("写入文件失败: {}", e))?;
            }
        }
        
        zip.finish().map_err(|e| format!("完成压缩失败: {}", e))?;
    } else {
        return Err(format!("不支持的压缩格式: {}", format));
    }
    
    // 计算压缩率
    let archive_size = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
    let compression_ratio = if total_size > 0 {
        (1.0 - (archive_size as f64 / total_size as f64)) * 100.0
    } else {
        0.0
    };
    
    info!("压缩完成: {} 个文件, 原始大小: {}, 压缩后: {}, 压缩率: {:.1}%", 
          file_count, total_size, archive_size, compression_ratio);
    
    Ok(ArchiveResult {
        archive_path: output_path,
        file_count,
        total_size,
        compression_ratio,
    })
}

#[tauri::command(rename_all = "snake_case")]
fn delete_files_after_archive(
    db_path: String,
    directory_path: String,
    image_extensions: Vec<String>,
) -> Result<usize, String> {
    info!("删除已打包的图片文件: {}, 目录: {}", db_path, directory_path);
    
    let conn = init_database(&db_path)?;
    
    // 查询要删除的文件
    let placeholders: Vec<String> = image_extensions.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        "SELECT path FROM files WHERE path LIKE ? || '%' AND file_extension IN ({})",
        placeholders.join(",")
    );
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let paths_to_delete: Vec<String> = stmt.query_map(
        rusqlite::params_from_iter(std::iter::once(&directory_path).chain(image_extensions.iter())),
        |row| row.get::<_, String>(0)
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    let mut deleted_count = 0;
    let mut failed_paths: Vec<String> = Vec::new();
    
    // 删除文件系统中的文件
    for path in &paths_to_delete {
        match std::fs::remove_file(path) {
            Ok(_) => {
                deleted_count += 1;
            }
            Err(e) => {
                warn!("删除文件失败 {}: {}", path, e);
                failed_paths.push(path.clone());
            }
        }
    }
    
    // 从数据库中删除记录
    if deleted_count > 0 {
        let delete_query = format!(
            "DELETE FROM files WHERE path LIKE ? || '%' AND file_extension IN ({})",
            placeholders.join(",")
        );
        
        let _ = conn.execute(
            &delete_query,
            rusqlite::params_from_iter(std::iter::once(&directory_path).chain(image_extensions.iter()))
        );
    }
    
    info!("删除完成: 成功 {} 个, 失败 {} 个", deleted_count, failed_paths.len());
    
    if !failed_paths.is_empty() {
        return Err(format!("部分文件删除失败: {:?}", failed_paths));
    }
    
    Ok(deleted_count)
}

// ========== 分析功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn analyze_duplicate_distribution(
    _db_path: String,
    groups: Vec<DuplicateGroup>,
) -> Result<DuplicateDistribution, String> {
    info!("分析重复文件分布");
    
    // 按文件类型统计
    let mut type_stats: HashMap<String, (i32, i32, u64)> = HashMap::new();
    for group in &groups {
        for file in &group.files {
            let ext = file.file_extension.clone();
            let entry = type_stats.entry(ext).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.2 += file.size;
        }
        // 每个组只计一次组数
        if let Some(first) = group.files.first() {
            let ext = first.file_extension.clone();
            if let Some(entry) = type_stats.get_mut(&ext) {
                entry.1 += 1;
            }
        }
    }
    
    let total_files: i32 = type_stats.values().map(|(c, _, _)| c).sum();
    
    let mut by_type: Vec<FileTypeGroup> = type_stats
        .into_iter()
        .map(|(ext, (count, groups, size))| FileTypeGroup {
            extension: ext,
            file_count: count,
            group_count: groups,
            total_size: size,
            percentage: if total_files > 0 {
                (count as f64 / total_files as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    
    by_type.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    
    // 按大小范围统计
    let size_ranges = [
        (0, 1024, "0-1KB"),
        (1024, 1024 * 1024, "1KB-1MB"),
        (1024 * 1024, 10 * 1024 * 1024, "1MB-10MB"),
        (10 * 1024 * 1024, 100 * 1024 * 1024, "10MB-100MB"),
        (100 * 1024 * 1024, 1024 * 1024 * 1024, "100MB-1GB"),
        (1024 * 1024 * 1024, u64::MAX, ">1GB"),
    ];
    
    let mut by_size: Vec<SizeRangeGroup> = Vec::new();
    for (min, max, label) in &size_ranges {
        let mut file_count = 0;
        let mut total_size = 0u64;
        let mut group_count = 0;
        
        for group in &groups {
            let mut group_matched = false;
            for file in &group.files {
                if file.size >= *min && file.size < *max {
                    file_count += 1;
                    total_size += file.size;
                    group_matched = true;
                }
            }
            if group_matched {
                group_count += 1;
            }
        }
        
        by_size.push(SizeRangeGroup {
            range: label.to_string(),
            min_size: *min,
            max_size: *max,
            file_count,
            total_size,
            group_count,
        });
    }
    
    // 按目录统计
    let mut dir_stats: HashMap<String, (i32, i32, u64)> = HashMap::new();
    for group in &groups {
        for file in &group.files {
            if let Some(parent) = Path::new(&file.path).parent() {
                let dir = parent.to_string_lossy().to_string();
                let entry = dir_stats.entry(dir).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.2 += file.size;
            }
        }
    }
    
    let mut by_directory: Vec<DirectoryGroup> = dir_stats
        .into_iter()
        .map(|(dir, (count, groups, size))| DirectoryGroup {
            directory: dir,
            file_count: count,
            group_count: groups,
            total_size: size,
        })
        .collect();
    
    by_directory.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    by_directory.truncate(20); // 只保留前20个目录
    
    // 计算摘要
    let total_groups = groups.len() as i64;
    let total_wasted: u64 = groups.iter().map(|g| g.wasted_space).sum();
    let avg_group_size = if !groups.is_empty() {
        groups.iter().map(|g| g.files.len() as f64).sum::<f64>() / groups.len() as f64
    } else {
        0.0
    };
    
    Ok(DuplicateDistribution {
        by_size,
        by_type,
        by_directory,
        summary: DistributionSummary {
            total_files: total_files as i64,
            total_groups,
            total_wasted_space: total_wasted,
            average_group_size: avg_group_size,
        },
    })
}

// ========== 合规检查功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn generate_compliance_report(
    db_path: String,
    report_type: String,
) -> Result<ComplianceReport, String> {
    info!("生成合规报告: {}", report_type);
    
    let _conn = init_database(&db_path)?;
    let classification = find_duplicates(db_path.clone())?;
    
    // 合并所有重复组
    let all_groups: Vec<_> = classification.complete_duplicates.iter()
        .chain(classification.name_duplicates.iter())
        .chain(classification.content_duplicates.iter())
        .collect();
    
    let total_files: usize = all_groups.iter().map(|g| g.files.len()).sum();
    let total_duplicates = all_groups.len();
    let total_space: u64 = all_groups.iter().map(|g| g.total_size).sum();
    
    // 计算合规评分
    let compliance_score = if total_files > 0 {
        let duplicate_ratio = total_duplicates as f64 / total_files as f64;
        (1.0 - duplicate_ratio.min(1.0)) * 100.0
    } else {
        100.0
    };
    
    let risk_level = if compliance_score >= 90.0 {
        "低"
    } else if compliance_score >= 70.0 {
        "中"
    } else {
        "高"
    };
    
    // 生成发现项
    let mut findings = Vec::new();
    
    if total_duplicates > 100 {
        findings.push(ComplianceFinding {
            finding_id: "F001".to_string(),
            category: "Volume".to_string(),
            severity: "高".to_string(),
            description: format!("发现大量重复文件: {} 组", total_duplicates),
            affected_files: all_groups.iter().take(10).flat_map(|g| {
                g.files.iter().map(|f| f.path.clone())
            }).collect(),
            remediation: "建议立即清理重复文件以释放存储空间".to_string(),
        });
    }
    
    // 检查大文件重复
    let large_duplicates: Vec<_> = all_groups.iter()
        .filter(|g| g.total_size > 100 * 1024 * 1024)
        .collect();
    
    if !large_duplicates.is_empty() {
        findings.push(ComplianceFinding {
            finding_id: "F002".to_string(),
            category: "Large Files".to_string(),
            severity: "中".to_string(),
            description: format!("发现 {} 组大文件重复", large_duplicates.len()),
            affected_files: large_duplicates.iter().take(5).flat_map(|g| {
                g.files.iter().map(|f| f.path.clone())
            }).collect(),
            remediation: "优先清理大文件重复以最大化空间回收".to_string(),
        });
    }
    
    let recommendations = vec![
        "定期执行重复文件扫描".to_string(),
        "建立文件命名规范避免重复".to_string(),
        "使用版本控制系统管理文档".to_string(),
        "配置自动清理策略".to_string(),
    ];
    
    Ok(ComplianceReport {
        report_id: format!("RPT-{}", Utc::now().timestamp()),
        report_type,
        generated_at: Utc::now().to_rfc3339(),
        summary: ComplianceSummary {
            total_files_reviewed: total_files,
            total_duplicates_found: total_duplicates,
            total_space_occupied: total_space,
            risk_level: risk_level.to_string(),
            compliance_score,
        },
        findings,
        recommendations,
    })
}

// ========== 多维度分析报告 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDimensionalSummary {
    pub total_files_analyzed: i64,
    pub total_duplicate_groups: i64,
    pub total_duplicate_files: i64,
    pub total_wasted_space: i64,
    pub top_insight: String,
    pub recommendations: Vec<String>,
    pub compliance_score: Option<f64>,
    pub risk_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeDetail {
    pub extension: String,
    pub file_count: i64,
    pub duplicate_count: i64,
    pub total_size: i64,
    pub wasted_space: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRangeDetail {
    pub range_label: String,
    pub min_bytes: i64,
    pub max_bytes: i64,
    pub file_count: i64,
    pub group_count: i64,
    pub total_size: i64,
    pub wasted_space: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFileInfo {
    pub hash: String,
    pub size: i64,
    pub file_count: i64,
    pub locations: Vec<String>,
    pub potential_savings: i64,
    pub modified_at: String,
    pub filenames: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryStats {
    pub directory: String,
    pub duplicate_file_count: i64,
    pub duplicate_group_count: i64,
    pub wasted_space: i64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAnalysisReport {
    pub directory_tree: Vec<serde_json::Value>,
    pub top_duplicate_directories: Vec<DirectoryStats>,
    pub path_depth_distribution: Vec<serde_json::Value>,
    pub cross_directory_duplicates: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeAnalysisReport {
    pub type_distribution: Vec<FileTypeDetail>,
    pub top_duplicate_extensions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeAnalysisReport {
    pub size_ranges: Vec<SizeRangeDetail>,
    pub large_duplicate_files: Vec<LargeFileInfo>,
    pub size_efficiency_metrics: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartVisualizationData {
    pub pie_charts: serde_json::Value,
    pub bar_charts: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDimensionalReport {
    pub by_path: PathAnalysisReport,
    pub by_type: FileTypeAnalysisReport,
    pub by_size: SizeAnalysisReport,
    pub by_owner: serde_json::Value,
    pub charts: ChartVisualizationData,
    pub summary: MultiDimensionalSummary,
    pub generated_at: String,
}

#[tauri::command(rename_all = "snake_case")]
fn generate_multi_dimensional_report(db_path: String) -> Result<MultiDimensionalReport, String> {
    info!("生成多维度分析报告");
    
    let _conn = init_database(&db_path)?;

    // 获取所有重复文件组
    let classification = find_duplicates(db_path)?;
    
    // 合并所有重复组
    let all_groups: Vec<_> = classification.complete_duplicates.iter()
        .chain(classification.name_duplicates.iter())
        .chain(classification.content_duplicates.iter())
        .collect();
    
    let total_groups = all_groups.len() as i64;
    let total_duplicate_files: i64 = all_groups.iter().map(|g| g.files.len() as i64).sum();
    let total_wasted_space: i64 = all_groups.iter().map(|g| g.wasted_space as i64).sum();
    
    // 分析文件类型分布
    let mut type_map: HashMap<String, (i64, i64, i64, i64)> = HashMap::new();
    let mut dir_map: HashMap<String, (i64, i64, i64)> = HashMap::new();
    let mut size_ranges: Vec<SizeRangeDetail> = vec![
        SizeRangeDetail {
            range_label: "< 1KB".to_string(),
            min_bytes: 0,
            max_bytes: 1024,
            file_count: 0,
            group_count: 0,
            total_size: 0,
            wasted_space: 0,
            percentage: 0.0,
        },
        SizeRangeDetail {
            range_label: "1KB - 1MB".to_string(),
            min_bytes: 1024,
            max_bytes: 1024 * 1024,
            file_count: 0,
            group_count: 0,
            total_size: 0,
            wasted_space: 0,
            percentage: 0.0,
        },
        SizeRangeDetail {
            range_label: "1MB - 100MB".to_string(),
            min_bytes: 1024 * 1024,
            max_bytes: 100 * 1024 * 1024,
            file_count: 0,
            group_count: 0,
            total_size: 0,
            wasted_space: 0,
            percentage: 0.0,
        },
        SizeRangeDetail {
            range_label: "100MB - 1GB".to_string(),
            min_bytes: 100 * 1024 * 1024,
            max_bytes: 1024 * 1024 * 1024,
            file_count: 0,
            group_count: 0,
            total_size: 0,
            wasted_space: 0,
            percentage: 0.0,
        },
        SizeRangeDetail {
            range_label: "> 1GB".to_string(),
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: i64::MAX,
            file_count: 0,
            group_count: 0,
            total_size: 0,
            wasted_space: 0,
            percentage: 0.0,
        },
    ];
    
    let mut large_files: Vec<LargeFileInfo> = Vec::new();
    
    for group in &all_groups {
        let file_count = group.files.len() as i64;
        let size = group.total_size as i64;
        let wasted = group.wasted_space as i64;
        
        // 按文件类型统计
        for file in &group.files {
            let ext = file.file_extension.clone();
            let entry = type_map.entry(ext).or_insert((0, 0, 0, 0));
            entry.0 += 1; // file_count
            entry.2 += size; // total_size
            entry.3 += wasted; // wasted_space
        }
        
        // 更新重复计数
        if let Some(first_file) = group.files.first() {
            let ext = first_file.file_extension.clone();
            if let Some(entry) = type_map.get_mut(&ext) {
                entry.1 += file_count; // duplicate_count
            }
        }
        
        // 按目录统计
        for file in &group.files {
            if let Some(parent) = Path::new(&file.path).parent() {
                let dir = parent.to_string_lossy().to_string();
                let entry = dir_map.entry(dir).or_insert((0, 0, 0));
                entry.0 += 1; // duplicate_file_count
                entry.2 += wasted; // wasted_space
            }
        }
        
        // 更新目录的组计数
        if let Some(first_file) = group.files.first() {
            if let Some(parent) = Path::new(&first_file.path).parent() {
                let dir = parent.to_string_lossy().to_string();
                if let Some(entry) = dir_map.get_mut(&dir) {
                    entry.1 += 1; // duplicate_group_count
                }
            }
        }
        
        // 按大小范围统计
        for range in &mut size_ranges {
            if size >= range.min_bytes && size < range.max_bytes {
                range.file_count += file_count;
                range.group_count += 1;
                range.total_size += size * file_count;
                range.wasted_space += wasted;
                break;
            }
        }
        
        // 收集重复文件（由下方自然排序并截取 TOP10）
        if size > 0 {
            let locations: Vec<String> = group.files.iter().map(|f| f.path.clone()).collect();
            let filenames: Vec<String> = group.files.iter().map(|f| f.filename.clone()).collect();
            // 获取最新的修改日期
            let latest_modified = group.files.iter()
                .map(|f| &f.modified_at)
                .max()
                .cloned()
                .unwrap_or_default();
            large_files.push(LargeFileInfo {
                hash: group.hash.clone(),
                size,
                file_count,
                locations: locations.clone(),
                potential_savings: wasted,
                modified_at: latest_modified,
                filenames: filenames.clone(),
            });
        }
    }
    
    // 计算百分比
    let total_wasted = total_wasted_space as f64;
    for range in &mut size_ranges {
        if total_wasted > 0.0 {
            range.percentage = (range.wasted_space as f64 / total_wasted) * 100.0;
        }
    }
    
    // 转换为文件类型详情
    let mut type_distribution: Vec<FileTypeDetail> = type_map
        .into_iter()
        .map(|(ext, (count, dup_count, total, wasted))| {
            let percentage = if total_wasted > 0.0 {
                (wasted as f64 / total_wasted) * 100.0
            } else {
                0.0
            };
            FileTypeDetail {
                extension: ext,
                file_count: count,
                duplicate_count: dup_count,
                total_size: total,
                wasted_space: wasted,
                percentage,
            }
        })
        .collect();
    
    // 按浪费空间排序
    type_distribution.sort_by(|a, b| b.wasted_space.cmp(&a.wasted_space));
    
    // 转换为目录统计
    let mut top_directories: Vec<DirectoryStats> = dir_map
        .into_iter()
        .map(|(dir, (file_count, group_count, wasted))| {
            let percentage = if total_wasted > 0.0 {
                (wasted as f64 / total_wasted) * 100.0
            } else {
                0.0
            };
            DirectoryStats {
                directory: dir,
                duplicate_file_count: file_count,
                duplicate_group_count: group_count,
                wasted_space: wasted,
                percentage_of_total: percentage,
            }
        })
        .collect();
    
    top_directories.sort_by(|a, b| b.wasted_space.cmp(&a.wasted_space));
    top_directories.truncate(10);
    
    // 大文件排序
    large_files.sort_by(|a, b| b.size.cmp(&a.size));
    large_files.truncate(10);
    
    // 计算合规评分
    let compliance_score = if total_duplicate_files > 0 {
        let ratio = total_groups as f64 / total_duplicate_files as f64;
        (1.0 - ratio.min(1.0)) * 100.0
    } else {
        100.0
    };
    
    let risk_level = if compliance_score >= 90.0 {
        "低"
    } else if compliance_score >= 70.0 {
        "中"
    } else {
        "高"
    };

    // 生成洞察和建议
    let top_insight = if let Some(top_type) = type_distribution.first() {
        let size_str = if top_type.wasted_space < 1024 {
            format!("{} B", top_type.wasted_space)
        } else if top_type.wasted_space < 1024 * 1024 {
            format!("{:.2} KB", top_type.wasted_space as f64 / 1024.0)
        } else if top_type.wasted_space < 1024 * 1024 * 1024 {
            format!("{:.2} MB", top_type.wasted_space as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", top_type.wasted_space as f64 / (1024.0 * 1024.0 * 1024.0))
        };
        format!(
            "{} 类型文件占用最多重复空间，共 {} 个文件，浪费 {} 空间",
            if top_type.extension.is_empty() { "无扩展名" } else { &top_type.extension },
            top_type.duplicate_count,
            size_str
        )
    } else {
        "暂无重复文件数据".to_string()
    };
    
    let first_wasted = type_distribution.first().map(|t| t.wasted_space).unwrap_or(0);
    let first_wasted_str = if first_wasted < 1024 {
        format!("{} B", first_wasted)
    } else if first_wasted < 1024 * 1024 {
        format!("{:.2} KB", first_wasted as f64 / 1024.0)
    } else if first_wasted < 1024 * 1024 * 1024 {
        format!("{:.2} MB", first_wasted as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", first_wasted as f64 / (1024.0 * 1024.0 * 1024.0))
    };
    
    let recommendations = vec![
        format!("优先清理 {} 类型文件可释放 {} 空间", 
            type_distribution.first().map(|t| if t.extension.is_empty() { "无扩展名".to_string() } else { t.extension.clone() }).unwrap_or_default(),
            first_wasted_str
        ),
        "定期执行重复文件扫描以保持系统整洁".to_string(),
        "考虑使用增量扫描减少重复检测时间".to_string(),
        format!("当前风险等级为 {}，建议 {} 处理",
            risk_level,
            if risk_level == "高" { "立即" } else { "适时" }
        ),
    ];
    
    let summary = MultiDimensionalSummary {
        total_files_analyzed: total_duplicate_files,
        total_duplicate_groups: total_groups,
        total_duplicate_files,
        total_wasted_space,
        top_insight,
        recommendations,
        compliance_score: Some(compliance_score),
        risk_level: Some(risk_level.to_string()),
    };
    
    let by_type = FileTypeAnalysisReport {
        type_distribution: type_distribution.clone(),
        top_duplicate_extensions: type_distribution.iter().take(5).map(|t| {
            serde_json::json!({
                "extension": t.extension.clone(),
                "count": t.duplicate_count,
                "wasted_space": t.wasted_space
            })
        }).collect(),
    };
    
    let by_size = SizeAnalysisReport {
        size_ranges,
        large_duplicate_files: large_files,
        size_efficiency_metrics: serde_json::json!({
            "total_analyzed": total_duplicate_files,
            "efficiency_score": compliance_score
        }),
    };
    
    let by_path = PathAnalysisReport {
        directory_tree: vec![],
        top_duplicate_directories: top_directories,
        path_depth_distribution: vec![],
        cross_directory_duplicates: vec![],
    };
    
    let charts = ChartVisualizationData {
        pie_charts: serde_json::json!({
            "type_distribution": type_distribution.iter().take(8).map(|t| {
                serde_json::json!({
                    "label": if t.extension.is_empty() { "无扩展名" } else { &t.extension },
                    "value": t.file_count
                })
            }).collect::<Vec<_>>()
        }),
        bar_charts: serde_json::json!({
            "size_ranges": vec!["<1KB", "1KB-1MB", "1MB-100MB", "100MB-1GB", ">1GB"]
        }),
    };
    
    Ok(MultiDimensionalReport {
        by_path,
        by_type,
        by_size,
        by_owner: serde_json::json!({}),
        charts,
        summary,
        generated_at: Utc::now().to_rfc3339(),
    })
}

// ========== 日志审计功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn query_operation_logs(
    db_path: String,
    params: LogQueryParams,
) -> Result<LogQueryResult, String> {
    let conn = init_database(&db_path)?;
    
    let mut query = String::from(
        "SELECT id, timestamp, operation_type, target_files, source_path, 
                destination_path, status, message, user
         FROM operation_logs WHERE 1=1"
    );
    let mut query_params: Vec<String> = Vec::new();
    
    if let Some(start) = params.start_time {
        query.push_str(" AND timestamp >= ?");
        query_params.push(start);
    }
    
    if let Some(end) = params.end_time {
        query.push_str(" AND timestamp <= ?");
        query_params.push(end);
    }
    
    if let Some(op_type) = params.operation_type {
        query.push_str(" AND operation_type = ?");
        query_params.push(op_type);
    }
    
    if let Some(status) = params.status {
        query.push_str(" AND status = ?");
        query_params.push(status);
    }
    
    query.push_str(" ORDER BY timestamp DESC");
    query.push_str(" LIMIT ? OFFSET ?");
    query_params.push(params.page_size.to_string());
    query_params.push((params.page * params.page_size).to_string());
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let logs: Vec<OperationLog> = stmt.query_map(
        rusqlite::params_from_iter(query_params.iter()),
        |row| {
            let target_files_str: String = row.get(3)?;
            let target_files: Vec<String> = target_files_str
                .split(',')
                .map(|s| s.to_string())
                .collect();
            
            Ok(OperationLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                operation_type: row.get(2)?,
                target_files,
                source_path: row.get(4)?,
                destination_path: row.get(5)?,
                status: row.get(6)?,
                message: row.get(7)?,
                user: row.get(8)?,
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 获取总数
    let count_query = query.split("LIMIT").next().unwrap_or("").to_string();
    let count_query = count_query.replace(
        "SELECT id, timestamp, operation_type, target_files, source_path, destination_path, status, message, user",
        "SELECT COUNT(*)",
    );
    
    let total: i64 = conn.query_row(
        &count_query,
        rusqlite::params_from_iter(query_params[..query_params.len()-2].iter()),
        |row| row.get(0)
    ).unwrap_or(0);
    
    Ok(LogQueryResult {
        logs,
        total: total as usize,
        page: params.page,
        page_size: params.page_size,
    })
}

#[tauri::command(rename_all = "snake_case")]
fn query_audit_logs(
    db_path: String,
    params: AuditQueryParams,
) -> Result<AuditQueryResult, String> {
    let conn = init_database(&db_path)?;
    
    let mut query = String::from(
        "SELECT id, timestamp, event_type, severity, user, resource, action, details, result
         FROM audit_logs WHERE 1=1"
    );
    let mut query_params: Vec<String> = Vec::new();
    
    if let Some(start) = params.start_time {
        query.push_str(" AND timestamp >= ?");
        query_params.push(start);
    }
    
    if let Some(end) = params.end_time {
        query.push_str(" AND timestamp <= ?");
        query_params.push(end);
    }
    
    if let Some(event_type) = params.event_type {
        query.push_str(" AND event_type = ?");
        query_params.push(event_type);
    }
    
    if let Some(severity) = params.severity {
        query.push_str(" AND severity = ?");
        query_params.push(severity);
    }
    
    query.push_str(" ORDER BY timestamp DESC");
    query.push_str(" LIMIT ? OFFSET ?");
    query_params.push(params.page_size.to_string());
    query_params.push((params.page * params.page_size).to_string());
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    let events: Vec<AuditLog> = stmt.query_map(
        rusqlite::params_from_iter(query_params.iter()),
        |row| {
            Ok(AuditLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                event_type: row.get(2)?,
                severity: row.get(3)?,
                user: row.get(4)?,
                resource: row.get(5)?,
                action: row.get(6)?,
                details: row.get(7)?,
                result: row.get(8)?,
            })
        }
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 获取总数
    let count_query = query.split("LIMIT").next().unwrap_or("").to_string();
    let count_query = count_query.replace(
        "SELECT id, timestamp, event_type, severity, user, resource, action, details, result",
        "SELECT COUNT(*)",
    );
    
    let total: i64 = conn.query_row(
        &count_query,
        rusqlite::params_from_iter(query_params[..query_params.len()-2].iter()),
        |row| row.get(0)
    ).unwrap_or(0);
    
    Ok(AuditQueryResult {
        events,
        total: total as usize,
        page: params.page,
        page_size: params.page_size,
    })
}

// ========== 扫描历史功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn get_scan_history_list(
    db_path: String,
    params: Option<ScanHistoryQueryParams>,
) -> Result<ScanHistoryResult, String> {
    info!("获取扫描历史列表");

    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    let conn = init_database(&db_path)?;
    let params = params.unwrap_or_default();

    // 构建查询条件
    let mut conditions: Vec<String> = Vec::new();
    let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(start_date) = &params.start_date {
        conditions.push("scan_time >= ?".to_string());
        query_params.push(Box::new(start_date.clone()));
    }

    if let Some(end_date) = &params.end_date {
        conditions.push("scan_time <= ?".to_string());
        query_params.push(Box::new(end_date.clone()));
    }

    if let Some(status) = &params.status {
        conditions.push("status = ?".to_string());
        query_params.push(Box::new(status.clone()));
    }

    if let Some(min_files) = params.min_files {
        conditions.push("total_files >= ?".to_string());
        query_params.push(Box::new(min_files));
    }

    if let Some(max_files) = params.max_files {
        conditions.push("total_files <= ?".to_string());
        query_params.push(Box::new(max_files));
    }

    // 搜索关键词（搜索目录路径）
    if let Some(keyword) = &params.search_keyword {
        if !keyword.is_empty() {
            conditions.push("directories LIKE ?".to_string());
            query_params.push(Box::new(format!("%{}%", keyword)));
        }
    }

    // 构建 WHERE 子句
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 排序
    let sort_by = params.sort_by.as_deref().unwrap_or("scan_time");
    let sort_order = params.sort_order.as_deref().unwrap_or("DESC");

    // 验证排序字段，防止 SQL 注入
    let valid_sort_fields = ["scan_time", "total_files", "duplicate_files", "wasted_space", "duration_seconds"];
    let sort_by = if valid_sort_fields.contains(&sort_by) {
        sort_by
    } else {
        "scan_time"
    };
    let sort_order = if sort_order.eq_ignore_ascii_case("ASC") {
        "ASC"
    } else {
        "DESC"
    };

    // 分页
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // 查询数据
    let query = format!(
        "SELECT id, scan_time, directories, total_files, duplicate_groups,
                duplicate_files, wasted_space, duration_seconds, status
         FROM scan_history
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_clause, sort_by, sort_order
    );

    query_params.push(Box::new(page_size));
    query_params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = query_params
        .iter()
        .map(|p| p.as_ref())
        .collect();

    let history: Vec<ScanHistoryItem> = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
            let directories_str: String = row.get(2)?;
            let directories: Vec<String> = directories_str
                .split(',')
                .map(|s| s.to_string())
                .collect();

            Ok(ScanHistoryItem {
                id: row.get(0)?,
                scan_time: row.get(1)?,
                directories,
                total_files: row.get(3)?,
                duplicate_groups: row.get(4)?,
                duplicate_files: row.get(5)?,
                wasted_space: row.get(6)?,
                duration_seconds: row.get(7)?,
                status: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 获取总数
    let count_query = format!("SELECT COUNT(*) FROM scan_history {}", where_clause);
    let count_params: Vec<&dyn rusqlite::ToSql> = query_params[..query_params.len() - 2]
        .iter()
        .map(|p| p.as_ref())
        .collect();

    let total_count: i64 = conn
        .query_row(
            &count_query,
            rusqlite::params_from_iter(count_params.iter()),
            |row| row.get(0),
        )
        .unwrap_or(0);

    info!("获取到 {} 条扫描历史记录，总计 {}", history.len(), total_count);

    Ok(ScanHistoryResult {
        history,
        total_count,
        page,
        page_size,
    })
}

#[tauri::command(rename_all = "snake_case")]
fn add_scan_history(
    db_path: String,
    directories: Vec<String>,
    total_files: i64,
    duplicate_groups: i64,
    duplicate_files: i64,
    wasted_space: i64,
    duration_seconds: i64,
) -> Result<i64, String> {
    info!("添加扫描历史记录");
    
    let conn = init_database(&db_path)?;
    
    let directories_str = directories.join(",");
    
    conn.execute(
        "INSERT INTO scan_history (directories, total_files, duplicate_groups, duplicate_files, wasted_space, duration_seconds, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        (&directories_str, total_files, duplicate_groups, duplicate_files, wasted_space, duration_seconds, "completed"),
    ).map_err(|e| e.to_string())?;
    
    let id = conn.last_insert_rowid();
    
    info!("扫描历史记录已添加，ID: {}", id);
    Ok(id)
}

#[tauri::command(rename_all = "snake_case")]
fn clear_scan_history(
    db_path: String,
    before_date: Option<String>,
    keep_recent: Option<i64>,
) -> Result<i64, String> {
    info!("清空扫描历史");

    let conn = init_database(&db_path)?;

    let deleted_count: i64;

    if let Some(date) = before_date {
        // 删除指定日期之前的历史
        deleted_count = conn.execute(
            "DELETE FROM scan_history WHERE scan_time < ?",
            [&date],
        ).map_err(|e| e.to_string())? as i64;
        info!("已删除 {} 之前的历史记录，共 {} 条", date, deleted_count);
    } else if let Some(keep) = keep_recent {
        // 保留最近 N 条记录
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM scan_history", [], |row| row.get(0))
            .unwrap_or(0);
        let to_delete = total.saturating_sub(keep.max(0));
        if to_delete > 0 {
            deleted_count = conn.execute(
                "DELETE FROM scan_history WHERE id IN (
                    SELECT id FROM scan_history ORDER BY scan_time ASC LIMIT ?
                )",
                [to_delete],
            ).map_err(|e| e.to_string())? as i64;
            info!("已保留最近 {} 条历史记录，删除 {} 条", keep, deleted_count);
        } else {
            deleted_count = 0;
            info!("历史记录数量 {} 少于保留数量 {}，无需删除", total, keep);
        }
    } else {
        // 清空所有历史
        deleted_count = conn
            .execute("DELETE FROM scan_history", [])
            .map_err(|e| e.to_string())? as i64;
        info!("已清空所有历史记录，共 {} 条", deleted_count);
    }

    Ok(deleted_count)
}

#[tauri::command(rename_all = "snake_case")]
fn clear_database(db_path: String) -> Result<i64, String> {
    info!("清空数据库: {}", db_path);

    if db_path.is_empty() {
        return Err("数据库路径不能为空".to_string());
    }

    // 检查数据库文件是否存在
    let path = Path::new(&db_path);
    let db_exists = path.exists();
    
    if db_exists {
        // 获取记录数（用于返回）
        let conn = Connection::open(&db_path).map_err(|e| format!("打开数据库失败: {}", e))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .unwrap_or(0);
        
        // 关闭连接
        drop(conn);
        
        // 删除数据库文件（完全重建）
        fs::remove_file(&db_path).map_err(|e| format!("删除数据库文件失败: {}", e))?;
        info!("已删除旧数据库文件，共 {} 条记录", count);
        
        // 重新初始化数据库（创建新文件和表结构）
        init_database(&db_path)?;
        info!("已创建全新数据库");
        
        Ok(count)
    } else {
        // 数据库文件不存在，直接初始化
        init_database(&db_path)?;
        info!("数据库文件不存在，已创建新数据库");
        Ok(0)
    }
}

#[tauri::command(rename_all = "snake_case")]
fn get_scan_history_statistics(db_path: String) -> Result<ScanHistoryStatistics, String> {
    info!("获取扫描历史统计信息");

    let conn = init_database(&db_path)?;

    // 基本统计
    let (
        total_scans,
        completed_scans,
        interrupted_scans,
        total_files,
        total_duplicates,
        total_wasted_space,
        avg_duration,
    ): (i64, i64, i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                COUNT(*),
                COUNT(CASE WHEN status = 'completed' THEN 1 END),
                COUNT(CASE WHEN status = 'interrupted' THEN 1 END),
                COALESCE(SUM(total_files), 0),
                COALESCE(SUM(duplicate_files), 0),
                COALESCE(SUM(wasted_space), 0),
                COALESCE(AVG(duration_seconds), 0)
             FROM scan_history",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .unwrap_or((0, 0, 0, 0, 0, 0, 0));

    // 按天统计扫描频率（最近30天）
    let mut stmt = conn
        .prepare(
            "SELECT
                DATE(scan_time) as scan_date,
                COUNT(*) as count
             FROM scan_history
             WHERE scan_time >= DATE('now', '-30 days')
             GROUP BY DATE(scan_time)
             ORDER BY scan_date DESC",
        )
        .map_err(|e| e.to_string())?;

    let scan_frequency: Vec<DayScanCount> = stmt
        .query_map([], |row| {
            Ok(DayScanCount {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    info!(
        "扫描历史统计: 总计 {} 次扫描，完成 {} 次，中断 {} 次",
        total_scans, completed_scans, interrupted_scans
    );

    Ok(ScanHistoryStatistics {
        total_scans,
        completed_scans,
        interrupted_scans,
        total_files_scanned: total_files,
        total_duplicates_found: total_duplicates,
        total_wasted_space,
        average_scan_duration: avg_duration,
        scan_frequency_by_day: scan_frequency,
    })
}

// ========== 回收站功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn move_to_recycle_bin(
    path: String,
    db_path: String,
    recycle_bin_path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<String, String> {
    info!("移动文件到回收站: {}", path);
    
    let roots = allowed_roots.unwrap_or_default();
    
    if !is_path_safe(&path, &roots) {
        return Err("路径不安全".to_string());
    }
    
    let src_path = Path::new(&path);
    if !src_path.exists() {
        return Err("文件不存在".to_string());
    }
    
    let metadata = std::fs::metadata(src_path).map_err(|e| e.to_string())?;
    let size = metadata.len();
    
    // 生成回收站项目ID
    let id = format!("{}_{}", Utc::now().timestamp_millis(), rand::random::<u32>());
    
    // 创建回收站目录
    std::fs::create_dir_all(&recycle_bin_path).map_err(|e| e.to_string())?;
    
    // 生成目标路径
    let file_name = src_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let dest_path = Path::new(&recycle_bin_path).join(format!("{}_{}", id, file_name));
    
    // 先记录到数据库（确保元数据不会丢失）
    let conn = init_database(&db_path)?;
    conn.execute(
        "INSERT INTO recycle_bin (id, original_path, deleted_at, size, file_name) 
         VALUES (?, ?, ?, ?, ?)",
        (&id, &path, &Utc::now().to_rfc3339(), &(size as i64), file_name),
    ).map_err(|e| {
        warn!("回收站数据库记录失败 {}: {}", path, e);
        format!("回收站数据库记录失败: {}", e)
    })?;
    
    // 从文件表删除
    if let Err(e) = conn.execute("DELETE FROM files WHERE path = ?", [&path]) {
        warn!("从文件表删除失败 {}: {}", path, e);
        // 继续执行，因为回收站记录已创建
    }
    
    // 最后移动物理文件
    if let Err(e) = std::fs::rename(src_path, &dest_path) {
        // 物理移动失败，但数据库记录已存在
        warn!("物理文件移动到回收站失败 {}: {}，但数据库记录已保存", path, e);
        // 删除回收站记录以保持数据一致性
        let _ = conn.execute("DELETE FROM recycle_bin WHERE id = ?", [&id]);
        return Err(format!("移动文件到回收站失败: {}", e));
    }
    
    // 记录操作日志
    let _ = log_operation(&conn, "MOVE_TO_RECYCLE_BIN", &[path.clone()], None, Some(&dest_path.to_string_lossy()), "SUCCESS", None);
    
    info!("文件已移动到回收站: {}", id);
    Ok(id)
}

#[tauri::command(rename_all = "snake_case")]
fn list_recycle_bin(db_path: String) -> Result<RecycleBinList, String> {
    let conn = init_database(&db_path)?;
    
    let mut stmt = conn.prepare(
        "SELECT id, original_path, deleted_at, size, file_name 
         FROM recycle_bin ORDER BY deleted_at DESC"
    ).map_err(|e| e.to_string())?;
    
    let items: Vec<RecycleBinItem> = stmt.query_map([], |row| {
        Ok(RecycleBinItem {
            id: row.get(0)?,
            original_path: row.get(1)?,
            deleted_at: row.get(2)?,
            size: row.get::<_, i64>(3)? as u64,
            file_name: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    let total_count = items.len();
    let total_size: u64 = items.iter().map(|i| i.size).sum();
    
    Ok(RecycleBinList {
        items,
        total_count,
        total_size,
    })
}

#[tauri::command(rename_all = "snake_case")]
fn restore_from_recycle_bin(
    id: Option<String>,
    original_path: Option<String>,
    db_path: String,
    recycle_bin_path: String,
) -> Result<(), String> {
    info!("从回收站恢复文件: id={:?}, original_path={:?}", id, original_path);
    
    let conn = init_database(&db_path)?;
    
    // 如果有具体的 id，我们直接根据 id 查，否则根据 original_path 检索最新的一条记录
    let (target_id, target_original_path, file_name): (String, String, String) = if let Some(ref concrete_id) = id {
        let (orig_path, f_name): (String, String) = conn.query_row(
            "SELECT original_path, file_name FROM recycle_bin WHERE id = ?",
            [concrete_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("未找到指定的回收站记录: {}", e))?;
        (concrete_id.clone(), orig_path, f_name)
    } else if let Some(ref path) = original_path {
        let (found_id, f_name): (String, String) = conn.query_row(
            "SELECT id, file_name FROM recycle_bin WHERE original_path = ? ORDER BY deleted_at DESC LIMIT 1",
            [path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("未找到该路径的删除记录: {}", e))?;
        (found_id, path.clone(), f_name)
    } else {
        return Err("必须提供 id 或 original_path 其中之一".to_string());
    };
    
    // 检查原始位置是否已存在文件
    if Path::new(&target_original_path).exists() {
        return Err("原始位置已存在同名文件".to_string());
    }
    
    // 确保原始目录存在
    if let Some(parent) = Path::new(&target_original_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // 移动文件回原始位置
    let src_path = Path::new(&recycle_bin_path).join(format!("{}_{}", target_id, file_name));
    
    // 先移动物理文件
    if let Err(e) = std::fs::rename(&src_path, &target_original_path) {
        return Err(format!("恢复文件失败: {}", e));
    }
    
    // 物理移动成功后，从回收站表删除
    if let Err(e) = conn.execute("DELETE FROM recycle_bin WHERE id = ?", [&target_id]) {
        warn!("从回收站表删除记录失败 {}: {}，但文件已恢复", target_id, e);
        // 记录审计日志
        let _ = log_audit_event(&conn, "RESTORE_INCOMPLETE", "WARNING", &target_original_path, "RESTORE", 
            &format!("文件已恢复但数据库记录删除失败: {}", e), "PARTIAL");
    } else {
        // 记录操作日志
        let _ = log_operation(&conn, "RESTORE_FROM_RECYCLE_BIN", &[target_original_path.clone()], 
            Some(&src_path.to_string_lossy()), Some(&target_original_path), "SUCCESS", None);
    }
    
    info!("文件已恢复: {}", target_original_path);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
fn empty_recycle_bin(
    db_path: String,
    recycle_bin_path: String,
) -> Result<(), String> {
    info!("清空回收站，数据库: {}, 回收站目录: {}", db_path, recycle_bin_path);
    let conn = init_database(&db_path)?;
    
    // 1. 获取回收站中所有的记录
    let mut stmt = conn.prepare("SELECT id, file_name FROM recycle_bin").map_err(|e| e.to_string())?;
    let items: Vec<(String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // 2. 清空数据库中的回收站表
    conn.execute("DELETE FROM recycle_bin", []).map_err(|e| e.to_string())?;
    
    // 3. 删除所有的物理文件
    for (id, file_name) in items {
        let file_path = Path::new(&recycle_bin_path).join(format!("{}_{}", id, file_name));
        if file_path.exists() {
            if let Err(e) = std::fs::remove_file(&file_path) {
                warn!("清空回收站时物理文件删除失败 {}: {}", file_path.display(), e);
            }
        }
    }
    
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
fn permanently_delete_from_recycle_bin(
    id: String,
    db_path: String,
    recycle_bin_path: String,
) -> Result<(), String> {
    info!("永久删除回收站文件: {}", id);
    
    let conn = init_database(&db_path)?;
    
    // 获取文件名和原始路径
    let (file_name, original_path): (String, String) = conn.query_row(
        "SELECT file_name, original_path FROM recycle_bin WHERE id = ?",
        [&id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| e.to_string())?;
    
    // 先删除数据库记录（确保元数据不会丢失）
    if let Err(e) = conn.execute("DELETE FROM recycle_bin WHERE id = ?", [&id]) {
        return Err(format!("删除数据库记录失败: {}", e));
    }
    
    // 再删除物理文件
    let file_path = Path::new(&recycle_bin_path).join(format!("{}_{}", id, file_name));
    if file_path.exists() {
        if let Err(e) = std::fs::remove_file(&file_path) {
            warn!("物理文件删除失败 {}: {}，但数据库记录已删除", file_path.display(), e);
            // 记录审计日志
            let _ = log_audit_event(&conn, "PERMANENT_DELETE_INCOMPLETE", "WARNING", &original_path, "DELETE", 
                &format!("数据库记录已删除但物理文件删除失败: {}", e), "PARTIAL");
        } else {
            // 记录操作日志
            let _ = log_operation(&conn, "PERMANENT_DELETE", &[original_path], None, None, "SUCCESS", None);
        }
    }
    
    info!("文件已永久删除: {}", id);
    Ok(())
}

// ========== 空文件扫描功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn scan_empty_files(
    db_path: String,
    allowed_roots: Option<Vec<String>>,
) -> Result<EmptyFileScanResult, String> {
    info!("扫描空文件");
    
    let conn = init_database(&db_path)?;
    let roots = allowed_roots.unwrap_or_default();
    
    let mut stmt = conn.prepare(
        "SELECT id, path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension
         FROM files WHERE size = 0"
    ).map_err(|e| e.to_string())?;
    
    let empty_files: Vec<FileInfo> = stmt.query_map([], |row| {
        Ok(FileInfo {
            id: row.get(0)?,
            path: row.get(1)?,
            filename: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            size: row.get::<_, i64>(3)? as u64,
            hash: row.get(4)?,
            hash_algorithm: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "xxhash3".to_string()),
            created_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
            modified_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            file_extension: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .filter(|f| is_path_safe(&f.path, &roots))
    .collect();
    
    let total_count = empty_files.len();
    let total_size = 0u64;
    
    info!("找到 {} 个空文件", total_count);
    Ok(EmptyFileScanResult {
        empty_files,
        total_count,
        total_size,
    })
}

// ========== 重复文件夹检测功能 ==========

#[tauri::command(rename_all = "snake_case")]
fn find_duplicate_folders(
    db_path: String,
    similarity_threshold: f64,
    allowed_roots: Option<Vec<String>>,
) -> Result<DuplicateFolderResult, String> {
    info!("查找重复文件夹，相似度阈值: {}", similarity_threshold);
    
    let conn = init_database(&db_path)?;
    let roots = allowed_roots.unwrap_or_default();
    
    // 获取所有目录及其文件哈希
    let mut stmt = conn.prepare(
        "SELECT path, hash FROM files WHERE hash IS NOT NULL"
    ).map_err(|e| e.to_string())?;
    
    let file_hashes: Vec<(String, String)> = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let hash: String = row.get(1)?;
        Ok((path, hash))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .filter(|(path, _)| is_path_safe(path, &roots))
    .collect();
    
    // 按目录分组（使用 HashSet 方便去重并计算交集）
    let mut folder_hashes: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for (path, hash) in file_hashes {
        if let Some(parent) = Path::new(&path).parent() {
            let folder = parent.to_string_lossy().to_string();
            folder_hashes.entry(folder).or_insert_with(std::collections::HashSet::new).insert(hash);
        }
    }
    
    // 转换为方便索引的 Vec 结构
    let folders: Vec<(String, std::collections::HashSet<String>)> = folder_hashes.into_iter().collect();
    
    // 建立倒排索引：文件哈希 -> 包含该哈希的文件夹在 folders 中的索引下标
    let mut hash_to_folders: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, (_, hashes)) in folders.iter().enumerate() {
        for hash in hashes {
            hash_to_folders.entry(hash.clone()).or_default().push(idx);
        }
    }
    
    // 查找相似文件夹
    let mut duplicate_folders: Vec<DuplicateFolderGroup> = Vec::new();
    let mut processed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    
    for i in 0..folders.len() {
        if processed.contains(&i) {
            continue;
        }
        
        let (ref folder1, ref hashes1) = folders[i];
        let mut similar_folders = vec![folder1.clone()];
        let mut total_file_count = hashes1.len();
        
        // 倒排匹配剪枝：只考虑与当前文件夹至少有一个共同哈希文件的其他文件夹
        let mut common_counts: HashMap<usize, usize> = HashMap::new();
        for hash in hashes1 {
            if let Some(folder_indices) = hash_to_folders.get(hash) {
                for &j in folder_indices {
                    if j > i && !processed.contains(&j) {
                        *common_counts.entry(j).or_default() += 1;
                    }
                }
            }
        }
        
        for (j, common_count) in common_counts {
            let hashes2 = &folders[j].1;
            
            // 计算 Jaccard 相似度（重合大小 / 并集大小）
            let total_unique = hashes1.len() + hashes2.len() - common_count;
            let similarity = if total_unique > 0 {
                common_count as f64 / total_unique as f64
            } else {
                0.0
            };
            
            if similarity >= similarity_threshold {
                similar_folders.push(folders[j].0.clone());
                total_file_count += hashes2.len();
                processed.insert(j);
            }
        }
        
        if similar_folders.len() > 1 {
            // 计算总大小
            let total_size: u64 = similar_folders.iter()
                .filter_map(|f| {
                    conn.query_row(
                        "SELECT SUM(size) FROM files WHERE path LIKE ? || '%'",
                        [f],
                        |row| row.get::<_, i64>(0)
                    ).ok()
                })
                .map(|s| s as u64)
                .sum();
            
            duplicate_folders.push(DuplicateFolderGroup {
                folder_paths: similar_folders,
                file_count: total_file_count,
                total_size,
                similarity: similarity_threshold,
            });
        }
        
        processed.insert(i);
    }
    
    let total_groups = duplicate_folders.len();
    let total_wasted: u64 = duplicate_folders.iter()
        .map(|g| if g.folder_paths.len() > 1 { 
            g.total_size / g.folder_paths.len() as u64 * (g.folder_paths.len() - 1) as u64 
        } else { 
            0 
        })
        .sum();
    
    info!("找到 {} 个重复文件夹组", total_groups);
    Ok(DuplicateFolderResult {
        duplicate_folders,
        total_groups,
        total_wasted_space: total_wasted,
    })
}

// ========== 版本信息 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVersionInfo {
    pub app_name: String,
    pub app_version: String,
    pub tauri_version: String,
    pub rust_version: String,
    pub build_date: String,
    pub author: String,
    pub license: String,
    pub description: String,
}

#[tauri::command]
fn get_app_version_info() -> AppVersionInfo {
    AppVersionInfo {
        app_name: "空间树 (SpaceTree)".to_string(),
        app_version: "3.40.0".to_string(),
        tauri_version: "2.10.3".to_string(),
        rust_version: "1.77.2".to_string(),
        build_date: "2026-07-12".to_string(),
        author: "呆若木鸡".to_string(),
        license: "MIT License".to_string(),
        description: "一款功能强大的重复文件查找和管理工具".to_string(),
    }
}

#[tauri::command]
fn get_exe_dir() -> Result<String, String> {
    use std::env;
    
    // 尝试获取可执行文件路径
    match env::current_exe() {
        Ok(exe_path) => {
            info!("可执行文件路径: {:?}", exe_path);
            
            // 获取父目录
            match exe_path.parent() {
                Some(parent) => {
                    let dir = parent.to_string_lossy().to_string();
                    info!("可执行文件所在目录: {}", dir);
                    Ok(dir)
                }
                None => {
                    // 如果没有父目录（在根目录下），使用当前工作目录
                    warn!("可执行文件没有父目录，使用当前工作目录");
                    env::current_dir()
                        .map(|d| d.to_string_lossy().to_string())
                        .map_err(|e| format!("无法获取当前工作目录: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("无法获取可执行文件路径: {}", e);
            // 如果无法获取可执行文件路径，使用当前工作目录
            env::current_dir()
                .map(|d| d.to_string_lossy().to_string())
                .map_err(|e| format!("无法获取当前工作目录: {}", e))
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
fn check_directory_writable(dir_path: String) -> Result<bool, String> {
    use std::fs;
    use std::path::Path;
    
    let path = Path::new(&dir_path);
    
    // 检查目录是否存在
    if !path.exists() {
        // 尝试创建目录
        match fs::create_dir_all(path) {
            Ok(_) => {},
            Err(e) => return Err(format!("无法创建目录 '{}': {}", dir_path, e)),
        }
    }
    
    // 检查是否是目录
    if !path.is_dir() {
        return Err(format!("路径 '{}' 不是目录", dir_path));
    }
    
    // 尝试创建一个临时文件来测试写入权限
    let test_file = path.join(".write_test_tmp");
    match fs::write(&test_file, b"test") {
        Ok(_) => {
            // 删除测试文件
            let _ = fs::remove_file(&test_file);
            Ok(true)
        }
        Err(e) => Err(format!("目录 '{}' 没有写入权限: {}", dir_path, e)),
    }
}

// ========== 清理影响分析 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupImpactRequest {
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupImpactResult {
    pub files_to_delete: usize,
    pub space_to_free: u64,
    pub total_groups: usize,
    pub safety_score: i32,
    pub warnings: Vec<String>,
}

#[tauri::command]
fn analyze_cleanup_impact(request: CleanupImpactRequest) -> Result<CleanupImpactResult, String> {
    info!("分析清理影响");
    
    let groups = request.groups;
    let total_groups = groups.len();
    let mut files_to_delete = 0;
    let mut space_to_free: u64 = 0;
    let mut warnings = Vec::new();
    
    for group in &groups {
        if group.files.len() > 1 {
            // 保留第一个文件，删除其余的
            files_to_delete += group.files.len() - 1;
            space_to_free += group.files.iter().skip(1).map(|f| f.size).sum::<u64>();
        }
    }
    
    // 计算安全评分（简单算法）
    let safety_score = if files_to_delete == 0 {
        100
    } else {
        let base_score = 80;
        let group_bonus = (total_groups as i32 * 2).min(20);
        base_score + group_bonus
    };
    
    if files_to_delete > 100 {
        warnings.push(format!("将删除 {} 个文件，请谨慎操作", files_to_delete));
    }
    
    info!("清理影响分析完成: {} 个文件, {} 字节", files_to_delete, space_to_free);
    Ok(CleanupImpactResult {
        files_to_delete,
        space_to_free,
        total_groups,
        safety_score,
        warnings,
    })
}

// ========== 企业级扫描器命令 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseScanRequest {
    pub directories: Vec<String>,
    pub db_path: String,
    pub config: Option<EnterpriseScanConfig>,
}

#[tauri::command(rename_all = "snake_case")]
fn enterprise_scan_directories(
    request: EnterpriseScanRequest,
    _app_handle: tauri::AppHandle,
) -> Result<Vec<FileInfo>, String> {
    info!("企业级扫描启动: {:?}", request.directories);

    let config = request.config.unwrap_or_default();
    let scanner = EnterpriseScanner::new(config);

    // 执行扫描
    let file_infos = scanner.scan_directories(&request.directories)?;

    // 保存到数据库
    let conn = init_database(&request.db_path)?;
    save_files_to_database(&conn, &file_infos)?;

    info!("企业级扫描完成，发现 {} 个文件", file_infos.len());
    Ok(file_infos)
}

#[tauri::command]
fn get_scan_statistics() -> Result<serde_json::Value, String> {
    // 返回默认统计，实际使用时需要从扫描器获取
    Ok(serde_json::json!({
        "total_files": 0,
        "scanned_files": 0,
        "files_per_second": 0.0
    }))
}

#[tauri::command(rename_all = "snake_case")]
fn get_storage_stats(db_path: String) -> Result<serde_json::Value, String> {
    use std::fs;

    info!("获取存储统计: db_path={}", db_path);

    let mut database_size: u64 = 0;
    let mut scan_history_count: i64 = 0;
    let mut log_file_size: u64 = 0;
    let mut files_count: i64 = 0;

    // 处理 Windows 长路径前缀
    let normalized_path = if db_path.starts_with("\\\\?\\") {
        db_path[4..].to_string()
    } else {
        db_path.clone()
    };

    info!("标准化后的路径: {}", normalized_path);

    // 获取数据库文件大小
    match fs::metadata(&normalized_path) {
        Ok(metadata) => {
            database_size = metadata.len();
            info!("数据库文件大小: {} bytes", database_size);
        }
        Err(e) => {
            warn!("无法获取数据库文件大小: {:?}, 路径: {}", e, normalized_path);
        }
    }

    // 获取数据库中的记录数
    if !normalized_path.is_empty() {
        match Connection::open(&normalized_path) {
            Ok(conn) => {
                // 查询 scan_history 表记录数
                match conn.query_row(
                    "SELECT COUNT(*) FROM scan_history",
                    [],
                    |row| row.get::<_, i64>(0)
                ) {
                    Ok(count) => {
                        scan_history_count = count;
                        info!("扫描历史记录数: {}", scan_history_count);
                    }
                    Err(e) => {
                        warn!("查询扫描历史记录数失败: {:?}", e);
                    }
                }

                // 查询 files 表记录数（用于调试）
                match conn.query_row(
                    "SELECT COUNT(*) FROM files",
                    [],
                    |row| row.get::<_, i64>(0)
                ) {
                    Ok(count) => {
                        files_count = count;
                        info!("文件记录数: {}", files_count);
                    }
                    Err(e) => {
                        warn!("查询文件记录数失败: {:?}", e);
                    }
                }
            }
            Err(e) => {
                warn!("无法打开数据库: {:?}, 路径: {}", e, normalized_path);
            }
        }
    } else {
        warn!("数据库路径为空");
    }

    // 获取日志文件大小（如果存在）
    let log_path = std::path::Path::new(&normalized_path).with_extension("log");
    if let Ok(metadata) = fs::metadata(&log_path) {
        log_file_size = metadata.len();
        info!("日志文件大小: {} bytes", log_file_size);
    }

    info!("存储统计结果: database_size={}, scan_history_count={}, files_count={}, log_file_size={}",
          database_size, scan_history_count, files_count, log_file_size);

    Ok(serde_json::json!({
        "database_size": database_size,
        "scan_history_count": scan_history_count,
        "files_count": files_count,
        "log_file_size": log_file_size
    }))
}

#[tauri::command(rename_all = "snake_case")]
fn check_file_exists(path: String) -> Result<bool, String> {
    use std::fs;

    info!("检查文件是否存在: {}", path);

    // 处理 Windows 长路径前缀
    let normalized_path = if path.starts_with("\\\\?\\") {
        path[4..].to_string()
    } else {
        path
    };

    let exists = fs::metadata(&normalized_path).is_ok();
    info!("文件 {} 存在性: {}", normalized_path, exists);

    Ok(exists)
}

/// 保存文件到数据库
fn save_files_to_database(conn: &Connection, files: &[FileInfo]) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for file in files {
        let (volume_guid, relative_path) = {
            let mut guid = None;
            let mut rel = file.filename.clone();
            
            #[cfg(windows)]
            {
                if file.path.len() >= 3 && file.path.chars().nth(1) == Some(':') {
                    let drive_prefix = &file.path[..3];
                    let drive_w = drive_prefix.replace("/", "\\");
                    guid = get_volume_guid_for_path(&drive_w);
                    rel = file.path[3..].to_string();
                }
            }
            (guid, rel)
        };

        tx.execute(
            "INSERT OR REPLACE INTO files 
             (path, filename, size, hash, hash_algorithm, created_at, modified_at, file_extension, scanned_at, volume_guid, relative_path) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &file.path,
                &file.filename,
                &(file.size as i64),
                &file.hash,
                &file.hash_algorithm,
                &file.created_at,
                &file.modified_at,
                &file.file_extension,
                &Utc::now().to_rfc3339(),
                volume_guid,
                relative_path,
            ),
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ========== 应用程序入口 ==========

pub fn run() {
    env_logger::init();
    info!("SpaceTree 空间树启动");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![find_duplicates_partial, check_and_remap_volumes, verify_file_hashes_match, replace_files_with_hardlinks, 
            // 核心功能
            scan_directories,
            incremental_scan,
            get_scan_progress,
            pause_scan,
            resume_scan,
            stop_scan,
            find_duplicates,
            find_duplicates_advanced,
            check_database_has_files,
            debug_database_files,
            delete_file,
            smart_select_duplicates,
            batch_delete_files,
            
            // 图像功能
            find_similar_images,
            generate_thumbnail_command,
            
            // 文件操作
            show_in_folder,
            open_file,
            read_file_content,
            write_file,
            
            // 导出功能
            export_duplicates,
            
            // 分析功能
            analyze_duplicate_distribution,
            analyze_cleanup_impact,
            
            // 合规检查
            generate_compliance_report,
            
            // 多维度分析报告
            generate_multi_dimensional_report,
            
            // 日志审计
            query_operation_logs,
            query_audit_logs,
            
            // 回收站
            move_to_recycle_bin,
            list_recycle_bin,
            restore_from_recycle_bin,
            permanently_delete_from_recycle_bin,
            empty_recycle_bin,
            
            // 空文件扫描
            scan_empty_files,
            
            // 重复文件夹
            find_duplicate_folders,
            
            // 设置
            save_settings,
            load_settings,
            
            // 文件系统监控
            start_file_watcher_command,
            stop_file_watcher_command,
            get_file_watcher_status,
            add_watch_path_command,
            remove_watch_path_command,
            
            // 搜索
            search_files,
            
            // 目录树功能
            get_directory_tree,
            get_files_in_directory,
            get_all_files,
            search_in_directory,
            
            // 图片打包功能
            analyze_image_directories,
            create_archive,
            delete_files_after_archive,
            
            // 扫描历史
            get_scan_history_list,
            add_scan_history,
            clear_scan_history,
            get_scan_history_statistics,

            // 数据库管理
            clear_database,
            
            // 版本信息
            get_app_version_info,
            
            // 系统路径
            get_exe_dir,
            
            // 数据库初始化
            init_database_cmd,
            
            // 目录权限检查
            check_directory_writable,

            // 企业级扫描器
            enterprise_scan_directories,
            get_scan_statistics,

            // 存储管理
            get_storage_stats,
            check_file_exists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
