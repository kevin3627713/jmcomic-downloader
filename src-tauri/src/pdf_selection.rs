//! Selection rules kept independent of Tauri so they can be regression-tested on any host.
use std::collections::HashSet;

/// Tuple fields: chapter ID, book order, downloaded. Return indices in book order.
pub fn select_chapters(
    chapters: &[(i64, i64, bool)],
    ids: Option<&[i64]>,
) -> Result<Vec<usize>, String> {
    if chapters.is_empty() {
        return Err("没有可导出的章节".into());
    }
    let requested = ids.map(|values| values.iter().copied().collect::<HashSet<_>>());
    if requested.as_ref().is_some_and(HashSet::is_empty) {
        return Err("请至少选择一章".into());
    }
    if let Some(requested) = &requested {
        if requested
            .iter()
            .any(|id| !chapters.iter().any(|c| c.0 == *id))
        {
            return Err("所选章节不属于此漫画，请刷新后重试".into());
        }
    }
    let mut selected = chapters
        .iter()
        .enumerate()
        .filter(|(_, c)| requested.as_ref().is_none_or(|set| set.contains(&c.0)))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if selected.iter().any(|index| !chapters[*index].2) {
        return Err(if ids.is_none() {
            "整本尚未下载完整，请先下载全部章节，或选择已下载章节"
        } else {
            "所选章节尚未下载完整，请刷新后重试"
        }
        .into());
    }
    selected.sort_by_key(|index| (chapters[*index].1, *index));
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    const CHAPTERS: &[(i64, i64, bool)] = &[(20, 2, true), (10, 1, true), (30, 3, false)];
    #[test]
    fn selection_is_deduplicated_and_sorted_in_book_order() {
        assert_eq!(
            select_chapters(CHAPTERS, Some(&[20, 10, 20])).unwrap(),
            vec![1, 0]
        );
    }
    #[test]
    fn explicit_single_chapter_is_allowed_with_partial_book() {
        assert_eq!(select_chapters(CHAPTERS, Some(&[20])).unwrap(), vec![0]);
    }
    #[test]
    fn rejects_empty_unknown_and_unavailable_selection() {
        for ids in [&[][..], &[999][..], &[30][..], &[10, 30][..]] {
            assert!(select_chapters(CHAPTERS, Some(ids)).is_err());
        }
    }
    #[test]
    fn whole_book_requires_every_chapter() {
        assert!(select_chapters(CHAPTERS, None).is_err());
        assert!(select_chapters(&[], None).is_err());
        assert_eq!(select_chapters(&CHAPTERS[..2], None).unwrap(), vec![1, 0]);
    }
    #[test]
    fn equal_order_keeps_book_sequence() {
        assert_eq!(
            select_chapters(&[(9, 1, true), (2, 1, true)], None).unwrap(),
            vec![0, 1]
        );
    }
}
