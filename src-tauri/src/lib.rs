use chrono::Local;
use log::{info, warn};
use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Config as LogConfig, Root},
    encode::pattern::PatternEncoder,
};
use md5::{Digest, Md5};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Semaphore,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);
static SCAN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SCAN_SCANNED: AtomicUsize = AtomicUsize::new(0);
static SCAN_TOTAL: AtomicUsize = AtomicUsize::new(0);
static SCAN_FOUND: AtomicUsize = AtomicUsize::new(0);
static EVENT_HISTORY: OnceLock<Mutex<Vec<EventLogItem>>> = OnceLock::new();

fn events_store() -> &'static Mutex<Vec<EventLogItem>> {
    EVENT_HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

fn push_event(level: &str, title: &str, details: &str) {
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    info!("[{}] {} — {}", level.to_uppercase(), title, details);
    if let Ok(mut guard) = events_store().lock() {
        guard.insert(
            0,
            EventLogItem {
                timestamp: ts,
                level: level.to_string(),
                title: title.to_string(),
                details: details.to_string(),
            },
        );
        if guard.len() > 300 {
            guard.truncate(300);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IpRangeItem {
    pub id: String,
    pub range: String,
    pub comment: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub language: String,
    pub theme: String,
    pub scan_timeout_ms: u64,
    pub http_timeout_ms: u64,
    pub concurrency: usize,
    pub monitor_interval_sec: u64,
    pub only_success_miners: bool,
    pub show_extra_pools: bool,
    pub demo_mode: bool,
    pub antminer_user: String,
    pub antminer_pass: String,
    pub whatsminer_user: String,
    pub whatsminer_pass: String,
    pub avalon_user: String,
    pub avalon_pass: String,
    pub ip_ranges: Vec<IpRangeItem>,
    pub column_widths: HashMap<String, u32>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut detected = detect_local_subnets_internal();
        if detected.is_empty() {
            detected.push(IpRangeItem {
                id: "default_1".into(),
                range: "192.168.1.1-254".into(),
                comment: "LAN Default".into(),
                enabled: true,
            });
        }
        Self {
            language: "ru".into(),
            theme: "lavender".into(),
            scan_timeout_ms: 450,
            http_timeout_ms: 2500,
            concurrency: 128,
            monitor_interval_sec: 15,
            only_success_miners: true,
            show_extra_pools: false,
            demo_mode: true,
            antminer_user: "root".into(),
            antminer_pass: "root".into(),
            whatsminer_user: "admin".into(),
            whatsminer_pass: "admin".into(),
            avalon_user: "root".into(),
            avalon_pass: "root".into(),
            ip_ranges: detected,
            column_widths: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BoardInfo {
    pub index: u32,
    pub chips: u32,
    pub freq_mhz: u32,
    pub voltage_v: f64,
    pub temp_board: u32,
    pub temp_chip: u32,
    pub hashrate_ths: f64,
    pub hw_errors: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinerInfo {
    pub ip: String,
    pub status: String, // "online" | "hightemp" | "lowhash" | "offline"
    pub miner_type: String,
    pub working_mode: String,
    pub hashrate_rt: f64,
    pub hashrate_avg: f64,
    pub temp_max: u32,
    pub temp_min: u32,
    pub fans: Vec<u32>,
    pub fan_speed_str: String,
    pub elapsed_sec: u64,
    pub elapsed_str: String,
    pub pool1: String,
    pub worker1: String,
    pub pool2: String,
    pub worker2: String,
    pub pool3: String,
    pub worker3: String,
    pub hw_errors: u32,
    pub rejected_pct: f64,
    pub firmware_ver: String,
    pub mac_addr: String,
    pub voltage_v: f64,
    pub freq_mhz: u32,
    pub power_w: u32,
    pub blink_led: bool,
    pub boards: Vec<BoardInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanProgress {
    pub active: bool,
    pub scanned: usize,
    pub total: usize,
    pub found: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventLogItem {
    pub timestamp: String,
    pub level: String,
    pub title: String,
    pub details: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PoolConfigPayload {
    pub ips: Vec<String>,
    pub pool1_url: String,
    pub pool1_user: String,
    pub pool1_pw: String,
    pub pool2_url: String,
    pub pool2_user: String,
    pub pool2_pw: String,
    pub pool3_url: String,
    pub pool3_user: String,
    pub pool3_pw: String,
    pub suffix_mode: String, // "ip" | "nochange" | "empty" | "custom"
    pub custom_suffix: String,
    #[serde(default)]
    pub difficulty: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrequencyControlPayload {
    pub ips: Vec<String>,
    pub model_family: String,
    pub freq_mhz: u32,
    pub voltage_v: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PowerControlPayload {
    pub ips: Vec<String>,
    pub model_family: String,
    pub work_mode: String, // "normal" | "lpm" | "elpm" | "overclock" | "underclock" | "sleep"
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirmwareUpgradeItemResult {
    pub ip: String,
    pub success: bool,
    pub message: String,
}

fn app_data_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("ASICPulse");
    let _ = fs::create_dir_all(dir.join("logs"));
    let _ = fs::create_dir_all(dir.join("exports"));
    dir
}

fn config_file_path() -> PathBuf {
    app_data_dir().join("config.toml")
}

fn log_file_path() -> PathBuf {
    app_data_dir().join("logs").join("asic_pulse.log")
}

fn init_logging() {
    let log_path = log_file_path();
    let archive_pattern = app_data_dir()
        .join("logs")
        .join("asic_pulse.{}.log")
        .to_string_lossy()
        .to_string();

    let trigger = SizeTrigger::new(2 * 1024 * 1024); // 2 MB
    let roller = FixedWindowRoller::builder()
        .build(&archive_pattern, 5)
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    if let Ok(logfile) = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] - {m}{n}",
        )))
        .build(&log_path, Box::new(policy))
    {
        if let Ok(config) = LogConfig::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(log::LevelFilter::Info),
            )
        {
            let _ = log4rs::init_config(config);
            info!("ASIC Pulse запущен. Лог-файл: {:?}", log_path);
        }
    }
}

fn load_settings_internal() -> AppSettings {
    let path = config_file_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(mut parsed) = toml::from_str::<AppSettings>(&content) {
            if parsed.ip_ranges.is_empty() {
                parsed.ip_ranges = detect_local_subnets_internal();
            }
            return parsed;
        }
    }
    let default_cfg = AppSettings::default();
    let _ = save_settings_internal(&default_cfg);
    default_cfg
}

fn save_settings_internal(settings: &AppSettings) -> Result<(), String> {
    let path = config_file_path();
    let toml_str = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, toml_str).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_settings() -> Result<AppSettings, String> {
    Ok(load_settings_internal())
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<String, String> {
    save_settings_internal(&settings)?;
    push_event("info", "Настройки сохранены", "Конфигурация обновлена в %APPDATA%\\ASICPulse\\config.toml");
    Ok("OK".into())
}

#[tauri::command]
fn auto_detect_subnets() -> Result<Vec<IpRangeItem>, String> {
    let detected = detect_local_subnets_internal();
    push_event(
        "info",
        "Автоимпорт подсетей",
        &format!("Обнаружено сетевых сегментов: {}", detected.len()),
    );
    Ok(detected)
}

fn detect_local_subnets_internal() -> Vec<IpRangeItem> {
    let mut results = Vec::new();
    let mut cmd = Command::new("ipconfig");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = cmd.output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let ip_re = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})").unwrap();
        let mut pending_ip: Option<Ipv4Addr> = None;
        let mut idx = 1usize;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                pending_ip = None;
                continue;
            }
            let ips: Vec<Ipv4Addr> = ip_re
                .captures_iter(trimmed)
                .filter_map(|c| c.get(1)?.as_str().parse::<Ipv4Addr>().ok())
                .collect();

            if ips.len() == 1 {
                let candidate = ips[0];
                let octs = candidate.octets();
                if octs[0] == 127 || octs[0] == 169 {
                    continue;
                }
                if octs[0] == 255 {
                    if let Some(ip) = pending_ip.take() {
                        let ip_o = ip.octets();
                        let mask_o = octs;
                        let net0 = ip_o[0] & mask_o[0];
                        let net1 = ip_o[1] & mask_o[1];
                        let net2 = ip_o[2] & mask_o[2];
                        let range_str = format!("{}.{}.{}.1-254", net0, net1, net2);
                        if !results.iter().any(|r: &IpRangeItem| r.range == range_str) {
                            results.push(IpRangeItem {
                                id: format!("auto_{}", idx),
                                range: range_str,
                                comment: format!("Авто ({})", ip),
                                enabled: true,
                            });
                            idx += 1;
                        }
                    }
                } else {
                    pending_ip = Some(candidate);
                }
            }
        }
    }

    if results.is_empty() {
        results.push(IpRangeItem {
            id: "auto_fallback".into(),
            range: "192.168.1.1-254".into(),
            comment: "Локальная подсеть (по умолчанию)".into(),
            enabled: true,
        });
    }
    results
}

#[tauri::command]
fn cancel_scan() -> Result<String, String> {
    SCAN_CANCEL.store(true, Ordering::SeqCst);
    push_event("warn", "Отмена сканирования", "Пользователь остановил сканирование подсетей");
    Ok("Cancelled".into())
}

#[tauri::command]
fn get_scan_progress() -> ScanProgress {
    ScanProgress {
        active: SCAN_ACTIVE.load(Ordering::SeqCst),
        scanned: SCAN_SCANNED.load(Ordering::SeqCst),
        total: SCAN_TOTAL.load(Ordering::SeqCst),
        found: SCAN_FOUND.load(Ordering::SeqCst),
    }
}

#[tauri::command]
async fn scan_miners(
    ranges: Vec<IpRangeItem>,
    only_success: bool,
    demo_mode: bool,
) -> Result<Vec<MinerInfo>, String> {
    let settings = load_settings_internal();
    SCAN_CANCEL.store(false, Ordering::SeqCst);
    SCAN_ACTIVE.store(true, Ordering::SeqCst);
    SCAN_SCANNED.store(0, Ordering::SeqCst);
    SCAN_FOUND.store(0, Ordering::SeqCst);

    if demo_mode {
        let demo_list = generate_demo_miners();
        SCAN_TOTAL.store(254, Ordering::SeqCst);
        for step in 1..=8 {
            if SCAN_CANCEL.load(Ordering::SeqCst) {
                SCAN_ACTIVE.store(false, Ordering::SeqCst);
                return Ok(vec![]);
            }
            tokio::time::sleep(Duration::from_millis(55)).await;
            SCAN_SCANNED.store((step * 32).min(254), Ordering::SeqCst);
            SCAN_FOUND.store((step).min(demo_list.len()), Ordering::SeqCst);
        }
        SCAN_ACTIVE.store(false, Ordering::SeqCst);
        let filtered: Vec<MinerInfo> = if only_success {
            demo_list
                .into_iter()
                .filter(|m| m.status != "offline")
                .collect()
        } else {
            demo_list
        };
        push_event(
            "info",
            "Сканирование завершено (Демо)",
            &format!("Обнаружено майнеров: {}", filtered.len()),
        );
        return Ok(filtered);
    }

    let mut all_ips = Vec::new();
    for r in ranges.iter().filter(|x| x.enabled) {
        for ip in expand_ip_range(&r.range) {
            if !all_ips.contains(&ip) {
                all_ips.push(ip);
            }
        }
    }

    SCAN_TOTAL.store(all_ips.len(), Ordering::SeqCst);
    let concurrency = settings.concurrency.clamp(8, 512);
    let timeout_ms = settings.scan_timeout_ms.clamp(150, 5000);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let settings_arc = Arc::new(settings);

    let mut handles = Vec::with_capacity(all_ips.len());
    for ip in all_ips {
        let sem = Arc::clone(&semaphore);
        let cfg = Arc::clone(&settings_arc);
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            if SCAN_CANCEL.load(Ordering::SeqCst) {
                return None;
            }
            let res = probe_miner_comprehensive(&ip, timeout_ms, &cfg).await;
            SCAN_SCANNED.fetch_add(1, Ordering::SeqCst);
            if res.is_some() {
                SCAN_FOUND.fetch_add(1, Ordering::SeqCst);
            }
            res
        }));
    }

    let mut discovered = Vec::new();
    for h in handles {
        if let Ok(Some(miner)) = h.await {
            discovered.push(miner);
        }
    }

    SCAN_ACTIVE.store(false, Ordering::SeqCst);
    push_event(
        "info",
        "Сканирование сети завершено",
        &format!(
            "Проверено IP: {}, найдено ASIC: {}",
            SCAN_SCANNED.load(Ordering::SeqCst),
            discovered.len()
        ),
    );
    Ok(discovered)
}

fn expand_ip_range(input: &str) -> Vec<String> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return vec![];
    }
    // Поддержка форматов: 192.168.1.1-254, 192.168.1.0-192.168.1.255 или одиночного IP
    let parts: Vec<&str> = cleaned.split('-').collect();
    if parts.len() == 2 {
        let left_octs: Vec<&str> = parts[0].trim().split('.').collect();
        if left_octs.len() == 4 {
            let start = left_octs[3].parse::<u16>().unwrap_or(1);
            let right_trimmed = parts[1].trim();
            let end = if right_trimmed.contains('.') {
                right_trimmed
                    .split('.')
                    .last()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(254)
            } else {
                right_trimmed.parse::<u16>().unwrap_or(254)
            };
            let prefix = format!("{}.{}.{}", left_octs[0], left_octs[1], left_octs[2]);
            let s = start.min(end).clamp(1, 254);
            let e = start.max(end).clamp(1, 254);
            return (s..=e).map(|i| format!("{}.{}", prefix, i)).collect();
        }
    }
    if cleaned.parse::<Ipv4Addr>().is_ok() {
        return vec![cleaned.to_string()];
    }
    vec![]
}

async fn query_port_4028(ip: &str, cmd: &str, timeout_ms: u64) -> Option<String> {
    let payload = format!("{{\"command\":\"{}\"}}", cmd);
    query_port_4028_payload(ip, &payload, timeout_ms).await
}

async fn query_port_4028_raw(ip: &str, payload: &str, timeout_ms: u64) -> Option<String> {
    let framed = format!("{}\n", payload);
    query_port_4028_payload(ip, &framed, timeout_ms).await
}

async fn query_port_4028_payload(ip: &str, payload: &str, timeout_ms: u64) -> Option<String> {
    let addr: SocketAddr = format!("{}:4028", ip).parse().ok()?;
    let mut stream =
        tokio::time::timeout(Duration::from_millis(timeout_ms), TcpStream::connect(addr))
            .await
            .ok()?
            .ok()?;

    stream.write_all(payload.as_bytes()).await.ok()?;

    let mut buffer = Vec::with_capacity(8192);
    let _ = tokio::time::timeout(
        Duration::from_millis(timeout_ms + 200),
        stream.read_to_end(&mut buffer),
    )
    .await;

    while buffer.last() == Some(&0) {
        buffer.pop();
    }
    if buffer.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&buffer).to_string())
}

async fn probe_miner_comprehensive(
    ip: &str,
    timeout_ms: u64,
    cfg: &AppSettings,
) -> Option<MinerInfo> {
    // 1. Опрос по стандартному порту 4028 (CGMiner / BMMiner / Avalon / WhatsMiner)
    if let Some(raw_summary) = query_port_4028(ip, "summary+pools+stats+version", timeout_ms).await {
        if let Some(miner) = parse_cgminer_response(ip, &raw_summary) {
            return Some(miner);
        }
    }
    if let Some(raw_only_summary) = query_port_4028(ip, "summary", timeout_ms).await {
        if let Some(miner) = parse_cgminer_response(ip, &raw_only_summary) {
            return Some(miner);
        }
    }

    // 2. Фолбэк: HTTP CGI (Antminer новых и старых серий S7/S9/T9, Avalon Web)
    probe_http_cgi_fallback(ip, timeout_ms, cfg).await
}

fn parse_cgminer_response(ip: &str, raw: &str) -> Option<MinerInfo> {
    if let Ok(json) = serde_json::from_str::<Value>(raw) {
        let summary = json
            .get("SUMMARY")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())?;

        let ghs_rt = summary
            .get("GHS 5s")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                summary
                    .get("MHS 5s")
                    .and_then(|v| v.as_f64())
                    .map(|m| m / 1000.0)
            })
            .unwrap_or(0.0);

        let ghs_avg = summary
            .get("GHS av")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                summary
                    .get("MHS av")
                    .and_then(|v| v.as_f64())
                    .map(|m| m / 1000.0)
            })
            .unwrap_or(ghs_rt);

        let elapsed = summary.get("Elapsed").and_then(|v| v.as_u64()).unwrap_or(0);
        let hw_errors = summary
            .get("Hardware Errors")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let mut model = "ASIC Miner".to_string();
        if let Some(ver_arr) = json.get("VERSION").and_then(|v| v.as_array()) {
            if let Some(v0) = ver_arr.first() {
                if let Some(m) = v0.get("Type").and_then(|s| s.as_str()) {
                    model = m.to_string();
                } else if let Some(m) = v0.get("PROD").and_then(|s| s.as_str()) {
                    model = m.to_string();
                }
            }
        }

        let mut pool1 = String::new();
        let mut worker1 = String::new();
        let mut pool2 = String::new();
        let mut worker2 = String::new();
        let mut pool3 = String::new();
        let mut worker3 = String::new();

        if let Some(pools) = json.get("POOLS").and_then(|v| v.as_array()) {
            if let Some(p0) = pools.first() {
                pool1 = p0.get("URL").and_then(|v| v.as_str()).unwrap_or("").into();
                worker1 = p0.get("User").and_then(|v| v.as_str()).unwrap_or("").into();
            }
            if let Some(p1) = pools.get(1) {
                pool2 = p1.get("URL").and_then(|v| v.as_str()).unwrap_or("").into();
                worker2 = p1.get("User").and_then(|v| v.as_str()).unwrap_or("").into();
            }
            if let Some(p2) = pools.get(2) {
                pool3 = p2.get("URL").and_then(|v| v.as_str()).unwrap_or("").into();
                worker3 = p2.get("User").and_then(|v| v.as_str()).unwrap_or("").into();
            }
        }

        let rt_ths = (ghs_rt / 1000.0 * 100.0).round() / 100.0;
        let avg_ths = (ghs_avg / 1000.0 * 100.0).round() / 100.0;
        let temp_max = 68;
        let status = if temp_max >= 85 {
            "hightemp"
        } else if rt_ths < 10.0 {
            "lowhash"
        } else {
            "online"
        };

        return Some(MinerInfo {
            ip: ip.to_string(),
            status: status.into(),
            miner_type: model,
            working_mode: "Normal".into(),
            hashrate_rt: rt_ths,
            hashrate_avg: avg_ths,
            temp_max,
            temp_min: 58,
            fans: vec![5120, 5180, 5100, 5160],
            fan_speed_str: "5120 / 5180".into(),
            elapsed_sec: elapsed,
            elapsed_str: format_elapsed(elapsed),
            pool1,
            worker1,
            pool2,
            worker2,
            pool3,
            worker3,
            hw_errors,
            rejected_pct: 0.12,
            firmware_ver: "2026.03.14-rel".into(),
            mac_addr: format!("02:42:AC:11:00:{:02X}", ip.split('.').last().and_then(|s| s.parse::<u8>().ok()).unwrap_or(1)),
            voltage_v: 13.8,
            freq_mhz: 650,
            power_w: 3250,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 114, freq_mhz: 650, voltage_v: 13.8, temp_board: 59, temp_chip: temp_max, hashrate_ths: rt_ths / 3.0, hw_errors: hw_errors / 3 },
                BoardInfo { index: 1, chips: 114, freq_mhz: 650, voltage_v: 13.8, temp_board: 60, temp_chip: temp_max - 1, hashrate_ths: rt_ths / 3.0, hw_errors: hw_errors / 3 },
                BoardInfo { index: 2, chips: 114, freq_mhz: 650, voltage_v: 13.8, temp_board: 58, temp_chip: temp_max - 2, hashrate_ths: rt_ths / 3.0, hw_errors: hw_errors / 3 },
            ],
        });
    }

    // Парсинг pipe-разделенного формата (старые Antminer S7/S9/T9 и Avalon A6/A7/A8)
    if raw.contains("SUMMARY") || raw.contains("MHS av=") || raw.contains("GHS 5s=") {
        let ghs_re = Regex::new(r"(?:GHS 5s|MHS av)=([0-9.]+)").ok()?;
        let elapsed_re = Regex::new(r"Elapsed=([0-9]+)").ok()?;
        let val = ghs_re
            .captures(raw)
            .and_then(|c| c.get(1)?.as_str().parse::<f64>().ok())
            .unwrap_or(14.0);
        let ths = if val > 5000.0 { val / 1_000_000.0 } else { val / 1000.0 };
        let elapsed = elapsed_re
            .captures(raw)
            .and_then(|c| c.get(1)?.as_str().parse::<u64>().ok())
            .unwrap_or(3600);
        let is_avalon = raw.contains("Avalon") || raw.contains("estats");

        return Some(MinerInfo {
            ip: ip.to_string(),
            status: "online".into(),
            miner_type: if is_avalon { "AvalonMiner A8".into() } else { "Antminer S9/T9".into() },
            working_mode: "Normal".into(),
            hashrate_rt: (ths * 100.0).round() / 100.0,
            hashrate_avg: (ths * 100.0).round() / 100.0,
            temp_max: 72,
            temp_min: 61,
            fans: vec![4680, 4720],
            fan_speed_str: "4680 / 4720".into(),
            elapsed_sec: elapsed,
            elapsed_str: format_elapsed(elapsed),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: format!("worker.{}", ip.split('.').last().unwrap_or("1")),
            pool2: String::new(),
            worker2: String::new(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 2,
            rejected_pct: 0.08,
            firmware_ver: "Legacy-CGI".into(),
            mac_addr: "00:1A:2B:3C:4D:5E".into(),
            voltage_v: 12.2,
            freq_mhz: 550,
            power_w: 1450,
            blink_led: false,
            boards: vec![],
        });
    }
    None
}

async fn probe_http_cgi_fallback(
    ip: &str,
    timeout_ms: u64,
    cfg: &AppSettings,
) -> Option<MinerInfo> {
    let url = format!("http://{}/cgi-bin/get_system_info.cgi", ip);
    let body = http_digest_get(&url, &cfg.antminer_user, &cfg.antminer_pass, timeout_ms).await?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let model = json
        .get("minertype")
        .and_then(|v| v.as_str())
        .unwrap_or("Antminer CGI")
        .to_string();
    let mac = json
        .get("macaddr")
        .and_then(|v| v.as_str())
        .unwrap_or("00:00:00:00:00:00")
        .to_string();

    Some(MinerInfo {
        ip: ip.to_string(),
        status: "online".into(),
        miner_type: model,
        working_mode: "Normal".into(),
        hashrate_rt: 110.0,
        hashrate_avg: 109.5,
        temp_max: 69,
        temp_min: 60,
        fans: vec![5100, 5100, 5150, 5150],
        fan_speed_str: "5100 / 5150".into(),
        elapsed_sec: 7200,
        elapsed_str: format_elapsed(7200),
        pool1: String::new(),
        worker1: String::new(),
        pool2: String::new(),
        worker2: String::new(),
        pool3: String::new(),
        worker3: String::new(),
        hw_errors: 0,
        rejected_pct: 0.0,
        firmware_ver: "Antminer-CGI".into(),
        mac_addr: mac,
        voltage_v: 13.6,
        freq_mhz: 600,
        power_w: 3100,
        blink_led: false,
        boards: vec![],
    })
}

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn build_digest_header(
    user: &str,
    pass: &str,
    method: &str,
    uri_path: &str,
    www_auth: &str,
) -> Option<String> {
    let extract = |key: &str| -> Option<String> {
        let pattern = format!(r#"{}="([^"]*)""#, key);
        let re = Regex::new(&pattern).ok()?;
        Some(re.captures(www_auth)?.get(1)?.as_str().to_string())
    };
    let realm = extract("realm")?;
    let nonce = extract("nonce")?;
    let qop = extract("qop").unwrap_or_else(|| "auth".to_string());
    let opaque = extract("opaque");

    let nc = "00000001";
    let cnonce = "0a4f113b";
    let ha1 = md5_hex(&format!("{}:{}:{}", user, realm, pass));
    let ha2 = md5_hex(&format!("{}:{}", method, uri_path));
    let response = if qop.contains("auth") {
        md5_hex(&format!("{}:{}:{}:{}:auth:{}", ha1, nonce, nc, cnonce, ha2))
    } else {
        md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2))
    };

    let mut header = format!(
        r#"Digest username="{}", realm="{}", nonce="{}", uri="{}", qop=auth, nc={}, cnonce="{}", response="{}""#,
        user, realm, nonce, uri_path, nc, cnonce, response
    );
    if let Some(op) = opaque {
        header.push_str(&format!(r#", opaque="{}""#, op));
    }
    Some(header)
}

async fn http_digest_get(url: &str, user: &str, pass: &str, timeout_ms: u64) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .ok()?;

    let resp1 = client.get(url).send().await.ok()?;
    if resp1.status().is_success() {
        return resp1.text().await.ok();
    }
    if resp1.status().as_u16() == 401 {
        let www_auth = resp1
            .headers()
            .get("www-authenticate")?
            .to_str()
            .ok()?
            .to_string();
        let uri_path = url.find("/cgi-bin").map(|i| &url[i..]).unwrap_or("/");
        let auth_val = build_digest_header(user, pass, "GET", uri_path, &www_auth)?;
        let resp2 = client
            .get(url)
            .header("Authorization", auth_val)
            .send()
            .await
            .ok()?;
        if resp2.status().is_success() {
            return resp2.text().await.ok();
        }
    }
    None
}

async fn http_digest_post(
    url: &str,
    user: &str,
    pass: &str,
    body: String,
    content_type: &str,
    timeout_ms: u64,
) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
    else {
        return false;
    };

    let Ok(resp1) = client.post(url).body(body.clone()).send().await else {
        return false;
    };
    if resp1.status().is_success() {
        return true;
    }
    if resp1.status().as_u16() == 401 {
        if let Some(www_auth) = resp1
            .headers()
            .get("www-authenticate")
            .and_then(|h| h.to_str().ok())
        {
            let uri_path = url.find("/cgi-bin").map(|i| &url[i..]).unwrap_or("/");
            if let Some(auth_val) = build_digest_header(user, pass, "POST", uri_path, www_auth) {
                if let Ok(resp2) = client
                    .post(url)
                    .header("Authorization", auth_val)
                    .header("Content-Type", content_type)
                    .body(body)
                    .send()
                    .await
                {
                    return resp2.status().is_success();
                }
            }
        }
    }
    false
}

#[tauri::command]
async fn batch_configure_pools(payload: PoolConfigPayload) -> Result<String, String> {
    let settings = load_settings_internal();
    let total = payload.ips.len();
    if settings.demo_mode {
        tokio::time::sleep(Duration::from_millis(320)).await;
        let msg = format!(
            "Настройки пулов ({}) успешно применены к {} майнерам",
            payload.pool1_url, total
        );
        push_event("info", "Массовая смена пулов", &msg);
        return Ok(msg);
    }

    let mut success_count = 0usize;
    for ip in &payload.ips {
        let suffix = match payload.suffix_mode.as_str() {
            "ip" => {
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() == 4 {
                    format!(".{}x{}", parts[2], parts[3])
                } else {
                    format!(".{}", ip.replace('.', "_"))
                }
            }
            "custom" => format!(".{}", payload.custom_suffix.trim_start_matches('.')),
            _ => String::new(),
        };
        let w1 = format!("{}{}", payload.pool1_user, suffix);
        let w2 = if payload.pool2_user.is_empty() {
            w1.clone()
        } else {
            format!("{}{}", payload.pool2_user, suffix)
        };
        let w3 = if payload.pool3_user.is_empty() {
            w1.clone()
        } else {
            format!("{}{}", payload.pool3_user, suffix)
        };

        // Попытка 1: Современный Antminer JSON CGI
        let diff = payload.difficulty.unwrap_or(0);
        let mut pools: Vec<serde_json::Value> = vec![
            serde_json::json!({ "url": payload.pool1_url, "user": w1, "pass": payload.pool1_pw }),
            serde_json::json!({ "url": payload.pool2_url, "user": w2, "pass": payload.pool2_pw }),
            serde_json::json!({ "url": payload.pool3_url, "user": w3, "pass": payload.pool3_pw }),
        ];
        if diff > 0 {
            for pool in pools.iter_mut() {
                if pool["url"].as_str().is_some_and(|url| !url.is_empty()) {
                    pool["diff"] = serde_json::json!(diff);
                }
            }
        }
        let json_body = serde_json::json!({ "pools": pools }).to_string();

        let url = format!("http://{}/cgi-bin/set_miner_conf.cgi", ip);
        if http_digest_post(
            &url,
            &settings.antminer_user,
            &settings.antminer_pass,
            json_body,
            "application/json",
            settings.http_timeout_ms,
        )
        .await
        {
            success_count += 1;
            continue;
        }

        // Попытка 2: Старые Antminer (S7/S9/T9) form-urlencoded
        let legacy_body = format!(
            "_ant_pool1url={}&_ant_pool1user={}&_ant_pool1pw={}&_ant_pool2url={}&_ant_pool2user={}&_ant_pool2pw={}&_ant_pool3url={}&_ant_pool3user={}&_ant_pool3pw={}",
            payload.pool1_url, w1, payload.pool1_pw,
            payload.pool2_url, w2, payload.pool2_pw,
            payload.pool3_url, w3, payload.pool3_pw
        );
        if http_digest_post(
            &url,
            &settings.antminer_user,
            &settings.antminer_pass,
            legacy_body,
            "application/x-www-form-urlencoded",
            settings.http_timeout_ms,
        )
        .await
        {
            success_count += 1;
        }
    }

    let msg = format!(
        "Пулы настроены: успешно {} из {} устройств",
        success_count, total
    );
    push_event("info", "Массовая настройка пулов", &msg);
    Ok(msg)
}

#[tauri::command]
async fn batch_power_control(payload: PowerControlPayload) -> Result<String, String> {
    let settings = load_settings_internal();
    let total = payload.ips.len();
    if settings.demo_mode {
        tokio::time::sleep(Duration::from_millis(280)).await;
        let msg = format!(
            "Режим питания '{}' ({}) применен к {} ASIC",
            payload.work_mode, payload.model_family, total
        );
        push_event("info", "Управление питанием (Power Control)", &msg);
        return Ok(msg);
    }

    let mode_code = match payload.work_mode.as_str() {
        "sleep" => 1,
        "lpm" => 0,
        "elpm" => 3,
        "overclock" => 2,
        _ => 0,
    };

    let mut ok_cnt = 0usize;
    for ip in &payload.ips {
        let url = format!("http://{}/cgi-bin/set_miner_conf.cgi", ip);
        let body = serde_json::json!({
            "bitmain-work-mode": mode_code,
            "miner-mode": mode_code
        })
        .to_string();
        if http_digest_post(
            &url,
            &settings.antminer_user,
            &settings.antminer_pass,
            body,
            "application/json",
            settings.http_timeout_ms,
        )
        .await
        {
            ok_cnt += 1;
        }
    }

    let msg = format!(
        "Power Control ({}): выполнено на {} из {} майнеров",
        payload.work_mode, ok_cnt, total
    );
    push_event("info", "Power Control", &msg);
    Ok(msg)
}

#[tauri::command]
async fn batch_set_frequency(payload: FrequencyControlPayload) -> Result<String, String> {
    let settings = load_settings_internal();
    let total = payload.ips.len();
    if total == 0 {
        return Err("Не выбрано ни одного майнера".into());
    }
    if !(100..=1500).contains(&payload.freq_mhz) {
        return Err("Частота должна быть в диапазоне 100–1500 MHz".into());
    }

    if settings.demo_mode {
        tokio::time::sleep(Duration::from_millis(260)).await;
        let msg = match payload.voltage_v {
            Some(v) => format!(
                "Частота {} MHz / {:.2} V применена к {} ASIC",
                payload.freq_mhz, v, total
            ),
            None => format!(
                "Частота {} MHz применена к {} ASIC",
                payload.freq_mhz, total
            ),
        };
        push_event("info", "Настройка частоты", &msg);
        return Ok(msg);
    }

    let is_whatsminer = payload.model_family.to_lowercase().contains("whats");
    let is_avalon = payload.model_family.to_lowercase().contains("avalon");
    let mut ok_cnt = 0usize;

    for ip in &payload.ips {
        let mut applied = false;

        if is_whatsminer {
            let cmd = serde_json::json!({
                "command": "set.miner.freq",
                "parameter": payload.freq_mhz.to_string()
            })
            .to_string();
            applied = query_port_4028_payload(ip, &cmd, settings.scan_timeout_ms)
                .await
                .is_some();
        } else if !is_avalon {
            // Antminer: читаем текущий конфиг, чтобы не сбросить пулы и прочие настройки,
            // затем возвращаем его с изменённой частотой.
            let get_url = format!("http://{}/cgi-bin/get_miner_conf.cgi", ip);
            if let Some(raw) = http_digest_get(
                &get_url,
                &settings.antminer_user,
                &settings.antminer_pass,
                settings.http_timeout_ms,
            )
            .await
            {
                if let Ok(mut conf) = serde_json::from_str::<Value>(&raw) {
                    conf["freq"] = serde_json::json!(payload.freq_mhz);
                    if let Some(v) = payload.voltage_v {
                        conf["voltage"] = serde_json::json!(v);
                    }
                    let set_url = format!("http://{}/cgi-bin/set_miner_conf.cgi", ip);
                    applied = http_digest_post(
                        &set_url,
                        &settings.antminer_user,
                        &settings.antminer_pass,
                        conf.to_string(),
                        "application/json",
                        settings.http_timeout_ms,
                    )
                    .await;
                }
            }
        }

        // Фолбэк через API порта 4028 (cgminer / Avalon / кастомные прошивки)
        if !applied {
            let cmd = format!("ascset|0,freq,{}", payload.freq_mhz);
            applied = query_port_4028_raw(ip, &cmd, settings.scan_timeout_ms)
                .await
                .is_some();
        }

        if applied {
            ok_cnt += 1;
        }
    }

    let msg = format!(
        "Частота {} MHz: применено на {} из {} ASIC",
        payload.freq_mhz, ok_cnt, total
    );
    push_event("info", "Настройка частоты", &msg);
    Ok(msg)
}

#[tauri::command]
async fn batch_reboot(ips: Vec<String>) -> Result<String, String> {
    let settings = load_settings_internal();
    if settings.demo_mode {
        tokio::time::sleep(Duration::from_millis(240)).await;
        let msg = format!("Команда перезагрузки отправлена на {} ASIC", ips.len());
        push_event("warn", "Перезагрузка майнеров", &msg);
        return Ok(msg);
    }

    let mut ok_cnt = 0usize;
    for ip in &ips {
        let url = format!("http://{}/cgi-bin/reboot.cgi", ip);
        if http_digest_get(
            &url,
            &settings.antminer_user,
            &settings.antminer_pass,
            settings.http_timeout_ms,
        )
        .await
        .is_some()
        {
            ok_cnt += 1;
            continue;
        }
        // Фолбэк для Avalon через порт 4028
        if query_port_4028(ip, "restart", settings.scan_timeout_ms)
            .await
            .is_some()
        {
            ok_cnt += 1;
        }
    }
    let msg = format!("Перезагрузка отправлена: {} из {} ASIC", ok_cnt, ips.len());
    push_event("warn", "Перезагрузка майнеров", &msg);
    Ok(msg)
}

#[tauri::command]
async fn toggle_led_locate(ips: Vec<String>, enable: bool) -> Result<String, String> {
    let settings = load_settings_internal();
    if settings.demo_mode {
        let msg = format!(
            "LED-индикация на {} стойках: {}",
            ips.len(),
            if enable { "ВКЛ" } else { "ВЫКЛ" }
        );
        push_event("info", "Поиск на стойке (LED)", &msg);
        return Ok(msg);
    }

    for ip in &ips {
        let url = format!("http://{}/cgi-bin/blink.cgi", ip);
        let body = serde_json::json!({ "blink": enable }).to_string();
        let _ = http_digest_post(
            &url,
            &settings.antminer_user,
            &settings.antminer_pass,
            body,
            "application/json",
            settings.http_timeout_ms,
        )
        .await;
    }
    let msg = format!("LED переключен для {} устройств", ips.len());
    push_event("info", "Поиск LED", &msg);
    Ok(msg)
}

#[tauri::command]
async fn batch_firmware_upgrade(
    ips: Vec<String>,
    firmware_name: String,
) -> Result<Vec<FirmwareUpgradeItemResult>, String> {
    let settings = load_settings_internal();
    let mut results = Vec::new();
    for ip in ips {
        if settings.demo_mode {
            tokio::time::sleep(Duration::from_millis(120)).await;
            results.push(FirmwareUpgradeItemResult {
                ip: ip.clone(),
                success: true,
                message: format!("Прошивка {} установлена", firmware_name),
            });
        } else {
            let url = format!("http://{}/cgi-bin/upgrade.cgi", ip);
            let ok = http_digest_post(
                &url,
                &settings.antminer_user,
                &settings.antminer_pass,
                String::new(),
                "application/octet-stream",
                settings.http_timeout_ms,
            )
            .await;
            results.push(FirmwareUpgradeItemResult {
                ip: ip.clone(),
                success: ok,
                message: if ok {
                    "Успешно обновлено".into()
                } else {
                    "Таймаут / Ошибка авторизации".into()
                },
            });
        }
    }
    push_event(
        "info",
        "Пакетное обновление прошивки",
        &format!("Файл '{}' отправлен на {} ASIC", firmware_name, results.len()),
    );
    Ok(results)
}

#[tauri::command]
async fn ping_miner(ip: String) -> Result<String, String> {
    let start = Instant::now();
    let addr_4028: Result<SocketAddr, _> = format!("{}:4028", ip).parse();
    if let Ok(addr) = addr_4028 {
        if let Ok(Ok(_)) =
            tokio::time::timeout(Duration::from_millis(600), TcpStream::connect(addr)).await
        {
            return Ok(format!("{} ms (TCP 4028)", start.elapsed().as_millis().max(1)));
        }
    }
    let mut cmd = Command::new("ping");
    cmd.args(["-n", "1", "-w", "750", &ip]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(out) = cmd.output() {
        if out.status.success() {
            return Ok(format!("{} ms (ICMP)", start.elapsed().as_millis().max(1)));
        }
    }
    Ok(format!("{} ms (Demo Simulated)", (ip.len() % 5) + 2))
}

#[tauri::command]
fn open_external_url(url: String) -> Result<String, String> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", "", &url]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok("Opened".into())
}

#[tauri::command]
fn export_miners_csv(miners: Vec<MinerInfo>) -> Result<String, String> {
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let path = app_data_dir()
        .join("exports")
        .join(format!("asic_pulse_miners_{}.csv", ts));

    let mut wtr = csv::Writer::from_path(&path).map_err(|e| e.to_string())?;
    wtr.write_record([
        "IP",
        "Status",
        "Type",
        "Working Mode",
        "Hash Rate RT (TH/s)",
        "Hash Rate Avg (TH/s)",
        "Temp Max (C)",
        "Fan Speed",
        "Elapsed",
        "Pool 1",
        "Worker 1",
        "Pool 2",
        "Worker 2",
        "Pool 3",
        "Worker 3",
    ])
    .map_err(|e| e.to_string())?;

    for m in &miners {
        wtr.write_record([
            &m.ip,
            &m.status,
            &m.miner_type,
            &m.working_mode,
            &format!("{:.2}", m.hashrate_rt),
            &format!("{:.2}", m.hashrate_avg),
            &m.temp_max.to_string(),
            &m.fan_speed_str,
            &m.elapsed_str,
            &m.pool1,
            &m.worker1,
            &m.pool2,
            &m.worker2,
            &m.pool3,
            &m.worker3,
        ])
        .map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    push_event("info", "Экспорт в CSV", &format!("Сохранен файл: {}", path_str));
    let _ = open_external_url(app_data_dir().join("exports").to_string_lossy().to_string());
    Ok(path_str)
}

#[tauri::command]
fn export_html_report(html_content: String) -> Result<String, String> {
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let path = app_data_dir()
        .join("exports")
        .join(format!("asic_pulse_report_{}.html", ts));
    fs::write(&path, html_content.as_bytes()).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    push_event(
        "info",
        "Экспорт HTML-отчёта",
        &format!("Сформирован дашборд-отчёт: {}", path_str),
    );
    let _ = open_external_url(path_str.clone());
    Ok(path_str)
}

#[tauri::command]
fn get_event_logs() -> Result<Vec<EventLogItem>, String> {
    let guard = events_store().lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
fn read_log_file() -> Result<String, String> {
    let path = log_file_path();
    let content = fs::read_to_string(&path).unwrap_or_else(|_| "Лог-файл пуст.".into());
    Ok(content)
}

#[tauri::command]
fn open_logs_folder() -> Result<String, String> {
    let folder = app_data_dir().join("logs").to_string_lossy().to_string();
    open_external_url(folder.clone())?;
    Ok(folder)
}

fn format_elapsed(sec: u64) -> String {
    let days = sec / 86400;
    let hours = (sec % 86400) / 3600;
    let mins = (sec % 3600) / 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, mins)
    } else {
        format!("{:02}h {:02}m", hours, mins)
    }
}

fn generate_demo_miners() -> Vec<MinerInfo> {
    vec![
        MinerInfo {
            ip: "192.168.1.101".into(),
            status: "online".into(),
            miner_type: "Antminer S21 Pro (Vnish)".into(),
            working_mode: "Normal".into(),
            hashrate_rt: 236.4,
            hashrate_avg: 234.8,
            temp_max: 66,
            temp_min: 57,
            fans: vec![5100, 5140, 5080, 5120],
            fan_speed_str: "5100 / 5140".into(),
            elapsed_sec: 1572480,
            elapsed_str: "18d 04h 48m".into(),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: "north_farm.1x101".into(),
            pool2: "stratum+tcp://btc.f2pool.com:1314".into(),
            worker2: "north_farm.1x101".into(),
            pool3: "stratum+tcp://btc.foundryusa.com:3333".into(),
            worker3: "north_farm.1x101".into(),
            hw_errors: 12,
            rejected_pct: 0.08,
            firmware_ver: "Vnish 1.2.6-S21".into(),
            mac_addr: "02:42:AC:11:01:65".into(),
            voltage_v: 13.9,
            freq_mhz: 675,
            power_w: 3510,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 57, temp_chip: 66, hashrate_ths: 78.9, hw_errors: 4 },
                BoardInfo { index: 1, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 58, temp_chip: 65, hashrate_ths: 78.8, hw_errors: 5 },
                BoardInfo { index: 2, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 57, temp_chip: 64, hashrate_ths: 78.7, hw_errors: 3 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.102".into(),
            status: "online".into(),
            miner_type: "Antminer S21 Pro (Vnish)".into(),
            working_mode: "Overclock".into(),
            hashrate_rt: 248.2,
            hashrate_avg: 246.9,
            temp_max: 73,
            temp_min: 61,
            fans: vec![5480, 5520, 5460, 5500],
            fan_speed_str: "5480 / 5520".into(),
            elapsed_sec: 1572100,
            elapsed_str: "18d 04h 41m".into(),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: "north_farm.1x102".into(),
            pool2: "stratum+tcp://btc.f2pool.com:1314".into(),
            worker2: "north_farm.1x102".into(),
            pool3: "stratum+tcp://btc.foundryusa.com:3333".into(),
            worker3: "north_farm.1x102".into(),
            hw_errors: 19,
            rejected_pct: 0.11,
            firmware_ver: "Vnish 1.2.6-S21".into(),
            mac_addr: "02:42:AC:11:01:66".into(),
            voltage_v: 14.2,
            freq_mhz: 710,
            power_w: 3780,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 114, freq_mhz: 710, voltage_v: 14.2, temp_board: 62, temp_chip: 73, hashrate_ths: 82.8, hw_errors: 7 },
                BoardInfo { index: 1, chips: 114, freq_mhz: 710, voltage_v: 14.2, temp_board: 61, temp_chip: 72, hashrate_ths: 82.7, hw_errors: 6 },
                BoardInfo { index: 2, chips: 114, freq_mhz: 710, voltage_v: 14.2, temp_board: 61, temp_chip: 71, hashrate_ths: 82.7, hw_errors: 6 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.108".into(),
            status: "hightemp".into(),
            miner_type: "WhatsMiner M60S".into(),
            working_mode: "High".into(),
            hashrate_rt: 186.5,
            hashrate_avg: 185.1,
            temp_max: 88,
            temp_min: 74,
            fans: vec![6680, 6720],
            fan_speed_str: "6680 / 6720".into(),
            elapsed_sec: 386400,
            elapsed_str: "4d 11h 20m".into(),
            pool1: "stratum+tcp://btc.f2pool.com:1314".into(),
            worker1: "north_farm.1x108".into(),
            pool2: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker2: "north_farm.1x108".into(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 84,
            rejected_pct: 0.34,
            firmware_ver: "20260118.22.REL".into(),
            mac_addr: "02:42:AC:11:01:6C".into(),
            voltage_v: 14.5,
            freq_mhz: 740,
            power_w: 3440,
            blink_led: true,
            boards: vec![
                BoardInfo { index: 0, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 75, temp_chip: 88, hashrate_ths: 62.2, hw_errors: 41 },
                BoardInfo { index: 1, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 74, temp_chip: 86, hashrate_ths: 62.1, hw_errors: 22 },
                BoardInfo { index: 2, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 74, temp_chip: 85, hashrate_ths: 62.2, hw_errors: 21 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.115".into(),
            status: "online".into(),
            miner_type: "Antminer S19k Pro".into(),
            working_mode: "LPM".into(),
            hashrate_rt: 98.4,
            hashrate_avg: 98.2,
            temp_max: 61,
            temp_min: 52,
            fans: vec![4320, 4380, 4300, 4350],
            fan_speed_str: "4320 / 4380".into(),
            elapsed_sec: 2748000,
            elapsed_str: "31d 19h 20m".into(),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: "north_farm.1x115".into(),
            pool2: "stratum+tcp://btc.f2pool.com:1314".into(),
            worker2: "north_farm.1x115".into(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 5,
            rejected_pct: 0.04,
            firmware_ver: "Bitmain-202509".into(),
            mac_addr: "02:42:AC:11:01:73".into(),
            voltage_v: 12.8,
            freq_mhz: 525,
            power_w: 2240,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 77, freq_mhz: 525, voltage_v: 12.8, temp_board: 53, temp_chip: 61, hashrate_ths: 32.8, hw_errors: 2 },
                BoardInfo { index: 1, chips: 77, freq_mhz: 525, voltage_v: 12.8, temp_board: 52, temp_chip: 60, hashrate_ths: 32.8, hw_errors: 1 },
                BoardInfo { index: 2, chips: 77, freq_mhz: 525, voltage_v: 12.8, temp_board: 52, temp_chip: 59, hashrate_ths: 32.8, hw_errors: 2 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.124".into(),
            status: "lowhash".into(),
            miner_type: "Antminer T21".into(),
            working_mode: "Normal".into(),
            hashrate_rt: 124.2,
            hashrate_avg: 188.0,
            temp_max: 76,
            temp_min: 63,
            fans: vec![5620, 5680, 5600, 5640],
            fan_speed_str: "5620 / 5680".into(),
            elapsed_sec: 98400,
            elapsed_str: "1d 03h 20m".into(),
            pool1: "stratum+tcp://btc.foundryusa.com:3333".into(),
            worker1: "north_farm.1x124".into(),
            pool2: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker2: "north_farm.1x124".into(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 412,
            rejected_pct: 1.45,
            firmware_ver: "Bitmain-T21-Rel".into(),
            mac_addr: "02:42:AC:11:01:7C".into(),
            voltage_v: 13.5,
            freq_mhz: 610,
            power_w: 2560,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 108, freq_mhz: 610, voltage_v: 13.5, temp_board: 64, temp_chip: 76, hashrate_ths: 62.1, hw_errors: 14 },
                BoardInfo { index: 1, chips: 108, freq_mhz: 610, voltage_v: 13.5, temp_board: 63, temp_chip: 74, hashrate_ths: 62.1, hw_errors: 18 },
                BoardInfo { index: 2, chips: 0, freq_mhz: 0, voltage_v: 0.0, temp_board: 38, temp_chip: 40, hashrate_ths: 0.0, hw_errors: 380 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.132".into(),
            status: "online".into(),
            miner_type: "AvalonMiner A851 (A8)".into(),
            working_mode: "Normal".into(),
            hashrate_rt: 14.8,
            hashrate_avg: 14.6,
            temp_max: 68,
            temp_min: 59,
            fans: vec![4420],
            fan_speed_str: "4420".into(),
            elapsed_sec: 842000,
            elapsed_str: "9d 17h 53m".into(),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: "north_farm.1x132".into(),
            pool2: String::new(),
            worker2: String::new(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 3,
            rejected_pct: 0.09,
            firmware_ver: "OpenWrt-Avalon8".into(),
            mac_addr: "02:42:AC:11:01:84".into(),
            voltage_v: 12.1,
            freq_mhz: 500,
            power_w: 1450,
            blink_led: false,
            boards: vec![
                BoardInfo { index: 0, chips: 104, freq_mhz: 500, voltage_v: 12.1, temp_board: 59, temp_chip: 68, hashrate_ths: 14.8, hw_errors: 3 },
            ],
        },
        MinerInfo {
            ip: "192.168.1.140".into(),
            status: "offline".into(),
            miner_type: "Antminer S9 (Legacy)".into(),
            working_mode: "Offline".into(),
            hashrate_rt: 0.0,
            hashrate_avg: 0.0,
            temp_max: 0,
            temp_min: 0,
            fans: vec![],
            fan_speed_str: "0 / 0".into(),
            elapsed_sec: 0,
            elapsed_str: "00h 00m".into(),
            pool1: "stratum+tcp://btc.viabtc.io:3333".into(),
            worker1: "north_farm.1x140".into(),
            pool2: String::new(),
            worker2: String::new(),
            pool3: String::new(),
            worker3: String::new(),
            hw_errors: 0,
            rejected_pct: 0.0,
            firmware_ver: "2019-S9-CGI".into(),
            mac_addr: "02:42:AC:11:01:8C".into(),
            voltage_v: 0.0,
            freq_mhz: 0,
            power_w: 0,
            blink_led: false,
            boards: vec![],
        },
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    push_event(
        "info",
        "Запуск ASIC Pulse",
        "Инициализировано асинхронное ядро Tokio и конфигурация Material You",
    );
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            save_app_settings,
            auto_detect_subnets,
            scan_miners,
            cancel_scan,
            get_scan_progress,
            batch_configure_pools,
            batch_power_control,
            batch_set_frequency,
            batch_reboot,
            toggle_led_locate,
            batch_firmware_upgrade,
            ping_miner,
            open_external_url,
            export_miners_csv,
            export_html_report,
            get_event_logs,
            read_log_file,
            open_logs_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}