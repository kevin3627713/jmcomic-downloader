import { useStore } from '../store'
import type { Comic } from '../bindings'

export function useLibraryRemoval() {
  const store = useStore()
  function removeComicState(id: number) {
    // Replace objects so a refresh started before deletion cannot update the
    // objects still displayed in the current view.
    if (store.pickedComic?.id === id) {
      store.pickedComic = {
        ...store.pickedComic,
        isDownloaded: false,
        comicDownloadDir: null,
        chapterInfos: store.pickedComic.chapterInfos.map((chapter) => ({
          ...chapter,
          isDownloaded: false,
          chapterDownloadDir: null,
          pageCount: null,
        })),
      } as Comic
    }
    for (const [chapterId, progress] of store.progresses) {
      if (progress.comic.id === id) store.progresses.delete(chapterId)
    }
    if (store.searchResult)
      store.searchResult.content = store.searchResult.content.map((comic) =>
        comic.id === id ? { ...comic, isDownloaded: false, comicDownloadDir: '' } : comic,
      )
    if (store.getFavoriteResult)
      store.getFavoriteResult.list = store.getFavoriteResult.list.map((comic) =>
        comic.id === id ? { ...comic, isDownloaded: false, comicDownloadDir: '' } : comic,
      )
    if (store.getWeeklyResult)
      store.getWeeklyResult.list = store.getWeeklyResult.list.map((comic) =>
        comic.id === id ? { ...comic, is_downloaded: false, comic_download_dir: '' } : comic,
      )
  }
  return { removeComicState }
}
