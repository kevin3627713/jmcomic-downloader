use crate::types::Comic;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum DeleteKind {
    All,
    Pdf,
    Cbz,
}

#[derive(Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicFileStatus {
    pub pdf_count: usize,
    pub cbz_count: usize,
    pub has_pdf: bool,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub removed_files: u64,
    pub removed_comic: bool,
}

struct Layout {
    source: PathBuf,
    export: Option<PathBuf>,
    download_root: PathBuf,
    export_root: Option<PathBuf>,
}

// Resolve only ordinary child components, rejecting links/junctions before any
// traversal. Configured roots may themselves be relocated by the user.
fn child(root: &Path, relative: &Path) -> Result<Option<PathBuf>> {
    ensure!(
        !relative.as_os_str().is_empty(),
        "不能操作整个下载或导出目录"
    );
    ensure!(
        relative
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "文件路径包含不安全的目录层级"
    );
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part.as_os_str());
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("无法检查文件路径"),
        };
        ensure!(
            !metadata.file_type().is_symlink(),
            "拒绝操作符号链接或目录联接"
        );
        let resolved = fs::canonicalize(&path)?;
        ensure!(
            resolved.starts_with(root) && resolved != root,
            "目标不在指定目录内"
        );
    }
    Ok(Some(fs::canonicalize(path)?))
}

fn metadata_id(path: &Path) -> Result<i64> {
    let metadata: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    metadata
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .context("漫画元数据没有有效的 ID")
}

fn layout(download_root: &Path, export_root: &Path, comic: &Comic) -> Result<Layout> {
    let requested = comic
        .comic_download_dir
        .as_ref()
        .context("漫画没有下载目录")?;
    let relative = requested
        .strip_prefix(download_root)
        .context("漫画不在当前下载目录中")?;
    let download_root = fs::canonicalize(download_root)?;
    let source = child(&download_root, relative)?.context("漫画下载目录已不存在，请刷新书库")?;
    ensure!(source.is_dir(), "漫画下载目录不是文件夹");
    let metadata =
        child(&download_root, &relative.join("元数据.json"))?.context("找不到漫画元数据")?;
    ensure!(
        metadata_id(&metadata)? == comic.id,
        "漫画 ID 与目录中的元数据不一致"
    );
    let export_root = if export_root.try_exists()? {
        Some(fs::canonicalize(export_root)?)
    } else {
        None
    };
    let export = export_root
        .as_ref()
        .map(|root| child(root, relative))
        .transpose()?
        .flatten();
    if let Some(export) = &export {
        ensure!(export.is_dir(), "漫画导出目录不是文件夹");
    }
    Ok(Layout {
        source,
        export,
        download_root,
        export_root,
    })
}

fn archives(layout: &Layout, kind: DeleteKind) -> Result<Vec<PathBuf>> {
    let Some(export) = &layout.export else {
        return Ok(Vec::new());
    };
    let extension = match kind {
        DeleteKind::Pdf => "pdf",
        DeleteKind::Cbz => "cbz",
        DeleteKind::All => unreachable!(),
    };
    let mut files = Vec::new();
    if matches!(kind, DeleteKind::Pdf) {
        let name = layout
            .source
            .file_name()
            .context("漫画目录没有名称")?
            .to_string_lossy();
        if let Some(pdf) = child(export, Path::new(&format!("{name}.pdf")))? {
            ensure!(pdf.is_file(), "整本 PDF 路径不是文件");
            files.push(pdf);
        }
    }
    if let Some(directory) = child(export, Path::new(extension))? {
        ensure!(directory.is_dir(), "章节导出路径不是文件夹");
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
            {
                let path = child(&directory, Path::new(&entry.file_name()))?
                    .context("导出文件在操作期间消失")?;
                ensure!(path.is_file(), "导出文件不是普通文件");
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn file_status(
    download_root: &Path,
    export_root: &Path,
    comic: &Comic,
) -> Result<ComicFileStatus> {
    let layout = layout(download_root, export_root, comic)?;
    let pdf = archives(&layout, DeleteKind::Pdf)?;
    Ok(ComicFileStatus {
        has_pdf: pdf
            .iter()
            .any(|path| path.parent() == layout.export.as_deref()),
        pdf_count: pdf.len(),
        cbz_count: archives(&layout, DeleteKind::Cbz)?.len(),
    })
}

fn validate_tree(path: &Path, id: i64) -> Result<u64> {
    let mut count = 0;
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_symlink(),
            "目录包含符号链接或目录联接，不能整本删除"
        );
        if entry.file_name() == "元数据.json" {
            ensure!(
                metadata_id(entry.path())? == id,
                "目标目录包含其他漫画，不能整本删除"
            );
        }
        if entry.file_type().is_file() {
            count += 1;
        }
    }
    Ok(count)
}

pub fn delete_files(
    download_root: &Path,
    export_root: &Path,
    comic: &Comic,
    kind: DeleteKind,
) -> Result<DeleteResult> {
    let layout = layout(download_root, export_root, comic)?;
    if !matches!(kind, DeleteKind::All) {
        // Validate every path before removing the first file. No arbitrary path
        // sent by the WebView is passed to remove_file/remove_dir_all.
        let files = archives(&layout, kind)?;
        for path in &files {
            fs::remove_file(path).with_context(|| format!("删除 {} 失败", path.display()))?;
        }
        return Ok(DeleteResult {
            removed_files: files.len() as u64,
            removed_comic: false,
        });
    }
    let mut targets = Vec::new();
    if let Some(export) = &layout.export {
        if layout.source.starts_with(export) {
            targets.push(export.clone());
        } else {
            if !export.starts_with(&layout.source) {
                targets.push(export.clone());
            }
            targets.push(layout.source.clone());
        }
    } else {
        targets.push(layout.source.clone());
    }
    let mut count = 0;
    for target in &targets {
        ensure!(
            !layout.download_root.starts_with(target),
            "不能删除下载根目录或其父目录"
        );
        if let Some(root) = &layout.export_root {
            ensure!(!root.starts_with(target), "不能删除导出根目录或其父目录");
        }
        count += validate_tree(target, comic.id)?;
    }
    for target in targets {
        fs::remove_dir_all(&target).with_context(|| format!("删除 {} 失败", target.display()))?;
    }
    Ok(DeleteResult {
        removed_files: count,
        removed_comic: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        _temp: tempfile::TempDir,
        download: PathBuf,
        export: PathBuf,
        comic: Comic,
        output: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let download = temp.path().join("downloads");
            let export = temp.path().join("exports");
            let source = download.join("作者/书名 & [1]");
            let output = export.join("作者/书名 & [1]");
            for dir in [
                source.join("chapter"),
                output.join("pdf"),
                output.join("cbz"),
                export.join("other"),
            ] {
                fs::create_dir_all(dir).unwrap();
            }
            fs::write(source.join("元数据.json"), br#"{"id":1}"#).unwrap();
            fs::write(source.join("chapter/0001.jpg"), b"fictional source bytes").unwrap();
            fs::write(output.join("书名 & [1].pdf"), b"merged pdf fixture").unwrap();
            fs::write(output.join("pdf/chapter.PDF"), b"chapter pdf fixture").unwrap();
            fs::write(output.join("pdf/notes.txt"), b"keep this unrelated file").unwrap();
            fs::write(output.join("cbz/chapter.cbz"), b"cbz fixture").unwrap();
            fs::write(export.join("other/other.pdf"), b"another comic").unwrap();
            let comic = Comic {
                id: 1,
                comic_download_dir: Some(source),
                ..Default::default()
            };
            Self {
                _temp: temp,
                download,
                export,
                comic,
                output,
            }
        }
        fn delete(&self, kind: DeleteKind) -> Result<DeleteResult> {
            delete_files(&self.download, &self.export, &self.comic, kind)
        }
    }
    #[test]
    fn pdf_delete_includes_merged_and_chapter_files_preserving_cbz_and_source() {
        let f = Fixture::new();
        assert_eq!(f.delete(DeleteKind::Pdf).unwrap().removed_files, 2);
        assert!(f.output.join("cbz/chapter.cbz").exists());
        assert!(f.output.join("pdf/notes.txt").exists());
        assert!(f
            .comic
            .comic_download_dir
            .as_ref()
            .unwrap()
            .join("chapter/0001.jpg")
            .exists());
        assert_eq!(
            file_status(&f.download, &f.export, &f.comic)
                .unwrap()
                .pdf_count,
            0
        );
        assert_eq!(f.delete(DeleteKind::Pdf).unwrap().removed_files, 0);
    }
    #[test]
    fn cbz_delete_preserves_pdf_and_source() {
        let f = Fixture::new();
        assert_eq!(f.delete(DeleteKind::Cbz).unwrap().removed_files, 1);
        assert!(f.output.join("书名 & [1].pdf").exists());
        assert!(f.comic.comic_download_dir.as_ref().unwrap().exists());
    }
    #[test]
    fn whole_comic_delete_preserves_other_comics() {
        let f = Fixture::new();
        assert!(f.delete(DeleteKind::All).unwrap().removed_comic);
        assert!(!f.output.exists());
        assert!(!f.comic.comic_download_dir.as_ref().unwrap().exists());
        assert!(f.export.join("other/other.pdf").exists());
        assert!(f.download.exists());
    }
    #[test]
    fn root_traversal_wrong_id_and_foreign_nested_metadata_are_rejected_before_deletion() {
        let mut f = Fixture::new();
        let source = f.comic.comic_download_dir.clone();
        f.comic.comic_download_dir = Some(f.download.clone());
        assert!(f.delete(DeleteKind::All).is_err());
        f.comic.comic_download_dir = Some(f.download.join("../exports/作者/书名 & [1]"));
        assert!(f.delete(DeleteKind::Pdf).is_err());
        f.comic.comic_download_dir = source;
        f.comic.id = 2;
        assert!(f.delete(DeleteKind::All).is_err());
        f.comic.id = 1;
        fs::write(
            f.comic
                .comic_download_dir
                .as_ref()
                .unwrap()
                .join("chapter/元数据.json"),
            br#"{"id":2}"#,
        )
        .unwrap();
        assert!(f.delete(DeleteKind::All).is_err());
        assert!(f.output.join("书名 & [1].pdf").exists());
        assert!(f.comic.comic_download_dir.as_ref().unwrap().exists());
    }
    #[test]
    fn export_root_inside_comic_cannot_be_removed() {
        let f = Fixture::new();
        let source = f.comic.comic_download_dir.as_ref().unwrap();
        assert!(delete_files(
            &f.download,
            &source.join("chapter"),
            &f.comic,
            DeleteKind::All
        )
        .is_err());
        assert!(source.exists());
    }
    #[cfg(unix)]
    #[test]
    fn linked_export_directory_is_rejected() {
        let f = Fixture::new();
        fs::remove_dir_all(f.output.join("pdf")).unwrap();
        std::os::unix::fs::symlink(f.export.join("other"), f.output.join("pdf")).unwrap();
        assert!(f.delete(DeleteKind::Pdf).is_err());
        assert!(f.export.join("other/other.pdf").exists());
    }
}
