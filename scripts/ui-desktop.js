// Run after ui-fixtures.js on ?platform=windows, then resize to a desktop viewport.
// prettier-ignore
async (page) => {
  const errors = []
  page.on('pageerror', error => errors.push(String(error)))
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.getByRole('textbox', { name: '搜索关键词 / JM 号', exact: true }).fill('999001')
  await page.getByRole('button', { name: '搜索漫画', exact: true }).click()
  await page.getByRole('button', { name: '导出 PDF', exact: true }).waitFor()
  for (const size of [{ width: 1280, height: 800 }, { width: 800, height: 600 }]) {
    await page.setViewportSize(size)
    const box = await page.getByRole('button', { name: '导出 PDF', exact: true }).boundingBox()
    if (!box || box.x < 0 || box.y < 0 || box.y + box.height > size.height) throw new Error('Desktop export is outside the viewport')
    await page.screenshot({ path: 'output/playwright/desktop-' + size.width + '.png' })
  }
  await page.getByRole('button', { name: '导出 PDF', exact: true }).click()
  await page.getByText('导出进度', { exact: true }).click()
  await page.getByRole('button', { name: '打开导出位置', exact: true }).first().waitFor()
  const nativeCalls = await page.evaluate(() => window.__JM_TEST_CALLS__.filter(c => c.command === 'open_exported_files'))
  if (nativeCalls.length) throw new Error('Desktop export should not automatically open an iOS sharing sheet')
  if (errors.length) throw new Error(errors.join('\n'))
  console.log('Desktop chapter actions and export passed at 1280×800 and 800×600.')
}
