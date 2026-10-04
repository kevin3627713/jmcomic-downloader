// Run after ui-fixtures.js at ?platform=windows. Uses fictional files and comics only.
// Refresh promises are held explicitly to reproduce completion / export interleavings.
// prettier-ignore
async (page) => {
  const errors = []
  page.on('pageerror', (error) => errors.push(String(error)))
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke
    // Tauri serializes Vue proxies across IPC; reproduce that copy here.
    const ipcClone = value => JSON.parse(JSON.stringify(value))
    const books = new Map()
    const pending = []
    window.__JM_RACE_PENDING__ = pending
    window.__JM_RACE_REJECT_NEXT__ = false
    window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
      if (command === 'search' && /^99900[123]$/.test(args.keyword)) {
        window.__JM_TEST_CALLS__.push({ command, args: ipcClone(args) })
        const base = (await invoke('get_comic', { comicId: 999001 }))
        const id = Number(args.keyword)
        if (!books.has(id)) {
          const count = id === 999002 ? 2 : 1
          books.set(id, {
            ...structuredClone(base), id, name: `完成竞态示例 ${id}`,
            isDownloaded: false, comicDownloadDir: '/downloads/completion-' + id,
            chapterInfos: Array.from({ length: count }, (_, i) => ({
              chapterId: id * 10 + i, chapterTitle: `第${i + 1}话`, order: i + 1,
              isDownloaded: false, chapterDownloadDir: null, pageCount: null,
            })),
          })
        }
        return { Comic: structuredClone(books.get(id)) }
      }
      if (command === 'create_download_task') {
        window.__JM_TEST_CALLS__.push({ command, args: ipcClone(args) })
        const comic = ipcClone(args.comic)
        for (const chapter of comic.chapterInfos)
          chapter.chapterDownloadDir = comic.comicDownloadDir + '/chapter-' + chapter.chapterId
        window.__JM_TEST_EMIT__('download-task-event', {
          event: 'Create', data: {
            state: 'Downloading', comic,
            chapterInfo: structuredClone(comic.chapterInfos.find(c => c.chapterId === args.chapterId)),
            downloadedImgCount: 0, totalImgCount: 51,
          },
        })
        return null
      }
      if (command === 'get_synced_comic') {
        window.__JM_TEST_CALLS__.push({ command, args: ipcClone(args) })
        if (window.__JM_RACE_REJECT_NEXT__) {
          window.__JM_RACE_REJECT_NEXT__ = false
          throw new Error('Expected test-only refresh failure')
        }
        const snapshot = ipcClone(args.comic)
        return new Promise(resolve => pending.push({ comicId: args.comic.id, snapshot, resolve }))
      }
      return invoke(command, ipcClone(args))
    }
  })

  const openBook = async (id) => {
    await page.locator('.n-tabs-tab').filter({ hasText: /^搜索$/ }).click()
    await page.getByRole('textbox', { name: '搜索关键词 / JM 号', exact: true }).fill(String(id))
    await page.getByRole('button', { name: '搜索漫画', exact: true }).click()
    await page.getByRole('heading', { name: `完成竞态示例 ${id}`, exact: true }).waitFor()
  }
  const downloadAll = async () => {
    await page.getByRole('button', { name: '全选', exact: true }).click()
    await page.getByRole('button', { name: /^下载所选/ }).click()
  }
  const complete = async (chapterIds) => page.evaluate((ids) => {
    for (const chapterId of ids) window.__JM_TEST_EMIT__('download-task-event', {
      event: 'Update', data: { chapterId, state: 'Completed', downloadedImgCount: 51, totalImgCount: 51 },
    })
  }, chapterIds)
  const assertExport = async (id, count) => {
    await page.getByRole('button', { name: '导出 PDF', exact: true }).click()
    await page.waitForFunction(({ id, count }) => {
      const call = window.__JM_TEST_CALLS__.filter(c => c.command === 'export_pdf' && c.args.comic.id === id).at(-1)
      return call && call.args.comic.chapterInfos.filter(c => c.isDownloaded && c.pageCount === 51 && c.chapterDownloadDir).length === count
    }, { id, count })
  }
  const resolveRefresh = async (index) => page.evaluate(async (index) => {
    const refresh = window.__JM_RACE_PENDING__[index]
    if (!refresh) throw new Error('Missing held refresh ' + index)
    refresh.resolve(structuredClone(refresh.snapshot))
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))
  }, index)

  // A single-chapter export is available before the refresh promise resolves.
  await openBook(999001)
  await downloadAll()
  await complete([9990010])
  await page.getByText('共 1 章 · 已下载 1 章', { exact: true }).waitFor()
  await assertExport(999001, 1)
  const held = await page.evaluate(() => window.__JM_RACE_PENDING__.length)
  if (held !== 1) throw new Error('Expected one unresolved completion refresh')

  // An old response must not reopen the previous comic after navigation.
  await openBook(999002)
  await resolveRefresh(0)
  await page.getByRole('heading', { name: '完成竞态示例 999002', exact: true }).waitFor()

  // Two completions can refresh concurrently and resolve in the opposite order.
  await downloadAll()
  await complete([9990020, 9990021])
  await page.getByText('共 2 章 · 已下载 2 章', { exact: true }).waitFor()
  await resolveRefresh(2)
  await resolveRefresh(1)
  await page.getByText('共 2 章 · 已下载 2 章', { exact: true }).waitFor()
  await assertExport(999002, 2)

  // A rejected background refresh cannot undo completion or prevent export.
  await openBook(999003)
  await downloadAll()
  await page.evaluate(() => { window.__JM_RACE_REJECT_NEXT__ = true })
  await complete([9990030])
  await page.getByText('共 1 章 · 已下载 1 章', { exact: true }).waitFor()
  await assertExport(999003, 1)
  if (errors.length) throw new Error(errors.join('\n'))
  console.log('Completion race checks passed: immediate single-chapter export, navigation during refresh, reversed refresh order, and rejected refresh.')
}
