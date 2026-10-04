use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{atomic::AtomicU32, Arc, OnceLock},
};

use anyhow::{anyhow, ensure, Context};

use lopdf::{
    content::{Content, Operation},
    dictionary, Bookmark, Document, Object, Stream,
};
use parking_lot::Mutex;
use tauri::AppHandle;
use tauri_specta::Event;
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{
    events::{ExportCbzEvent, ExportPdfEvent},
    extensions::{AnyhowErrorToStringChain, PathIsImg},
    types::{ChapterInfo, Comic, ComicInfo},
};

pub enum ExportArchive {
    Cbz,
    Pdf,
}

// Prevent two requests from rewriting chapter files while another request merges them.
static ACTIVE_EXPORTS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

struct ExportGuard(PathBuf);

impl ExportGuard {
    fn acquire(path: PathBuf) -> anyhow::Result<Self> {
        ensure!(
            ACTIVE_EXPORTS
                .get_or_init(Default::default)
                .lock()
                .insert(path.clone()),
            "此漫画正在导出，请等待完成后再试"
        );
        Ok(Self(path))
    }
}

impl Drop for ExportGuard {
    fn drop(&mut self) {
        ACTIVE_EXPORTS
            .get_or_init(Default::default)
            .lock()
            .remove(&self.0);
    }
}

fn chapter_images(chapter: &ChapterInfo) -> anyhow::Result<Vec<PathBuf>> {
    let dir = chapter
        .chapter_download_dir
        .as_ref()
        .context("章节没有下载目录")?;
    let mut images = Vec::new();
    for entry in std::fs::read_dir(dir).context(format!("读取章节目录 {} 失败", dir.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().is_img() {
            images.push(entry.path());
        }
    }
    images.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    ensure!(
        !images.is_empty(),
        "章节 {} 没有图片，请重新下载",
        chapter.chapter_title
    );
    if let Some(expected) = chapter.page_count {
        ensure!(
            images.len() == expected as usize,
            "章节 {} 图片不完整：应有 {} 张，实际 {} 张，请重新下载",
            chapter.chapter_title,
            expected,
            images.len()
        );
    }
    // Decode, rather than checking only a file header, to reject interrupted downloads.
    for path in &images {
        image::open(path).context(format!("图片 {} 已损坏，请重新下载", path.display()))?;
    }
    Ok(images)
}

impl ExportArchive {
    pub fn extension(&self) -> &str {
        match self {
            ExportArchive::Cbz => "cbz",
            ExportArchive::Pdf => "pdf",
        }
    }
}

struct CbzErrorEventGuard {
    uuid: String,
    app: AppHandle,
    success: bool,
}

impl Drop for CbzErrorEventGuard {
    fn drop(&mut self) {
        if self.success {
            return;
        }

        let uuid = self.uuid.clone();
        let _ = ExportCbzEvent::Error { uuid }.emit(&self.app);
    }
}

#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::too_many_lines)]
pub fn cbz(app: &AppHandle, comic: &Comic) -> anyhow::Result<()> {
    let _export_guard = ExportGuard::acquire(comic.get_comic_export_dir(app)?.join("cbz"))?;
    let downloaded_chapter_infos = comic
        .chapter_infos
        .iter()
        .filter(|chapter_info| chapter_info.is_downloaded.unwrap_or(false))
        .collect::<Vec<_>>();
    // 生成格式化的xml
    let cfg = yaserde::ser::Config {
        perform_indent: true,
        ..Default::default()
    };
    let event_uuid = uuid::Uuid::new_v4().to_string();
    // 发送开始导出cbz事件
    let _ = ExportCbzEvent::Start {
        uuid: event_uuid.clone(),
        comic_title: comic.name.clone(),
        total: downloaded_chapter_infos.len() as u32,
    }
    .emit(app);
    // 如果success为false，drop时发送Error事件
    let mut error_event_guard = CbzErrorEventGuard {
        uuid: event_uuid.clone(),
        app: app.clone(),
        success: false,
    };
    ensure!(
        !downloaded_chapter_infos.is_empty(),
        "没有已下载完成的章节可导出"
    );
    // 用来记录导出进度
    let current = Arc::new(AtomicU32::new(0));

    let extension = ExportArchive::Cbz.extension();
    let comic_export_dir = comic
        .get_comic_export_dir(app)
        .context("获取导出目录失败")?;
    let chapter_export_dir = comic_export_dir.join(extension);
    // 保证导出目录存在
    std::fs::create_dir_all(&chapter_export_dir)
        .context(format!("创建目录`{}`失败", chapter_export_dir.display()))?;
    // 先把封面拷贝到导出目录(如果有)
    if let Err(err) = copy_cover(comic, &chapter_export_dir) {
        let comic_title = &comic.name;
        let err_title = format!("`{comic_title}`导出cbz时，将封面拷贝到导出目录失败");
        let string_chain = err.to_string_chain();
        tracing::error!(err_title, message = string_chain);
    }
    // Validate and package one chapter at a time to bound decoding memory on phones.
    downloaded_chapter_infos
        .into_iter()
        .try_for_each(|chapter_info| -> anyhow::Result<()> {
            let chapter_title = chapter_info.chapter_title.clone();
            // 生成ComicInfo
            let comic_info = ComicInfo::from(comic, chapter_info);
            // 序列化ComicInfo为xml
            let comic_info_xml =
                yaserde::ser::to_string_with_config(&comic_info, &cfg).map_err(|err_msg| {
                    anyhow!("章节`{chapter_title}`序列化`ComicInfo.xml`失败: {err_msg}")
                })?;
            // 创建cbz文件
            let chapter_download_dir_name = &chapter_info
                .get_chapter_download_dir_name()
                .context(format!("章节`{chapter_title}`获取章节下载目录名失败"))?;
            let save_path =
                chapter_export_dir.join(format!("{chapter_download_dir_name}.{extension}"));
            let image_paths = chapter_images(chapter_info)?;
            let mut zip_file = tempfile::NamedTempFile::new_in(&chapter_export_dir)?;
            let mut zip_writer = ZipWriter::new(zip_file.as_file_mut());
            // 把ComicInfo.xml写入cbz
            zip_writer
                .start_file("ComicInfo.xml", SimpleFileOptions::default())
                .context(format!(
                    "章节`{chapter_title}`在`{}`创建`ComicInfo.xml`失败",
                    save_path.display()
                ))?;
            zip_writer
                .write_all(comic_info_xml.as_bytes())
                .context(format!("章节`{chapter_title}`写入`ComicInfo.xml`失败"))?;

            for image_path in image_paths {
                let filename = match image_path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => continue,
                };
                // 将文件写入cbz
                zip_writer
                    .start_file(&filename, SimpleFileOptions::default())
                    .context(format!(
                        "章节`{chapter_title}`在`{}`创建`{filename}`失败",
                        save_path.display()
                    ))?;
                let mut file = std::fs::File::open(&image_path)
                    .context(format!("打开`{}`失败", image_path.display()))?;
                std::io::copy(&mut file, &mut zip_writer).context(format!(
                    "章节`{chapter_title}`将`{}`写入`{}`失败",
                    image_path.display(),
                    save_path.display()
                ))?;
            }

            zip_writer.finish().context(format!(
                "章节`{chapter_title}`关闭`{}`失败",
                save_path.display()
            ))?;
            zip_file.as_file().sync_all()?;
            zip_file.persist(&save_path).map_err(|err| err.error)?;
            // 更新导出cbz的进度
            let current = current.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            // 发送导出cbz进度事件
            let _ = ExportCbzEvent::Progress {
                uuid: event_uuid.clone(),
                current,
            }
            .emit(app);
            Ok(())
        })?;
    // 标记为成功，后面drop时就不会发送Error事件
    error_event_guard.success = true;
    // 发送导出cbz完成事件
    let _ = ExportCbzEvent::End {
        uuid: event_uuid,
        chapter_export_dir,
    }
    .emit(app);

    Ok(())
}

fn copy_cover(comic: &Comic, chapter_export_dir: &Path) -> anyhow::Result<()> {
    let src_cover_path = comic.get_cover_path().context("获取封面路径失败")?;
    let cover_filename = src_cover_path.file_name().context("获取封面的文件名失败")?;

    if src_cover_path.exists() {
        let dst_cover_path = chapter_export_dir.join(cover_filename);
        std::fs::copy(src_cover_path, dst_cover_path)?;
    }

    Ok(())
}

struct PdfCreateErrorEventGuard {
    uuid: String,
    app: AppHandle,
    success: bool,
}

impl Drop for PdfCreateErrorEventGuard {
    fn drop(&mut self) {
        if self.success {
            return;
        }

        let uuid = self.uuid.clone();
        let _ = ExportPdfEvent::CreateError { uuid }.emit(&self.app);
    }
}

struct PdfMergeErrorEventGuard {
    uuid: String,
    app: AppHandle,
    success: bool,
}

impl Drop for PdfMergeErrorEventGuard {
    fn drop(&mut self) {
        if self.success {
            return;
        }

        let uuid = self.uuid.clone();
        let _ = ExportPdfEvent::MergeError { uuid }.emit(&self.app);
    }
}

#[allow(clippy::cast_possible_truncation)]
pub fn pdf(app: &AppHandle, comic: &Comic) -> anyhow::Result<()> {
    let _export_guard = ExportGuard::acquire(comic.get_comic_export_dir(app)?.join("pdf"))?;
    let downloaded_chapter_infos: Vec<&ChapterInfo> = comic
        .chapter_infos
        .iter()
        .filter(|chapter_info| chapter_info.is_downloaded.unwrap_or(false))
        .collect();
    let event_uuid = uuid::Uuid::new_v4().to_string();
    // 发送开始创建pdf事件
    let _ = ExportPdfEvent::CreateStart {
        uuid: event_uuid.clone(),
        comic_title: comic.name.clone(),
        total: downloaded_chapter_infos.len() as u32,
    }
    .emit(app);
    // 如果success为false，drop时发送CreateError事件
    let mut create_error_event_guard = PdfCreateErrorEventGuard {
        uuid: event_uuid.clone(),
        app: app.clone(),
        success: false,
    };
    ensure!(
        !downloaded_chapter_infos.is_empty(),
        "没有已下载完成的章节可导出"
    );
    // 用来记录创建pdf的进度
    let current = Arc::new(AtomicU32::new(0));

    let extension = ExportArchive::Pdf.extension();
    let comic_export_dir = comic
        .get_comic_export_dir(app)
        .context("获取导出目录失败")?;
    let chapter_export_dir = comic_export_dir.join(extension);
    // 保证导出目录存在
    std::fs::create_dir_all(&chapter_export_dir)
        .context(format!("创建目录`{}`失败", chapter_export_dir.display()))?;
    let mut chapter_with_pdf_path = Vec::new();
    // Process chapters sequentially to bound image decoding memory on iOS.
    for chapter_info in downloaded_chapter_infos {
        let chapter_title = &chapter_info.chapter_title;

        let image_paths = chapter_images(chapter_info)?;

        let chapter_download_dir_name = &chapter_info
            .get_chapter_download_dir_name()
            .context(format!("章节`{chapter_title}`获取章节下载目录名失败"))?;
        // 创建pdf
        let save_path = chapter_export_dir.join(format!("{chapter_download_dir_name}.{extension}"));

        create_pdf(image_paths, &save_path).context(format!("章节`{chapter_title}`创建pdf失败"))?;

        chapter_with_pdf_path.push((chapter_info, save_path));
        // 更新创建pdf的进度
        let current = current.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        // 发送创建pdf进度事件
        let _ = ExportPdfEvent::CreateProgress {
            uuid: event_uuid.clone(),
            current,
        }
        .emit(app);
    }

    chapter_with_pdf_path.sort_by(|(a, _), (b, _)| a.order.cmp(&b.order));
    let chapter_pdf_paths: Vec<PathBuf> = chapter_with_pdf_path
        .into_iter()
        .map(|(_, pdf_path)| pdf_path)
        .collect();

    // 标记为成功，后面drop时就不会发送CreateError事件
    create_error_event_guard.success = true;
    // 发送创建pdf完成事件
    let _ = ExportPdfEvent::CreateEnd {
        uuid: event_uuid,
        chapter_export_dir,
    }
    .emit(app);

    let event_uuid = uuid::Uuid::new_v4().to_string();
    // 发送开始合并pdf事件
    let _ = ExportPdfEvent::MergeStart {
        uuid: event_uuid.clone(),
        comic_title: comic.name.clone(),
    }
    .emit(app);
    // 如果success为false，drop时发送MergeError事件
    let mut merge_error_event_guard = PdfMergeErrorEventGuard {
        uuid: event_uuid.clone(),
        app: app.clone(),
        success: false,
    };

    let comic_download_dir_name = &comic
        .get_comic_download_dir_name()
        .context("获取漫画下载目录名失败")?;
    let save_path = comic_export_dir.join(format!("{comic_download_dir_name}.{extension}"));
    merge_pdf(chapter_pdf_paths, &save_path).context("合并pdf失败")?;
    // 标记为成功，后面drop时就不会发送MergeError事件
    merge_error_event_guard.success = true;
    // 发送合并pdf完成事件
    let _ = ExportPdfEvent::MergeEnd {
        uuid: event_uuid,
        chapter_export_dir: save_path,
    }
    .emit(app);
    Ok(())
}

/// 用`image_paths`中的图片按顺序创建PDF，保存到`save_path`中
#[allow(clippy::similar_names)]
#[allow(clippy::cast_possible_truncation)]
fn create_pdf(image_paths: Vec<PathBuf>, save_path: &Path) -> anyhow::Result<()> {
    ensure!(!image_paths.is_empty(), "没有图片，拒绝创建空 PDF");
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids = vec![];

    for image_path in image_paths {
        ensure!(image_path.is_file(), "图片 {} 不存在", image_path.display());

        let buffer = read_image_to_buffer(&image_path)
            .context(format!("将`{}`读取到buffer失败", image_path.display()))?;
        let (width, height) = image::image_dimensions(&image_path)
            .context(format!("获取`{}`的尺寸失败", image_path.display()))?;
        let image_stream = lopdf::xobject::image_from(buffer)
            .context(format!("创建`{}`的图片流失败", image_path.display()))?;
        // 将图片流添加到doc中
        let img_id = doc.add_object(image_stream);
        // 图片的名称，用于 Do 操作在页面上显示图片
        let img_name = format!("X{}", img_id.0);
        // 用于设置图片在页面上的位置和大小
        let cm_operation = Operation::new(
            "cm",
            vec![
                width.into(),
                0.into(),
                0.into(),
                height.into(),
                0.into(),
                0.into(),
            ],
        );
        // 用于显示图片
        let do_operation = Operation::new("Do", vec![Object::Name(img_name.as_bytes().to_vec())]);
        // 创建页面，设置图片的位置和大小，然后显示图片
        // 因为是从零开始创建PDF，所以没必要用 q 和 Q 操作保存和恢复图形状态
        let content = Content {
            operations: vec![cm_operation, do_operation],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });
        // 将图片以 XObject 的形式添加到文档中
        // Do 操作只能引用 XObject(所以前面定义的 Do 操作的参数是 img_name, 而不是 img_id)
        doc.add_xobject(page_id, img_name.as_bytes(), img_id)?;
        // 记录新创建的页面的 ID
        page_ids.push(page_id);
    }
    // 将"Pages"添加到doc中
    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as u32,
        "Kids" => page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    // 新建一个"Catalog"对象，将"Pages"对象添加到"Catalog"对象中，然后将"Catalog"对象添加到doc中
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc.compress();

    save_pdf_atomic(doc, save_path)?;

    Ok(())
}

/// 读取`image_path`中的图片数据到buffer中
fn read_image_to_buffer(image_path: &Path) -> anyhow::Result<Vec<u8>> {
    if image_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"))
    {
        // PDF is static; embed the first frame of an animated page.
        let mut buffer = std::io::Cursor::new(Vec::new());
        image::open(image_path)?.write_to(&mut buffer, image::ImageFormat::Png)?;
        return Ok(buffer.into_inner());
    }
    let file =
        std::fs::File::open(image_path).context(format!("打开`{}`失败", image_path.display()))?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![];
    reader
        .read_to_end(&mut buffer)
        .context(format!("读取`{}`失败", image_path.display()))?;
    Ok(buffer)
}

/// 将`chapter_pdf_paths`中的PDF按顺序合并成一个，保存到`save_path`中
#[allow(clippy::cast_possible_truncation)]
fn merge_pdf(chapter_pdf_paths: Vec<PathBuf>, save_path: &Path) -> anyhow::Result<()> {
    ensure!(!chapter_pdf_paths.is_empty(), "没有章节 PDF 可合并");
    let mut doc = Document::with_version("1.5");
    let mut doc_page_ids = vec![];
    let mut doc_objects = BTreeMap::new();

    for chapter_pdf_path in chapter_pdf_paths {
        let mut chapter_doc = Document::load(&chapter_pdf_path)
            .context(format!("加载`{}`失败", chapter_pdf_path.display()))?;
        // 重新编号这个章节PDF的对象，避免与doc的对象编号冲突
        chapter_doc.renumber_objects_with(doc.max_id + 1);
        doc.max_id = chapter_doc.max_id;
        ensure!(
            !chapter_doc.get_pages().is_empty(),
            "章节 PDF {} 没有页面",
            chapter_pdf_path.display()
        );
        // 获取这个章节PDF中的所有页面，并给第一个页面添加书签
        let mut chapter_page_ids = vec![];
        for (page_num, object_id) in chapter_doc.get_pages() {
            // 第一个页面需要添加书签
            if page_num == 1 {
                let chapter_title = chapter_pdf_path
                    .file_stem()
                    .context(format!("获取`{}`的文件名失败", chapter_pdf_path.display()))?
                    .to_string_lossy()
                    .to_string();
                let bookmark = Bookmark::new(chapter_title, [0.0, 0.0, 1.0], 0, object_id);
                doc.add_bookmark(bookmark, None);
            }
            chapter_page_ids.push(object_id);
        }

        doc_page_ids.extend(chapter_page_ids);
        doc_objects.extend(chapter_doc.objects);
    }
    // 在doc中新建一个"Pages"对象，将所有章节的页面添加到这个"Pages"对象中
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => doc_page_ids.len() as u32,
        "Kids" => doc_page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
    });

    for (object_id, mut object) in doc_objects {
        match object.type_name().unwrap_or(b"") {
            b"Page" => {
                if let Ok(page_dict) = object.as_dict_mut() {
                    // 将页面对象的"Parent"字段设置为新建的"Pages"对象，这样这个页面就成为了"Pages"对象的子页面
                    page_dict.set("Parent", pages_id);
                    doc.objects.insert(object_id, object);
                };
            }
            // 忽略这些对象
            b"Catalog" | b"Pages" | b"Outlines" | b"Outline" => {}
            // 将所有其他对象添加到doc中
            _ => {
                doc.objects.insert(object_id, object);
            }
        }
    }
    // 新建一个"Catalog"对象，将"Pages"对象添加到"Catalog"对象中，然后将"Catalog"对象添加到doc中
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    // 如果有书签没有关联到具体页面，将这些书签指向第一个页面
    doc.adjust_zero_pages();
    // 将书签添加到doc中
    if let Some(outline_id) = doc.build_outline() {
        if let Ok(Object::Dictionary(catalog_dict)) = doc.get_object_mut(catalog_id) {
            catalog_dict.set("Outlines", Object::Reference(outline_id));
        }
    }
    // 重新编号doc的对象
    doc.renumber_objects();

    doc.compress();

    save_pdf_atomic(doc, save_path)?;
    Ok(())
}

fn save_pdf_atomic(mut doc: Document, save_path: &Path) -> anyhow::Result<()> {
    let expected_pages = doc.get_pages().len();
    ensure!(expected_pages > 0, "拒绝保存没有页面的 PDF");
    let mut temp =
        tempfile::NamedTempFile::new_in(save_path.parent().context("导出路径没有父目录")?)?;
    doc.save_to(temp.as_file_mut())
        .context("写入 PDF 临时文件失败")?;
    temp.as_file().sync_all()?;
    // Release the in-memory document before loading it again on memory-limited phones.
    drop(doc);
    let saved = Document::load(temp.path()).context("生成的 PDF 无法重新读取")?;
    ensure!(
        saved.get_pages().len() == expected_pages,
        "生成的 PDF 页数不完整"
    );
    validate_pdf(&saved)?;
    temp.persist(save_path)
        .map_err(|err| err.error)
        .context(format!("替换 PDF {} 失败", save_path.display()))?;
    Ok(())
}

fn validate_pdf(doc: &Document) -> anyhow::Result<()> {
    for page_id in doc.get_pages().values() {
        let content =
            Content::decode(&doc.get_page_content(*page_id).context("PDF 页面内容无效")?)?;
        let (resources, _) = doc.get_page_resources(*page_id)?;
        let resources = resources.context("PDF 页面缺少图片资源")?;
        let xobjects = doc.dereference(resources.get(b"XObject")?)?.1.as_dict()?;
        ensure!(!xobjects.is_empty(), "PDF 页面没有图片");
        for (_, object) in xobjects.iter() {
            let stream = doc.dereference(object)?.1.as_stream()?;
            ensure!(!stream.content.is_empty(), "PDF 图片数据为空");
        }
        let mut drawn_images = 0;
        for operation in content.operations.iter().filter(|op| op.operator == "Do") {
            let name = operation
                .operands
                .first()
                .context("PDF 图片绘制指令缺少资源名")?
                .as_name()?;
            let stream = doc.dereference(xobjects.get(name)?)?.1.as_stream()?;
            ensure!(
                stream.dict.get(b"Subtype")?.as_name()? == b"Image",
                "PDF 图片引用无效"
            );
            drawn_images += 1;
        }
        ensure!(drawn_images > 0, "PDF 页面缺少图片绘制指令");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reproduce the document the old merger wrote for an empty chapter list.
    fn legacy_empty_document() -> Document {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Count" => 0,
            "Kids" => Vec::<Object>::new(),
        });
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.adjust_zero_pages();
        doc.renumber_objects();
        doc.compress();
        doc
    }

    #[test]
    fn legacy_zero_page_pdf_cannot_replace_existing_export() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("result.pdf");
        create_pdf(vec![make_image(temp.path(), "good.jpg", 32)], &output).unwrap();
        let previous = std::fs::read(&output).unwrap();
        let empty = temp.path().join("legacy-empty.pdf");
        legacy_empty_document().save(&empty).unwrap();
        assert!(Document::load(&empty).unwrap().get_pages().is_empty());
        assert!(merge_pdf(vec![empty], &output).is_err());
        assert!(save_pdf_atomic(legacy_empty_document(), &output).is_err());
        assert_eq!(std::fs::read(output).unwrap(), previous);
    }

    /// Opt-in local diagnosis. Reads source files without changing their metadata.
    /// Outputs only structural statistics, never images or rendered PDF pages.
    #[test]
    #[ignore = "requires JM_LOCAL_COMIC_DIR and JM_LOCAL_EXPORT_DIR"]
    fn rebuild_local_comic_without_rendering() -> anyhow::Result<()> {
        let source = PathBuf::from(std::env::var("JM_LOCAL_COMIC_DIR")?).canonicalize()?;
        let output = PathBuf::from(std::env::var("JM_LOCAL_EXPORT_DIR")?).canonicalize()?;
        ensure!(
            !output.starts_with(&source) && !source.starts_with(&output),
            "诊断输出目录必须与源目录分开"
        );
        let metadata = std::fs::read_to_string(source.join("元数据.json"))?;
        let mut comic: Comic = serde_json::from_str(&metadata)?;
        // Avoid from_metadata's legacy migration: the source must remain read-only.
        comic.update_fields(&std::collections::HashMap::from([(comic.id, source)]))?;
        let mut chapters: Vec<_> = comic
            .chapter_infos
            .iter()
            .filter(|chapter| chapter.is_downloaded.unwrap_or(false))
            .collect();
        chapters.sort_by_key(|chapter| chapter.order);
        ensure!(!chapters.is_empty(), "没有已下载完成的章节可重建");
        let chapter_count = chapters.len();
        let mut image_count = 0;
        let mut chapter_pdfs = Vec::new();
        for chapter in chapters {
            let images = chapter_images(chapter)?;
            image_count += images.len();
            let pdf = output.join(format!(
                "chapter-{}-{}.pdf",
                chapter.order, chapter.chapter_id
            ));
            create_pdf(images, &pdf)?;
            chapter_pdfs.push(pdf);
        }
        let rebuilt = output.join("rebuilt.pdf");
        merge_pdf(chapter_pdfs, &rebuilt)?;
        let doc = Document::load(&rebuilt)?;
        ensure!(doc.get_pages().len() == image_count, "重建 PDF 页数不符");
        validate_pdf(&doc)?;
        println!(
            "comic_id={} completed_chapters={} validated_images={} rebuilt_pages={} rebuilt_bytes={}",
            comic.id,
            chapter_count,
            image_count,
            doc.get_pages().len(),
            std::fs::metadata(&rebuilt)?.len()
        );
        let legacy = output.join("legacy-empty.pdf");
        legacy_empty_document().save(&legacy)?;
        if let Ok(original) = std::env::var("JM_LOCAL_ORIGINAL_PDF") {
            let original_bytes = std::fs::read(original)?;
            let legacy_bytes = std::fs::read(legacy)?;
            println!(
                "legacy_empty_bytes={} original_bytes={} byte_identical={}",
                legacy_bytes.len(),
                original_bytes.len(),
                legacy_bytes == original_bytes
            );
        }
        Ok(())
    }

    fn make_image(dir: &Path, name: &str, width: u32) -> PathBuf {
        let path = dir.join(name);
        let image = image::RgbImage::from_fn(width, 48, |x, y| {
            image::Rgb([(x * 7) as u8, (y * 5) as u8, ((x + y) * 3) as u8])
        });
        if name.ends_with(".gif") {
            image::DynamicImage::ImageRgb8(image)
                .to_rgba8()
                .save(&path)
                .unwrap();
        } else {
            image.save(&path).unwrap();
        }
        path
    }

    #[test]
    fn single_chapter_pdf_keeps_all_image_resources() {
        let temp = tempfile::tempdir().unwrap();
        let images = ["0001.jpg", "0002.png", "0003.webp", "0004.gif"]
            .into_iter()
            .map(|name| make_image(temp.path(), name, 32))
            .collect();
        let chapter = temp.path().join("chapter.pdf");
        let merged = temp.path().join("merged.pdf");
        create_pdf(images, &chapter).unwrap();
        merge_pdf(vec![chapter], &merged).unwrap();
        let doc = Document::load(&merged).unwrap();
        assert_eq!(doc.get_pages().len(), 4);
        assert!(doc.objects.keys().all(|id| id.0 > 0));
        validate_pdf(&doc).unwrap();
        for page in doc.get_pages().values() {
            let content = Content::decode(&doc.get_page_content(*page).unwrap()).unwrap();
            let name = content
                .operations
                .iter()
                .find(|op| op.operator == "Do")
                .unwrap()
                .operands[0]
                .as_name()
                .unwrap();
            let (resources, _) = doc.get_page_resources(*page).unwrap();
            let images = doc
                .dereference(resources.unwrap().get(b"XObject").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            assert!(doc
                .dereference(images.get(name).unwrap())
                .unwrap()
                .1
                .as_stream()
                .is_ok());
        }
        // Optional output for validation by an independent renderer.
        if let Ok(dir) = std::env::var("JM_EXPORT_TEST_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(merged, Path::new(&dir).join("single-chapter.pdf")).unwrap();
        }
    }

    #[test]
    fn merged_chapters_preserve_page_order() {
        let temp = tempfile::tempdir().unwrap();
        let mut chapters = Vec::new();
        for width in [32, 64, 96] {
            let img = make_image(temp.path(), &format!("{width}.png"), width);
            let pdf = temp.path().join(format!("{width}.pdf"));
            create_pdf(vec![img], &pdf).unwrap();
            chapters.push(pdf);
        }
        let output = temp.path().join("merged.pdf");
        merge_pdf(chapters, &output).unwrap();
        let doc = Document::load(output).unwrap();
        let widths: Vec<_> = doc
            .get_pages()
            .values()
            .map(|id| {
                doc.get_object(*id)
                    .unwrap()
                    .as_dict()
                    .unwrap()
                    .get(b"MediaBox")
                    .unwrap()
                    .as_array()
                    .unwrap()[2]
                    .as_i64()
                    .unwrap()
            })
            .collect();
        assert_eq!(widths, vec![32, 64, 96]);
        validate_pdf(&doc).unwrap();
    }

    #[test]
    fn failed_export_preserves_existing_pdf() {
        let temp = tempfile::tempdir().unwrap();
        let good = make_image(temp.path(), "good.jpg", 32);
        let output = temp.path().join("result.pdf");
        create_pdf(vec![good], &output).unwrap();
        let previous = std::fs::read(&output).unwrap();
        let bad = temp.path().join("bad.jpg");
        std::fs::write(&bad, b"interrupted image").unwrap();
        assert!(create_pdf(vec![bad], &output).is_err());
        assert_eq!(std::fs::read(&output).unwrap(), previous);
        assert!(create_pdf(vec![], &output).is_err());
        assert!(merge_pdf(vec![], &output).is_err());
        assert_eq!(std::fs::read(output).unwrap(), previous);
    }

    #[test]
    fn invalid_draw_reference_does_not_replace_existing_pdf() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("result.pdf");
        create_pdf(vec![make_image(temp.path(), "good.jpg", 32)], &output).unwrap();
        let previous = std::fs::read(&output).unwrap();
        let mut doc = Document::load(&output).unwrap();
        let page = *doc.get_pages().values().next().unwrap();
        let content = Content {
            operations: vec![Operation::new(
                "Do",
                vec![Object::Name(b"missing".to_vec())],
            )],
        };
        let id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        doc.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", id);
        assert!(save_pdf_atomic(doc, &output).is_err());
        assert_eq!(std::fs::read(output).unwrap(), previous);
    }

    #[test]
    fn successful_export_replaces_existing_pdf() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("result.pdf");
        create_pdf(vec![make_image(temp.path(), "first.jpg", 32)], &output).unwrap();
        create_pdf(vec![make_image(temp.path(), "second.png", 96)], &output).unwrap();
        let doc = Document::load(output).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
        let page = *doc.get_pages().values().next().unwrap();
        assert_eq!(
            doc.get_object(page)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap()[2]
                .as_i64()
                .unwrap(),
            96
        );
    }

    #[test]
    fn chapter_validation_rejects_empty_missing_and_corrupt_pages() {
        let temp = tempfile::tempdir().unwrap();
        let mut chapter = ChapterInfo {
            chapter_id: 1,
            chapter_title: "test".into(),
            order: 1,
            page_count: Some(2),
            is_downloaded: Some(true),
            chapter_download_dir: Some(temp.path().into()),
        };
        assert!(chapter_images(&chapter).is_err());
        make_image(temp.path(), "0001.jpg", 32);
        assert!(chapter_images(&chapter).is_err());
        let second = make_image(temp.path(), "0002.jpeg", 32);
        assert_eq!(chapter_images(&chapter).unwrap().len(), 2);
        std::fs::write(second, b"corrupt").unwrap();
        chapter.page_count = None; // Legacy downloads still get corruption checks.
        assert!(chapter_images(&chapter).is_err());
    }

    #[test]
    fn concurrent_export_is_rejected_until_previous_request_finishes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pdf");
        let guard = ExportGuard::acquire(path.clone()).unwrap();
        assert!(ExportGuard::acquire(path.clone()).is_err());
        drop(guard);
        assert!(ExportGuard::acquire(path).is_ok());
    }
}
