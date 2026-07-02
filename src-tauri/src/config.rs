use std::path::{Path, PathBuf};

use crate::types::{DownloadFormat, ProxyMode};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

// 2025-07: Hardcoded domains as fallback.
// On startup, JmClient tries to fetch latest domains from BytePlus servers.
// If fetch succeeds, these hardcoded domains are replaced.
// If fetch fails (network down, server dead), these fallback domains are used.
// Note: these domains may become stale over time - the auto-update mechanism handles that.
const FALLBACK_API_DOMAINS: &[&str] = &[
    "www.cdnhjk.net",
    "www.cdngwc.cc",
    "www.cdngwc.net",
    "www.cdngwc.club",
    "www.cdnutc.me",
];

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub username: String,
    pub password: String,
    pub download_dir: PathBuf,
    pub export_dir: PathBuf,
    pub download_format: DownloadFormat,
    pub dir_fmt: String,
    pub proxy_mode: ProxyMode,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub enable_file_logger: bool,
    pub chapter_concurrency: usize,
    pub chapter_download_interval_sec: u64,
    pub img_concurrency: usize,
    pub img_download_interval_sec: u64,
    pub download_all_favorites_interval_sec: u64,
    pub update_downloaded_comics_interval_sec: u64,
    pub api_domain_mode: ApiDomainMode,
    pub custom_api_domain: String,
    pub should_download_cover: bool,
    /// Runtime-updated API domain list (set by auto-update on startup)
    #[serde(skip, default = "default_api_domains")]
    pub api_domains: Vec<String>,
}

fn default_api_domains() -> Vec<String> {
    FALLBACK_API_DOMAINS.iter().map(|s| s.to_string()).collect()
}

impl Config {
    pub fn new(app: &AppHandle) -> anyhow::Result<Self> {
        let app_data_dir = app.path().app_data_dir()?;
        let config_path = app_data_dir.join("config.json");

        let config = if config_path.exists() {
            let config_string = std::fs::read_to_string(config_path)?;
            match serde_json::from_str(&config_string) {
                // 如果能够直接解析为Config，则直接返回
                Ok(config) => config,
                // 否则，将默认配置与文件中已有的配置合并
                // 以免新版本添加了新的配置项，用户升级到新版本后，所有配置项都被重置
                Err(_) => Config::merge_config(&config_string, &app_data_dir),
            }
        } else {
            Config::default(&app_data_dir)
        };
        config.save(app)?;
        Ok(config)
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let resource_dir = app.path().app_data_dir()?;
        let config_path = resource_dir.join("config.json");
        // Don't save runtime-updated api_domains to config file
        let mut saveable = self.clone();
        saveable.api_domains = FALLBACK_API_DOMAINS.iter().map(|s| s.to_string()).collect();
        let config_string = serde_json::to_string_pretty(&saveable)?;
        std::fs::write(config_path, config_string)?;
        Ok(())
    }

    pub fn get_api_domain(&self) -> String {
        // Use runtime-updated domains if available, otherwise fall back to hardcoded list
        let domains: &Vec<String> = if !self.api_domains.is_empty() {
            &self.api_domains
        } else {
            return FALLBACK_API_DOMAINS[1].to_string();
        };
        
        match self.api_domain_mode {
            ApiDomainMode::Domain1 => domains.first().cloned().unwrap_or_default(),
            ApiDomainMode::Domain2 => domains.get(1).cloned().unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain3 => domains.get(2).cloned().unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain4 => domains.get(3).cloned().unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain5 => domains.get(4).cloned().unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Custom => self.custom_api_domain.clone(),
        }
    }

    fn merge_config(config_string: &str, app_data_dir: &Path) -> Config {
        let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(config_string) else {
            return Config::default(app_data_dir);
        };
        let serde_json::Value::Object(ref mut map) = json_value else {
            return Config::default(app_data_dir);
        };
        let Ok(default_config_value) = serde_json::to_value(Config::default(app_data_dir)) else {
            return Config::default(app_data_dir);
        };
        let serde_json::Value::Object(default_map) = default_config_value else {
            return Config::default(app_data_dir);
        };
        for (key, value) in default_map {
            map.entry(key).or_insert(value);
        }
        let Ok(config) = serde_json::from_value(json_value) else {
            return Config::default(app_data_dir);
        };
        config
    }

    fn default(app_data_dir: &Path) -> Config {
        Config {
            username: String::new(),
            password: String::new(),
            download_dir: app_data_dir.join("漫画下载"),
            export_dir: app_data_dir.join("漫画导出"),
            download_format: DownloadFormat::default(),
            dir_fmt: "{comic_title}/{chapter_title}".to_string(),
            proxy_mode: ProxyMode::default(),
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 7890,
            enable_file_logger: true,
            chapter_concurrency: 3,
            chapter_download_interval_sec: 0,
            img_concurrency: 20,
            img_download_interval_sec: 0,
            download_all_favorites_interval_sec: 0,
            update_downloaded_comics_interval_sec: 0,
            api_domain_mode: ApiDomainMode::Domain2,
            custom_api_domain: FALLBACK_API_DOMAINS[1].to_string(),
            should_download_cover: true,
            api_domains: default_api_domains(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum ApiDomainMode {
    Domain1,
    #[default]
    Domain2,
    Domain3,
    Domain4,
    Domain5,
    Custom,
}
