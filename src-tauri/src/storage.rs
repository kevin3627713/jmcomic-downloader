use std::path::{Component, Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

use crate::extensions::WalkDirEntryExt;

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub download_dir: PathBuf,
    pub export_dir: PathBuf,
    pub config_path: PathBuf,
    pub migration_warnings: Vec<String>,
}

pub struct MigrationWarnings(pub Vec<String>);

/// Resolve the active guest's Documents on every launch. Never store a host or
/// LiveContainer UUID as the permanent root of the iOS library.
pub fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    #[cfg(target_os = "ios")]
    return Ok(app.path().document_dir()?);
    #[cfg(not(target_os = "ios"))]
    Ok(app.path().app_data_dir()?)
}

/// Keep private settings at the location used by previous working versions.
pub fn config_dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_data_dir()?)
}

fn relative(path: &Path) -> Result<PathBuf> {
    ensure!(
        path.components()
            .any(|part| matches!(part, Component::Normal(_))),
        "存储目录不能是 Documents 根目录"
    );
    ensure!(
        path.components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "存储相对路径不能包含父目录或根目录"
    );
    Ok(path.to_path_buf())
}

/// Match only the app's old support directory. The actual migration source is
/// reconstructed inside the current guest, never opened at the saved UUID.
pub fn legacy_relative(path: &Path, legacy_root: &Path) -> Option<PathBuf> {
    if let Ok(suffix) = path.strip_prefix(legacy_root) {
        return Some(suffix.into());
    }
    let identifier = legacy_root.file_name()?;
    let parts: Vec<_> = path.components().collect();
    let index = parts.windows(3).rposition(|parts| {
        parts[0].as_os_str() == "Library"
            && parts[1].as_os_str() == "Application Support"
            && parts[2].as_os_str() == identifier
    })?;
    Some(
        parts[index + 3..]
            .iter()
            .map(|part| part.as_os_str())
            .collect(),
    )
}

pub fn documents_path(stored: &Path, documents: &Path, legacy_root: &Path) -> Result<PathBuf> {
    if !stored.has_root() && !stored.is_absolute() {
        return Ok(documents.join(relative(stored)?));
    }
    if let Ok(suffix) = stored.strip_prefix(documents) {
        return Ok(documents.join(relative(suffix)?));
    }
    if let Some(suffix) = legacy_relative(stored, legacy_root) {
        return Ok(documents.join(relative(&suffix)?));
    }
    // LiveContainer's host also has a Documents directory. Use the final one,
    // which belongs to the guest, so nested containers stay correctly anchored.
    let parts: Vec<_> = stored.components().collect();
    if let Some(index) = parts
        .iter()
        .rposition(|part| part.as_os_str() == "Documents")
    {
        let suffix: PathBuf = parts[index + 1..]
            .iter()
            .map(|part| part.as_os_str())
            .collect();
        return Ok(documents.join(relative(&suffix)?));
    }
    anyhow::bail!(
        "iOS 存储目录必须位于当前应用的 Documents 内：{}",
        stored.display()
    )
}

pub fn saved_path(runtime: &Path, documents: &Path) -> Result<PathBuf> {
    relative(
        runtime
            .strip_prefix(documents)
            .context("iOS 存储目录不在当前 Documents 内")?,
    )
}

fn existing(path: &Path) -> Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                !metadata.file_type().is_symlink(),
                "存储目录包含符号链接：{}",
                path.display()
            );
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("无法访问存储位置 `{}`", path.display())),
    }
}

/// Same-container renames do not copy large image libraries or overwrite files.
/// A partially completed migration can be resumed on the next launch.
pub fn migrate_directory(source: &Path, destination: &Path) -> Result<()> {
    let Some(source_info) = existing(source)? else {
        return Ok(());
    };
    ensure!(
        source_info.is_dir(),
        "旧存储位置不是目录：{}",
        source.display()
    );
    match existing(destination)? {
        None => {
            std::fs::create_dir_all(destination.parent().context("迁移目标没有父目录")?)?;
            std::fs::rename(source, destination).with_context(|| {
                format!(
                    "迁移 `{}` 到 `{}` 失败",
                    source.display(),
                    destination.display()
                )
            })?;
        }
        Some(target_info) => {
            ensure!(
                target_info.is_dir(),
                "迁移目标不是目录：{}",
                destination.display()
            );
            for entry in std::fs::read_dir(source)? {
                let entry = entry?;
                let old = entry.path();
                let new = destination.join(entry.file_name());
                let info = existing(&old)?.context("迁移期间源文件消失")?;
                if existing(&new)?.is_none() {
                    std::fs::rename(&old, &new)
                        .with_context(|| format!("迁移 `{}` 失败", old.display()))?;
                } else if info.is_dir() {
                    migrate_directory(&old, &new)?;
                } else {
                    anyhow::bail!(
                        "旧目录和 Documents 中存在同名文件，已保留两份文件，请处理后重试：{}；{}",
                        old.display(),
                        new.display()
                    );
                }
            }
            std::fs::remove_dir(source)?;
        }
    }
    Ok(())
}

pub fn prepare_directory(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("创建存储目录 `{}` 失败", path.display()))?;
    // Detect access failures at startup, before reporting a usable library.
    tempfile::NamedTempFile::new_in(path)
        .with_context(|| format!("存储目录不可写 `{}`", path.display()))?;
    Ok(())
}

pub fn metadata_paths(download_dir: &Path) -> Result<Vec<PathBuf>> {
    let Some(info) = existing(download_dir)? else {
        return Ok(Vec::new());
    };
    ensure!(
        info.is_dir(),
        "漫画下载位置不是目录：{}",
        download_dir.display()
    );
    let mut paths = Vec::new();
    for entry in WalkDir::new(download_dir).follow_links(false) {
        let entry =
            entry.with_context(|| format!("读取书库目录 `{}` 失败", download_dir.display()))?;
        if entry.is_comic_metadata() {
            paths.push(entry.path().to_path_buf());
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_livecontainer_paths_resolve_to_guest_documents_after_host_and_guest_move() {
        let old = Path::new("/private/var/mobile/Containers/Data/Application/old-host/Documents/Data/Application/old-guest/Library/Application Support/com.lanyeeee.jmcomic-downloader/漫画下载");
        let documents = Path::new("/private/var/mobile/Containers/Data/Application/new-host/Documents/Data/Application/new-guest/Documents");
        let legacy = documents
            .parent()
            .unwrap()
            .join("Library/Application Support/com.lanyeeee.jmcomic-downloader");
        let expected = documents.join("漫画下载");
        assert_eq!(documents_path(old, documents, &legacy).unwrap(), expected);
        let previous_documents = Path::new("/private/var/mobile/Containers/Data/Application/old-host/Documents/Data/Application/old-guest/Documents/漫画下载");
        assert_eq!(
            documents_path(previous_documents, documents, &legacy).unwrap(),
            expected
        );
        assert_eq!(
            documents_path(Path::new("漫画下载"), documents, &legacy).unwrap(),
            expected
        );
        assert_eq!(
            saved_path(&expected, documents).unwrap(),
            Path::new("漫画下载")
        );
        assert!(documents_path(Path::new("../another-container"), documents, &legacy).is_err());
    }

    #[test]
    fn migration_merges_directories_without_overwriting_and_can_resume() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("Library/old");
        let new = temp.path().join("Documents/new");
        std::fs::create_dir_all(old.join("book")).unwrap();
        std::fs::create_dir_all(new.join("book")).unwrap();
        std::fs::write(old.join("book/conflict.pdf"), b"old fictional export").unwrap();
        std::fs::write(new.join("book/conflict.pdf"), b"new fictional export").unwrap();
        assert!(migrate_directory(&old, &new).is_err());
        assert_eq!(
            std::fs::read(old.join("book/conflict.pdf")).unwrap(),
            b"old fictional export"
        );
        assert_eq!(
            std::fs::read(new.join("book/conflict.pdf")).unwrap(),
            b"new fictional export"
        );
        std::fs::rename(new.join("book/conflict.pdf"), new.join("book/retained.pdf")).unwrap();
        migrate_directory(&old, &new).unwrap();
        assert!(!old.exists());
        assert!(new.join("book/retained.pdf").exists());
        assert!(new.join("book/conflict.pdf").exists());
        migrate_directory(&old, &new).unwrap();
    }

    #[test]
    fn missing_library_is_empty_but_wrong_root_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("漫画下载");
        assert!(metadata_paths(&root).unwrap().is_empty());
        std::fs::write(&root, b"fictional non-directory").unwrap();
        assert!(metadata_paths(&root).is_err());
        assert!(prepare_directory(&root).is_err());
    }
}
