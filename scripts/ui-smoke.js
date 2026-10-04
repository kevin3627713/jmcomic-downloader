// Run after ui-fixtures.js using playwright-cli run-code --filename scripts/ui-smoke.js
// prettier-ignore
async (page) => {
  const assert = (condition, message) => {
    if (!condition) throw new Error(message)
  }
  const sizes = [
    { width: 320, height: 568 },
    { width: 390, height: 844 },
    { width: 844, height: 390 },
  ]
  await page.setViewportSize(sizes[0])
  await page.getByRole('button', { name: '更多选项', exact: true }).click()
  await page.getByText('设置', { exact: true }).click()
  const dialog = page.getByRole('dialog')
  await dialog.waitFor()
  const input = page.getByRole('textbox', { name: '章节并发数', exact: true })
  await input.fill('3')
  await input.press('Tab')
  await page.waitForFunction(() =>
    window.__JM_TEST_CALLS__.some((c) => c.command === 'save_config' && c.args.config.chapterConcurrency === 3),
  )
  const numberBounds = await input.boundingBox()
  assert(numberBounds.width > 150 && numberBounds.height >= 40, 'Narrow settings inputs must have usable dimensions')
  const dialogBounds = await dialog.boundingBox()
  assert(
    dialogBounds.x >= 0 && dialogBounds.y >= 0 && dialogBounds.y + dialogBounds.height <= 568,
    'Settings dialog must fit the visible viewport',
  )
  await input.blur()
  await page.waitForFunction(() => !document.querySelector('.n-message'))
  await page.screenshot({ path: 'output/playwright/settings-320.png' })
  await page.getByRole('button', { name: '完成', exact: true }).click()
  await page.getByRole('textbox', { name: '搜索关键词 / JM 号', exact: true }).fill('999001')
  await page.getByRole('button', { name: '搜索漫画', exact: true }).click()
  await page.getByRole('button', { name: '返回列表', exact: true }).waitFor()
  for (const size of sizes) {
    await page.setViewportSize(size)
    await page.waitForFunction(
      ({ height }) => Math.abs(document.querySelector('.app-shell').getBoundingClientRect().height - height) < 2,
      size,
    )
    const layout = await page.evaluate(() => {
      const shell = document.querySelector('.app-shell')
      const actions = document.querySelector('.chapter-actions')
      const scroll = document.querySelector('.chapter-grid').closest('.pane-scroll')
      const r = actions.getBoundingClientRect()
      return {
        mobile: shell.classList.contains('mobile-layout'),
        overflow: document.documentElement.scrollWidth > innerWidth,
        top: r.top,
        bottom: r.bottom,
        scrollHeight: scroll.clientHeight,
        total: scroll.scrollHeight,
      }
    })
    assert(
      layout.mobile && !layout.overflow && layout.top >= 0 && layout.bottom <= size.height,
      'Chapter actions must be on screen at ' + JSON.stringify(size),
    )
    assert(
      layout.scrollHeight > 30 && layout.total > layout.scrollHeight,
      'Chapters need an independently scrollable area',
    )
    console.log('Chapter viewport:', JSON.stringify(size), JSON.stringify(layout))
    await page.screenshot({ path: 'output/playwright/chapter-' + size.width + '.png' })
  }
  await page.setViewportSize(sizes[1])
  await page.getByRole('button', { name: '导出 PDF', exact: true }).click()
  await page.waitForFunction(() =>
    window.__JM_TEST_CALLS__.some(
      (c) =>
        c.command === 'open_exported_files' && c.args.preview === false && c.args.paths.includes('/exports/demo.pdf'),
    ),
  )
  await page.getByRole('button', { name: '查看 PDF', exact: true }).click()
  await page.waitForFunction(() =>
    window.__JM_TEST_CALLS__.some((c) => c.command === 'open_exported_files' && c.args.preview === true),
  )
  await page.getByRole('button', { name: '导出 CBZ', exact: true }).click()
  await page.waitForFunction(() =>
    window.__JM_TEST_CALLS__.some(
      (c) => c.command === 'open_exported_files' && c.args.paths.includes('/exports/demo.cbz'),
    ),
  )
  await page.getByRole('checkbox').filter({ hasText: '第 5 话' }).click()
  await page.getByRole('button', { name: '下载所选 (1)', exact: true }).click()
  await page.getByRole('button', { name: '暂停', exact: true }).waitFor()
  console.log(
    'UI smoke stage 1 passed: settings save, portrait/landscape chapter actions, PDF preview/share, CBZ share, download scheduling.',
  )
}
