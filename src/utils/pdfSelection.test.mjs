// Run: node --test src/utils/pdfSelection.test.mjs
import { readFileSync } from 'node:fs'
import { Buffer } from 'node:buffer'
import assert from 'node:assert/strict'
import test from 'node:test'
import ts from 'typescript'

const source = readFileSync(new URL('./pdfSelection.ts', import.meta.url), 'utf8')
const javascript = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
}).outputText
const { pdfRequest, orderedChapters, selectedChapterIds } = await import(
  `data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`
)
const chapters = [
  { chapterId: 20, order: 2, chapterTitle: '第二章', isDownloaded: true },
  { chapterId: 30, order: 3, chapterTitle: '第三章', isDownloaded: false },
  { chapterId: 10, order: 1, chapterTitle: '第一章', isDownloaded: true },
]

test('selection preserves canonical chapter order and deduplicates IDs', () => {
  assert.deepEqual(selectedChapterIds(chapters, [20, 10, 20]), [10, 20])
  assert.deepEqual(chapters.map((chapter) => chapter.chapterId), [20, 30, 10])
  assert.deepEqual(orderedChapters(chapters).map((chapter) => chapter.chapterId), [10, 20, 30])
})
test('stale, unavailable and foreign IDs are not submitted', () => {
  assert.deepEqual(selectedChapterIds(chapters, [30, 999, 20]), [20])
  assert.throws(() => pdfRequest(chapters, 'chapters', [30, 999], 'merged'), /至少选择/)
})
test('whole book refuses incomplete or empty chapters', () => {
  assert.throws(() => pdfRequest(chapters, 'whole', [], 'merged'), /所有章节/)
  assert.throws(() => pdfRequest([], 'whole', [], 'merged'), /所有章节/)
})
test('whole book always produces one combined PDF', () => {
  assert.deepEqual(pdfRequest(chapters.map((chapter) => ({ ...chapter, isDownloaded: true })), 'whole', [], 'separate'), {
    chapterIds: null, merge: true, count: 3,
  })
})
test('selected single and multiple chapters carry requested layout', () => {
  assert.deepEqual(pdfRequest(chapters, 'chapters', [20], 'merged'), { chapterIds: [20], merge: true, count: 1 })
  assert.deepEqual(pdfRequest(chapters, 'chapters', [20, 10], 'separate'), { chapterIds: [10, 20], merge: false, count: 2 })
  assert.deepEqual(pdfRequest(chapters, 'chapters', [20, 10], 'merged'), { chapterIds: [10, 20], merge: true, count: 2 })
})
