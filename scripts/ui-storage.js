// Run after ui-fixtures.js in its own session. All paths and books are fictional.
// prettier-ignore
async (page) => {
  const errors = []
  const captureError = error => errors.push(String(error))
  page.on('pageerror', captureError)
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke
    const root = '/private/var/mobile/Containers/Data/Application/fictional-host/Documents/Data/Application/fictional-guest/Documents'
    window.__JM_STORAGE_ROOT__ = root
    window.__JM_STORAGE_FAILURE__ = true
    window.__JM_STORAGE_INFO_FAILURE__ = false
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === 'get_downloaded_comics' && window.__JM_STORAGE_FAILURE__) {
        throw { err_title: '读取书库失败', err_message: `无法访问 ${root}/漫画下载\nOperation not permitted (os error 1)` }
      }
      if (command === 'get_storage_info') {
        if (window.__JM_STORAGE_INFO_FAILURE__) throw { err_title: '读取存储位置失败', err_message: 'fictional storage information failure' }
        return { downloadDir: `${root}/漫画下载`, exportDir: `${root}/漫画导出`, configPath: `${root.slice(0, -'/Documents'.length)}/Library/Application Support/com.lanyeeee.jmcomic-downloader/config.json`, migrationWarnings: ['虚构迁移提示：同名文件已保留，请检查旧目录。'] }
      }
      return original(command, args)
    }
  })
  await page.getByRole('button', { name: '书库', exact: true }).click()
  const alert = page.getByRole('alert')
  await alert.getByText('书库读取失败', { exact: true }).waitFor()
  if (await page.getByText('书库还没有漫画，下载完成后会显示在这里', { exact: true }).isVisible()) throw new Error('Access failure was reported as an empty library')
  await page.setViewportSize({ width: 320, height: 568 })
  if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) throw new Error('Storage error overflows a narrow phone')
  await alert.getByRole('button', { name: '重新读取书库', exact: true }).scrollIntoViewIfNeeded()
  await page.screenshot({ path: 'output/playwright/storage-error-320.png' })
  await page.evaluate(() => { window.__JM_STORAGE_FAILURE__ = false })
  await alert.getByRole('button', { name: '重新读取书库', exact: true }).click()
  await page.locator('.library-card').first().waitFor()
  if (await page.locator('.library-card').count() !== 2) throw new Error('Retry did not restore the library')
  await page.evaluate(() => { window.__JM_STORAGE_FAILURE__ = true })
  await page.getByRole('button', { name: '刷新书库', exact: true }).click()
  await alert.waitFor()
  if (await page.locator('.library-card').count() !== 2) throw new Error('Failed refresh erased loaded books')
  await page.evaluate(() => { window.__JM_STORAGE_FAILURE__ = false })
  await alert.getByRole('button', { name: '重新读取书库', exact: true }).click()
  await alert.waitFor({ state: 'hidden' })
  await page.getByRole('button', { name: '更多选项', exact: true }).click()
  await page.getByText('设置', { exact: true }).click()
  const dialog = page.getByRole('dialog')
  await dialog.waitFor()
  const root = await page.evaluate(() => window.__JM_STORAGE_ROOT__)
  await dialog.getByText('虚构迁移提示：同名文件已保留，请检查旧目录。', { exact: true }).waitFor()
  for (const [label, expected] of [['漫画下载目录', root + '/漫画下载'], ['PDF / CBZ 导出目录', root + '/漫画导出'], ['配置文件位置', root.slice(0, -'/Documents'.length) + '/Library/Application Support/com.lanyeeee.jmcomic-downloader/config.json']]) {
    const input = dialog.getByRole('textbox', { name: label, exact: true })
    await input.waitFor()
    if (await input.inputValue() !== expected || await input.getAttribute('readonly') === null) throw new Error(`Incorrect or editable storage field: ${label}`)
  }
  for (const size of [{ width: 320, height: 568 }, { width: 390, height: 844 }, { width: 844, height: 390 }]) {
    await page.setViewportSize(size)
    await dialog.getByRole('textbox', { name: '漫画下载目录', exact: true }).scrollIntoViewIfNeeded()
    const box = await dialog.boundingBox()
    if (!box || box.x < 0 || box.y < 0 || box.y + box.height > size.height + 1) throw new Error(`Storage dialog does not fit ${size.width}x${size.height}`)
    if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) throw new Error('Storage settings overflow horizontally')
    await page.screenshot({ path: `output/playwright/storage-settings-${size.width}.png` })
    const done = dialog.getByRole('button', { name: '完成', exact: true })
    const bounds = await done.boundingBox()
    if (!bounds || bounds.y < 0 || bounds.y + bounds.height > size.height) throw new Error('Settings completion is unreachable')
  }
  await page.setViewportSize({ width: 390, height: 844 })
  await dialog.getByRole('button', { name: '完成', exact: true }).click()
  await dialog.waitFor({ state: 'hidden' })
  await page.evaluate(() => { window.__JM_STORAGE_INFO_FAILURE__ = true })
  await page.getByRole('button', { name: '更多选项', exact: true }).click()
  await page.getByText('设置', { exact: true }).click()
  await dialog.getByText('fictional storage information failure', { exact: true }).waitFor()
  await dialog.getByRole('textbox', { name: '章节并发数', exact: true }).fill('3')
  await dialog.getByRole('textbox', { name: '章节并发数', exact: true }).press('Tab')
  await page.waitForFunction(() => window.__JM_TEST_CALLS__.some(c => c.command === 'save_config' && c.args.config.chapterConcurrency === 3))
  await dialog.getByRole('button', { name: '完成', exact: true }).click()
  page.off('pageerror', captureError)
  if (errors.length) throw new Error(errors.join('\n'))
  console.log(JSON.stringify({ status: 'passed', cases: ['permission error is visible', 'retry recovers library', 'failed refresh preserves books', 'Documents paths are readonly and copyable', 'settings fit 320/390/landscape', 'settings still save when storage information fails'], pageErrors: errors.length }))
}
