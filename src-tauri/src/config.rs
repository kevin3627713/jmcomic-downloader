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
        let root = crate::storage::config_dir(app)?;
        let documents = if cfg!(target_os = "ios") {
            Some(crate::storage::data_dir(app)?)
        } else {
            None
        };
        let (config, warnings) = Self::load_with_warnings(&root, documents.as_deref())?;
        app.manage(crate::storage::MigrationWarnings(warnings));
        Ok(config)
    }

    #[cfg(test)]
    fn load(root: &Path, documents: Option<&Path>) -> anyhow::Result<Self> {
        Ok(Self::load_with_warnings(root, documents)?.0)
    }

    fn load_with_warnings(
        root: &Path,
        documents: Option<&Path>,
    ) -> anyhow::Result<(Self, Vec<String>)> {
        let mut warnings = Vec::new();
        std::fs::create_dir_all(root)?;
        let read_config = |path: &Path| -> anyhow::Result<Option<String>> {
            match std::fs::read_to_string(path) {
                Ok(text) => Ok(Some(text)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error)
                    .map_err(anyhow::Error::from)
                    .map_err(|error| error.context(format!("读取配置 `{}` 失败", path.display()))),
            }
        };
        let mut original = read_config(&root.join("config.json"))?;
        let mut imported_documents_config = false;
        if original.is_none() {
            if let Some(documents) = documents {
                original = read_config(&documents.join("config.json"))?;
                imported_documents_config = original.is_some();
            }
        }
        let media_root = documents.unwrap_or(root);
        let mut config: Self = if let Some(config_string) = &original {
            match serde_json::from_str(&config_string) {
                // 如果能够直接解析为Config，则直接返回
                Ok(config) => config,
                // 否则，将默认配置与文件中已有的配置合并
                // 以免新版本添加了新的配置项，用户升级到新版本后，所有配置项都被重置
                Err(_) => Config::merge_config(&config_string, media_root),
            }
        } else {
            Config::default(media_root)
        };
        if let Some(documents) = documents {
            // Keep settings, but migrate media only from this active guest.
            let mut sources = vec![PathBuf::from("漫画下载"), PathBuf::from("漫画导出")];
            let mut needs_backup = imported_documents_config;
            for stored in [&config.download_dir, &config.export_dir] {
                if let Some(suffix) = crate::storage::legacy_relative(stored, root) {
                    needs_backup = true;
                    // Reject traversal before inspecting or moving anything.
                    crate::storage::documents_path(stored, documents, root)?;
                    if !sources.contains(&suffix) {
                        sources.push(suffix);
                    }
                }
            }
            config.resolve_documents(documents, root)?;
            if let Some(original) = original.filter(|_| needs_backup) {
                use std::io::Write;
                let backup = root.join("config.before-documents-migration.json");
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(backup)
                {
                    Ok(mut file) => {
                        file.write_all(original.as_bytes())?;
                        file.sync_all()?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(error) => return Err(error.into()),
                }
            }
            for suffix in sources {
                if let Err(error) =
                    crate::storage::migrate_directory(&root.join(&suffix), &documents.join(&suffix))
                {
                    // Keep both conflicting files and allow access to Documents.
                    warnings.push(format!("旧文件迁移未完成：{error:#}"));
                }
            }
            crate::storage::prepare_directory(&config.download_dir)?;
            crate::storage::prepare_directory(&config.export_dir)?;
        }
        config.save_in(root, documents)?;
        Ok((config, warnings))
    }

    pub fn resolve_documents(&mut self, root: &Path, legacy: &Path) -> anyhow::Result<()> {
        self.download_dir = crate::storage::documents_path(&self.download_dir, root, legacy)?;
        self.export_dir = crate::storage::documents_path(&self.export_dir, root, legacy)?;
        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let documents = if cfg!(target_os = "ios") {
            Some(crate::storage::data_dir(app)?)
        } else {
            None
        };
        self.save_in(&crate::storage::config_dir(app)?, documents.as_deref())
    }

    fn save_in(&self, root: &Path, documents: Option<&Path>) -> anyhow::Result<()> {
        let config_path = root.join("config.json");
        // Don't save runtime-updated api_domains to config file
        let mut saveable = self.clone();
        if let Some(documents) = documents {
            saveable.download_dir = crate::storage::saved_path(&self.download_dir, documents)?;
            saveable.export_dir = crate::storage::saved_path(&self.export_dir, documents)?;
        }
        saveable.api_domains = FALLBACK_API_DOMAINS.iter().map(|s| s.to_string()).collect();
        let config_string = serde_json::to_string_pretty(&saveable)?;
        crate::utils::write_atomic(&config_path, config_string.as_bytes())?;
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
            ApiDomainMode::Domain2 => domains
                .get(1)
                .cloned()
                .unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain3 => domains
                .get(2)
                .cloned()
                .unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain4 => domains
                .get(3)
                .cloned()
                .unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
            ApiDomainMode::Domain5 => domains
                .get(4)
                .cloned()
                .unwrap_or_else(|| domains.first().cloned().unwrap_or_default()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChapterInfo, Comic};
    use std::fs;

    fn roots(home: &Path) -> (PathBuf, PathBuf) {
        (
            home.join("Documents"),
            home.join("Library/Application Support/com.lanyeeee.jmcomic-downloader"),
        )
    }

    fn fake_book(download: &Path) -> PathBuf {
        let book = download.join("fictional book");
        fs::create_dir_all(book.join("chapter")).unwrap();
        let comic = Comic {
            id: -90010,
            name: "fictional book".into(),
            chapter_infos: vec![ChapterInfo {
                chapter_id: -90011,
                chapter_title: "chapter".into(),
                order: 1,
                page_count: None,
                is_downloaded: None,
                chapter_download_dir: None,
            }],
            ..Default::default()
        };
        fs::write(
            book.join("元数据.json"),
            serde_json::to_vec(&comic).unwrap(),
        )
        .unwrap();
        fs::write(
            book.join("chapter/章节元数据.json"),
            br#"{"chapterId":-90011,"pageCount":1}"#,
        )
        .unwrap();
        fs::write(
            book.join("chapter/0001.jpg"),
            b"fictional bytes; never rendered",
        )
        .unwrap();
        book
    }

    #[test]
    fn ios_moves_existing_library_and_exports_to_documents_and_keeps_settings() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, legacy) = roots(temp.path());
        fs::create_dir_all(&legacy).unwrap();
        let old_saved_root = Path::new("/private/var/mobile/Containers/Data/Application/old-host/Documents/Data/Application/old-guest/Library/Application Support/com.lanyeeee.jmcomic-downloader");
        let mut old = Config::default(old_saved_root);
        old.proxy_port = 8123;
        let original = serde_json::to_vec(&old).unwrap();
        fs::write(legacy.join("config.json"), &original).unwrap();
        fake_book(&legacy.join("漫画下载"));
        let export = legacy.join("漫画导出/fictional book");
        fs::create_dir_all(export.join("cbz")).unwrap();
        fs::write(export.join("fictional book.pdf"), b"fictional pdf").unwrap();
        fs::write(export.join("cbz/chapter.cbz"), b"fictional cbz").unwrap();
        let loaded = Config::load(&legacy, Some(&documents)).unwrap();
        assert_eq!(loaded.download_dir, documents.join("漫画下载"));
        assert_eq!(loaded.export_dir, documents.join("漫画导出"));
        assert_eq!(loaded.proxy_port, 8123);
        assert!(!legacy.join("漫画下载").exists());
        assert!(!legacy.join("漫画导出").exists());
        assert_eq!(
            fs::read(legacy.join("config.before-documents-migration.json")).unwrap(),
            original
        );
        let disk: serde_json::Value =
            serde_json::from_slice(&fs::read(legacy.join("config.json")).unwrap()).unwrap();
        assert_eq!(disk["downloadDir"], "漫画下载");
        assert_eq!(disk["exportDir"], "漫画导出");
        let paths = crate::storage::metadata_paths(&loaded.download_dir).unwrap();
        let comic = Comic::from_metadata(&paths[0]).unwrap();
        assert_eq!(
            comic.comic_download_dir,
            Some(documents.join("漫画下载/fictional book"))
        );
        assert_eq!(
            comic.chapter_infos[0].chapter_download_dir,
            Some(documents.join("漫画下载/fictional book/chapter"))
        );
        assert_eq!(comic.chapter_infos[0].is_downloaded, Some(true));
        let files =
            crate::library::file_status(&loaded.download_dir, &loaded.export_dir, &comic).unwrap();
        assert!(files.has_pdf);
        assert_eq!(files.cbz_count, 1);
        // Settings remain in Application Support; media stays in Documents.
        assert!(!documents.join("config.json").exists());
        assert_eq!(
            Config::load(&legacy, Some(&documents)).unwrap().proxy_port,
            8123
        );
    }

    #[test]
    fn support_config_rebases_media_and_survives_entire_container_move() {
        let temp = tempfile::tempdir().unwrap();
        let (first, legacy_first) = roots(
            &temp
                .path()
                .join("host-one/Documents/Data/Application/guest-one"),
        );
        let (second, legacy_second) = roots(
            &temp
                .path()
                .join("host-two/Documents/Data/Application/guest-two"),
        );
        let mut config = Config::load(&legacy_first, Some(&first)).unwrap();
        config.proxy_port = 8124;
        config.save_in(&legacy_first, Some(&first)).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::create_dir_all(&legacy_second).unwrap();
        fs::copy(
            legacy_first.join("config.json"),
            legacy_second.join("config.json"),
        )
        .unwrap();
        Config::default(&second)
            .save_in(&second, Some(&second))
            .unwrap();
        let book = fake_book(&second.join("漫画下载"));
        let loaded = Config::load(&legacy_second, Some(&second)).unwrap();
        assert_eq!(loaded.proxy_port, 8124);
        assert_eq!(loaded.download_dir, second.join("漫画下载"));
        assert_eq!(
            crate::storage::metadata_paths(&loaded.download_dir).unwrap(),
            vec![book.join("元数据.json")]
        );
    }

    #[test]
    fn missing_legacy_media_still_creates_writable_documents_and_never_uses_saved_uuid() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, legacy) = roots(temp.path());
        fs::create_dir_all(&legacy).unwrap();
        Config::default(Path::new(
            "/old/Library/Application Support/com.lanyeeee.jmcomic-downloader",
        ))
        .save_in(&legacy, None)
        .unwrap();
        let config = Config::load(&legacy, Some(&documents)).unwrap();
        fs::write(config.download_dir.join("fictional write probe"), b"test").unwrap();
        assert!(config.export_dir.is_dir());
        assert!(crate::storage::metadata_paths(&config.download_dir)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn desktop_keeps_custom_absolute_directories() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::default(temp.path());
        config.download_dir = temp.path().join("custom downloads");
        config.export_dir = temp.path().join("custom exports");
        config.save_in(temp.path(), None).unwrap();
        let loaded = Config::load(temp.path(), None).unwrap();
        assert_eq!(loaded.download_dir, config.download_dir);
        assert_eq!(loaded.export_dir, config.export_dir);
    }

    #[test]
    fn unsafe_relative_path_is_rejected_before_migration_or_config_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, legacy) = roots(temp.path());
        fs::create_dir_all(&legacy).unwrap();
        let mut config = Config::default(&documents);
        config.download_dir = PathBuf::from("../other-container");
        let original = serde_json::to_vec(&config).unwrap();
        fs::write(legacy.join("config.json"), &original).unwrap();
        assert!(Config::load(&legacy, Some(&documents)).is_err());
        assert_eq!(fs::read(legacy.join("config.json")).unwrap(), original);
    }

    #[test]
    fn conflicting_old_export_is_preserved_and_reported_without_blocking_documents() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, legacy) = roots(temp.path());
        fs::create_dir_all(legacy.join("漫画导出/book")).unwrap();
        fs::create_dir_all(documents.join("漫画导出/book")).unwrap();
        let old = legacy.join("漫画导出/book/book.pdf");
        let new = documents.join("漫画导出/book/book.pdf");
        fs::write(&old, b"fictional old pdf").unwrap();
        fs::write(&new, b"fictional new pdf").unwrap();
        let (config, warnings) = Config::load_with_warnings(&legacy, Some(&documents)).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("同名文件"));
        assert_eq!(fs::read(old).unwrap(), b"fictional old pdf");
        assert_eq!(fs::read(new).unwrap(), b"fictional new pdf");
        assert!(config.download_dir.is_dir());
        assert!(legacy.join("config.json").is_file());
    }

    fn fake_network_profile(root: &Path) -> Config {
        let mut config = Config::default(root);
        config.username = "fictional user".into();
        config.password = "fictional password".into();
        config.proxy_mode = ProxyMode::Custom;
        config.proxy_host = "127.0.0.1".into();
        config.proxy_port = 8123;
        config.api_domain_mode = ApiDomainMode::Custom;
        config.custom_api_domain = "working.example.invalid".into();
        config
    }

    fn assert_same_settings(actual: &Config, expected: &Config) {
        let settings = |config: &Config| {
            let mut value = serde_json::to_value(config).unwrap();
            value.as_object_mut().unwrap().remove("downloadDir");
            value.as_object_mut().unwrap().remove("exportDir");
            value
        };
        assert_eq!(settings(actual), settings(expected));
        assert_eq!(actual.get_api_domain(), expected.get_api_domain());
    }

    #[test]
    fn original_support_settings_take_priority_over_documents_settings() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, support) = roots(temp.path());
        fs::create_dir_all(&support).unwrap();
        fs::create_dir_all(&documents).unwrap();
        let original = fake_network_profile(&support);
        original.save_in(&support, None).unwrap();
        Config::default(&documents)
            .save_in(&documents, Some(&documents))
            .unwrap();
        let documents_bytes = fs::read(documents.join("config.json")).unwrap();
        let loaded = Config::load(&support, Some(&documents)).unwrap();
        assert_same_settings(&loaded, &original);
        assert_eq!(loaded.download_dir, documents.join("漫画下载"));
        assert_eq!(
            fs::read(documents.join("config.json")).unwrap(),
            documents_bytes
        );
        assert_same_settings(
            &Config::load(&support, Some(&documents)).unwrap(),
            &original,
        );
    }

    #[test]
    fn missing_support_config_imports_documents_settings_without_resetting_network_profile() {
        let temp = tempfile::tempdir().unwrap();
        let (documents, support) = roots(temp.path());
        fs::create_dir_all(&documents).unwrap();
        let original = fake_network_profile(&documents);
        original.save_in(&documents, Some(&documents)).unwrap();
        let source = fs::read(documents.join("config.json")).unwrap();
        let loaded = Config::load(&support, Some(&documents)).unwrap();
        assert_same_settings(&loaded, &original);
        assert_eq!(fs::read(documents.join("config.json")).unwrap(), source);
        assert_eq!(
            fs::read(support.join("config.before-documents-migration.json")).unwrap(),
            source
        );
        let persisted: Config =
            serde_json::from_str(&fs::read_to_string(support.join("config.json")).unwrap())
                .unwrap();
        assert_same_settings(&persisted, &original);
        assert_eq!(persisted.download_dir, Path::new("漫画下载"));
    }
}
