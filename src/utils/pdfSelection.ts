import type { ChapterInfo } from '../bindings'

export type PdfScope = 'whole' | 'chapters'
export type PdfLayout = 'merged' | 'separate'

export function orderedChapters(chapters: ChapterInfo[]): ChapterInfo[] {
  return [...chapters].sort((a, b) => a.order - b.order)
}

export function selectedChapterIds(chapters: ChapterInfo[], ids: number[]): number[] {
  const chosen = new Set(ids)
  return orderedChapters(chapters)
    .filter((chapter) => chapter.isDownloaded === true && chosen.has(chapter.chapterId))
    .map((chapter) => chapter.chapterId)
}

export function pdfRequest(chapters: ChapterInfo[], scope: PdfScope, ids: number[], layout: PdfLayout) {
  if (scope === 'whole') {
    if (!chapters.length || chapters.some((chapter) => chapter.isDownloaded !== true)) {
      throw new Error('整本操作需要所有章节下载完成，请改选已下载章节。')
    }
    return { chapterIds: null, merge: true, count: chapters.length }
  }
  const chapterIds = selectedChapterIds(chapters, ids)
  if (!chapterIds.length) throw new Error('请至少选择一个已下载章节。')
  return { chapterIds, merge: layout === 'merged', count: chapterIds.length }
}
