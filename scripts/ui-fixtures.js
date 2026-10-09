// Run with: playwright-cli -s=jm-ui run-code --filename scripts/ui-fixtures.js
// This bridge uses fictional data; it never contacts a comic API or reads local comics.
// prettier-ignore
async (page) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.route('**/media/albums/**', (route) =>
    route.fulfill({
      contentType: 'image/svg+xml',
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="160"><rect width="120" height="160" fill="#ffd8a8"/><circle cx="90" cy="45" r="40" fill="#ffb562"/><path d="M0 110L60 60L120 140V160H0" fill="#ec8729"/><text x="15" y="142" font-size="18" fill="white">JM DEMO</text></svg>',
    }),
  )
  const platform = new URL(page.url()).searchParams.get('platform') || 'ios'
  await page.addInitScript((platform) => {
    Object.defineProperty(navigator, 'userAgent', {
      configurable: true,
      value:
        platform === 'ios'
          ? 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148'
          : 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130.0.0.0 Safari/537.36',
    })
    const comic = {
      id: 999001,
      name: '漫长旅途中的故事：用于检查手机布局的示例漫画',
      author: ['示例作者'],
      tags: ['旅行', '故事', '布局测试'],
      chapterInfos: Array.from({ length: 64 }, (_, i) => ({
        chapterId: 999100 + i,
        chapterTitle: `第 ${i + 1} 话${i % 4 === 0 ? ' · 很长的章节名称也应该完整显示' : ''}`,
        order: i + 1,
        isDownloaded: i < 4,
        pageCount: i < 4 ? 24 : null,
        chapterDownloadDir: i < 4 ? `/exports/source/${i}` : null,
      })),
      comicDownloadDir: '/downloads/demo',
      isDownloaded: true,
      addtime: '',
      description: '',
      total_views: '1',
      likes: '1',
      series_id: '',
      comment_total: '0',
      works: [],
      actors: [],
      related_list: [],
      liked: false,
      is_favorite: false,
      is_aids: false,
    }
    const cards = Array.from({ length: 30 }, (_, i) => ({
      id: 999001 + i,
      name: i ? `示例漫画 ${i + 1}` : comic.name,
      author: '示例作者',
      image: '',
      category: { id: '1', title: '故事' },
      categorySub: { id: '1', title: '旅行' },
      isDownloaded: true,
      comicDownloadDir: '/downloads/demo',
      liked: false,
      isFavorite: false,
      updateAt: 0,
    }))
    const config = {
      username: '',
      password: '',
      downloadDir: '/downloads',
      exportDir: '/exports',
      downloadFormat: 'Jpeg',
      dirFmt: '{comic_title}/{chapter_title}',
      proxyMode: 'NoProxy',
      proxyHost: '',
      proxyPort: 8080,
      enableFileLogger: false,
      chapterConcurrency: 2,
      chapterDownloadIntervalSec: 0,
      imgConcurrency: 4,
      imgDownloadIntervalSec: 0,
      downloadAllFavoritesIntervalSec: 0,
      updateDownloadedComicsIntervalSec: 0,
      apiDomainMode: 'Domain1',
      customApiDomain: '',
      shouldDownloadCover: true,
    }
    let nextId = 1
    const callbacks = new Map()
    const listeners = new Map()
    const tasks = new Map()
    const deletedComics = new Set()
    const deletedFormats = new Map()
    window.__JM_TEST_CALLS__ = []
    const emit = (event, payload) => {
      for (const [id, listener] of listeners)
        if (listener.event === event) callbacks.get(listener.handler)?.({ event, id, payload })
    }
    window.__JM_TEST_EMIT__ = emit
    const update = (id, state) => {
      const task = tasks.get(id)
      if (task) {
        task.state = state
        emit('download-task-event', {
          event: 'Update',
          data: { chapterId: id, state, downloadedImgCount: 4, totalImgCount: 24 },
        })
      }
    }
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
      transformCallback: (callback) => {
        const id = nextId++
        callbacks.set(id, callback)
        return id
      },
      invoke: async (command, args = {}) => {
        window.__JM_TEST_CALLS__.push({ command, args })
        if (command === 'plugin:event|listen') {
          const id = nextId++
          listeners.set(id, args)
          return id
        }
        if (command === 'plugin:event|unlisten') {
          listeners.delete(args.eventId)
          return null
        }
        if (command === 'get_config') return config
        if (command === 'get_storage_info') return { downloadDir: config.downloadDir, exportDir: config.exportDir, configPath: '/current-container/Documents/config.json', migrationWarnings: [] }
        if (command === 'get_runtime_platform') return platform
        if (command === 'get_logs_dir_size') return 1024
        if (command === 'plugin:app|version') return '0.17.0'
        if (command === 'get_weekly_info')
          return {
            categories: [{ id: '1', title: '本周', time: '本周推荐' }],
            type: [
              { id: '1', title: '热门' },
              { id: '2', title: '新作' },
            ],
          }
        if (command === 'get_weekly')
          return {
            total: 30,
            list: cards.map((c) => ({
              ...c,
              category_sub: c.categorySub,
              is_downloaded: c.isDownloaded,
              comic_download_dir: c.comicDownloadDir,
            })),
          }
        if (command === 'search')
          return /^\d+$/.test(args.keyword)
            ? { Comic: comic }
            : { SearchResult: { searchQuery: args.keyword, total: 160, content: cards } }
        if (command === 'get_comic' || command === 'get_synced_comic') return comic
        if (command.startsWith('get_synced_comic_in_')) return args.comic
        if (command === 'get_downloaded_comics') {
          const count = Math.min(60, Math.max(2, Number(new URL(location.href).searchParams.get('books') || 2)))
          return Array.from({ length: count }, (_, i) => i ? { ...comic, id: 999001 + i, name: i === 1 ? '第二本示例漫画' : '示例漫画 ' + (i + 1) } : comic).filter(c => !deletedComics.has(c.id))
        }
        if (command === 'get_comic_file_status') {
          const deleted = deletedFormats.get(args.comic.id) || new Set()
          return { pdfCount: deleted.has('pdf') ? 0 : 5, cbzCount: deleted.has('cbz') ? 0 : 4, hasPdf: !deleted.has('pdf') }
        }
        if (command === 'delete_comic_files') {
          if (args.kind === 'all') deletedComics.add(args.comic.id)
          else {
            if (!deletedFormats.has(args.comic.id)) deletedFormats.set(args.comic.id, new Set())
            deletedFormats.get(args.comic.id).add(args.kind)
          }
          return { removedFiles: args.kind === 'pdf' ? 5 : args.kind === 'cbz' ? 4 : 25, removedComic: args.kind === 'all' }
        }
        if (command === 'get_comic_pdf_path') return deletedFormats.get(args.comic.id)?.has('pdf') ? null : '/exports/demo.pdf'
        if (command === 'get_comic_cbz_paths') return deletedFormats.get(args.comic.id)?.has('cbz') ? [] : ['/exports/demo.cbz']
        if (command === 'create_download_task') {
          const chapterInfo = comic.chapterInfos.find((c) => c.chapterId === args.chapterId)
          tasks.set(args.chapterId, { state: 'Downloading' })
          emit('download-task-event', {
            event: 'Create',
            data: { state: 'Downloading', comic, chapterInfo, downloadedImgCount: 4, totalImgCount: 24 },
          })
        }
        if (command === 'pause_download_task') update(args.chapterId, 'Paused')
        if (command === 'resume_download_task') update(args.chapterId, 'Downloading')
        if (command === 'cancel_download_task') update(args.chapterId, 'Cancelled')
        if (command === 'export_pdf') {
          deletedFormats.get(args.comic.id)?.delete('pdf')
          emit('export-pdf-event', {
            event: 'CreateStart',
            data: { uuid: 'create-demo', comicTitle: comic.name, total: 4 },
          })
          emit('export-pdf-event', {
            event: 'CreateEnd',
            data: { uuid: 'create-demo', chapterExportDir: '/exports/pdf' },
          })
          emit('export-pdf-event', { event: 'MergeStart', data: { uuid: 'merge-demo', comicTitle: comic.name } })
          emit('export-pdf-event', {
            event: 'MergeEnd',
            data: { uuid: 'merge-demo', chapterExportDir: '/exports/demo.pdf' },
          })
        }
        if (command === 'export_cbz') {
          deletedFormats.get(args.comic.id)?.delete('cbz')
          emit('export-cbz-event', { event: 'Start', data: { uuid: 'cbz-demo', comicTitle: comic.name, total: 4 } })
          emit('export-cbz-event', { event: 'End', data: { uuid: 'cbz-demo', chapterExportDir: '/exports/cbz' } })
        }
        if (command === 'login') return { username: '示例账号', photo: '' }
        if (command === 'get_favorite_folder') return { list: cards, folderList: [], total: 30, count: 30 }
        return null
      },
    }
  }, platform)
  await page.reload()
  await page.getByRole('button', { name: '搜索漫画', exact: true }).waitFor()
  console.log('Fictional iOS bridge initialized at 390 × 844.')
}
