import { commands, type Comic } from '../bindings'
import { useStore } from '../store'
import { useMessage } from 'naive-ui'

export function useComicExport() {
  const store = useStore()
  const message = useMessage()

  async function openFiles(paths: string[], preview = false) {
    if (!paths.length) {
      message.warning('请先完成导出')
      return
    }
    try {
      const result = await commands.openExportedFiles(paths, preview)
      if (result.status === 'error') message.error(result.error.err_message)
    } catch (error) {
      message.error(String(error))
    }
  }

  async function openPdf(comic: Comic, preview = true) {
    try {
      const result = await commands.getComicPdfPath(comic)
      if (result.status === 'error') {
        message.error(result.error.err_message)
        return
      }
      await openFiles(result.data ? [result.data] : [], preview)
    } catch (error) {
      message.error(String(error))
    }
  }

  async function shareCbz(comic: Comic) {
    try {
      const result = await commands.getComicCbzPaths(comic)
      if (result.status === 'error') {
        message.error(result.error.err_message)
        return
      }
      await openFiles(result.data)
    } catch (error) {
      message.error(String(error))
    }
  }

  async function exportComic(comic: Comic, format: 'pdf' | 'cbz', share = false) {
    const key = `${comic.id}:${format}`
    if (store.exportingComics.has(key)) {
      message.info('正在导出，请稍候')
      return false
    }
    store.exportingComics.add(key)
    store.progressesPaneTabName = 'export'
    try {
      const result = format === 'pdf' ? await commands.exportPdf(comic) : await commands.exportCbz(comic)
      if (result.status === 'error') {
        message.error(result.error.err_message)
        return false
      }
      message.success(`${format.toUpperCase()} 导出完成`)
      if (share && store.runtimePlatform === 'ios') {
        if (format === 'pdf') await openPdf(comic, false)
        else await shareCbz(comic)
      }
      return true
    } catch (error) {
      message.error(String(error))
      return false
    } finally {
      store.exportingComics.delete(key)
    }
  }

  return { exportComic, openPdf, shareCbz, openFiles }
}
