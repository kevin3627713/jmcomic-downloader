// Run after ui-fixtures.js. All images and files are fictional bridge data.
// prettier-ignore
async (page) => {
  const errors = []
  page.on('pageerror', error => errors.push(String(error)))
  await page.getByRole('button', { name: '书库', exact: true }).click()
  const firstName = '漫长旅途中的故事：用于检查手机布局的示例漫画'
  const first = page.locator('.library-card').filter({ has: page.getByRole('button', { name: firstName, exact: true }) })
  const second = page.locator('.library-card').filter({ has: page.getByRole('button', { name: '第二本示例漫画', exact: true }) })
  const menu = () => page.getByRole('dialog', { name: `${firstName}的文件操作`, exact: true })
  const confirmation = () => page.locator('.n-dialog').filter({ hasText: '删除后无法撤销。' })
  const countDeletes = () => page.evaluate(() => window.__JM_TEST_CALLS__.filter(c => c.command === 'delete_comic_files').length)
  const choose = async (kind) => {
    await first.getByRole('button', { name: `更多操作 · ${firstName}`, exact: true }).click()
    await menu().getByRole('button', { name: kind === 'all' ? '删除整本漫画' : new RegExp(`^删除 ${kind.toUpperCase()}`) }).click()
    await confirmation().waitFor()
  }
  await first.getByRole('button', { name: '查看 PDF', exact: true }).waitFor({ state: 'visible' })
  for (const size of [{ width: 390, height: 844 }, { width: 320, height: 568 }]) {
    await page.setViewportSize(size)
    await page.waitForFunction(() => Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--app-height')) === innerHeight)
    const buttons = await first.locator('.library-card-actions button').evaluateAll(nodes => nodes.map(node => {
      const rect = node.getBoundingClientRect()
      return { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
    }))
    if (buttons.length !== 3 || buttons.some(b => b.height < 44 || b.x < 0 || b.x + b.width > size.width)) throw new Error(`Card actions do not fit ${size.width}px`)
    if (Math.max(...buttons.map(b => b.y)) - Math.min(...buttons.map(b => b.y)) > 1) throw new Error('Primary actions wrap to multiple rows')
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
    if (overflow) throw new Error(`Horizontal overflow at ${size.width}px`)
    await page.screenshot({ path: `output/playwright/library-${size.width}.png` })
  }
  await page.setViewportSize({ width: 844, height: 390 })
  await first.getByRole('button', { name: `更多操作 · ${firstName}`, exact: true }).click()
  await menu().getByRole('button', { name: '删除整本漫画', exact: true }).scrollIntoViewIfNeeded()
  const lastItem = await menu().getByRole('button', { name: '删除整本漫画', exact: true }).boundingBox()
  if (!lastItem || lastItem.y < 0 || lastItem.y + lastItem.height > 390) throw new Error('Landscape menu cannot reach the last action')
  await page.screenshot({ path: 'output/playwright/library-menu-landscape.png' })
  await page.getByRole('button', { name: '关闭文件操作', exact: true }).click()
  await page.setViewportSize({ width: 390, height: 844 })
  await first.getByRole('button', { name: `更多操作 · ${firstName}`, exact: true }).click()
  await menu().waitFor()
  await page.waitForTimeout(250)
  await page.screenshot({ path: 'output/playwright/library-menu.png' })
  await menu().getByRole('button', { name: /^删除 PDF/ }).click()
  await confirmation().waitFor()
  await page.waitForTimeout(250)
  await page.screenshot({ path: 'output/playwright/library-delete-confirm.png' })
  const beforeCancel = await countDeletes()
  await confirmation().getByRole('button', { name: '取消', exact: true }).click()
  await confirmation().waitFor({ state: 'hidden' })
  if (await countDeletes() !== beforeCancel) throw new Error('Cancel invoked deletion')
  await choose('pdf')
  await confirmation().getByRole('button', { name: '确认删除', exact: true }).click()
  await first.getByRole('button', { name: '导出 PDF', exact: true }).waitFor()
  if (!await first.getByRole('button', { name: '分享 CBZ', exact: true }).isVisible()) throw new Error('Deleting PDF removed CBZ/source')
  await choose('cbz')
  await confirmation().getByRole('button', { name: '确认删除', exact: true }).click()
  await first.getByRole('button', { name: '导出 CBZ', exact: true }).waitFor()
  await first.getByText('尚未导出', { exact: true }).waitFor()
  await first.getByRole('button', { name: '导出 PDF', exact: true }).click()
  await page.getByRole('button', { name: '书库', exact: true }).click()
  await first.getByRole('button', { name: '查看 PDF', exact: true }).waitFor()
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke
    window.__JM_FAIL_DELETE__ = true
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === 'delete_comic_files' && window.__JM_FAIL_DELETE__) {
        window.__JM_FAIL_DELETE__ = false
        throw { err_title: '无法删除', err_message: '此漫画正在下载，请完成或取消任务后再删除' }
      }
      return original(command, args)
    }
  })
  await choose('pdf')
  await confirmation().getByRole('button', { name: '确认删除', exact: true }).click()
  await page.getByText('此漫画正在下载，请完成或取消任务后再删除', { exact: true }).waitFor()
  await confirmation().getByRole('button', { name: '取消', exact: true }).waitFor()
  await first.getByRole('button', { name: '查看 PDF', exact: true }).waitFor()
  if (!await confirmation().isVisible() || !await first.getByRole('button', { name: '查看 PDF', exact: true }).isVisible()) throw new Error('Failed deletion dismissed confirmation or changed files')
  await confirmation().getByRole('button', { name: '取消', exact: true }).click()
  await page.getByRole('button', { name: '管理', exact: true }).click()
  await first.getByRole('checkbox', { name: `选择 ${firstName}`, exact: true }).check()
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke
    window.__JM_PENDING_DELETE_CALLS__ = 0
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === 'delete_comic_files') {
        window.__JM_PENDING_DELETE_CALLS__++
        await new Promise(resolve => { window.__JM_RELEASE_DELETE__ = resolve })
      }
      return original(command, args)
    }
  })
  await choose('all')
  await confirmation().getByRole('button', { name: '确认删除', exact: true }).click()
  await page.waitForFunction(() => window.__JM_PENDING_DELETE_CALLS__ === 1)
  if (!await confirmation().getByRole('button', { name: '取消', exact: true }).isDisabled()) throw new Error('In-flight deletion allows dismissal')
  await confirmation().getByRole('button', { name: /确认删除/ }).click({ force: true })
  if (await page.evaluate(() => window.__JM_PENDING_DELETE_CALLS__) !== 1) throw new Error('Duplicate deletion submitted')
  await page.evaluate(() => window.__JM_RELEASE_DELETE__())
  await first.waitFor({ state: 'hidden' })
  await second.waitFor()
  if (await page.getByRole('button', { name: '导出所选 PDF', exact: true }).count()) throw new Error('Removed comic remains selected for bulk export')
  const kinds = await page.evaluate(() => window.__JM_TEST_CALLS__.filter(c => c.command === 'delete_comic_files').map(c => c.args.kind))
  if (JSON.stringify(kinds) !== JSON.stringify(['pdf', 'cbz', 'all'])) throw new Error(`Incorrect delete requests: ${kinds}`)
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke
    window.__TAURI_INTERNALS__.invoke = (command, args) => {
      const pending = original(command, args)
      if (command === 'delete_comic_files') queueMicrotask(() => window.__JM_RELEASE_DELETE__())
      return pending
    }
  })
  await second.getByRole('button', { name: '更多操作 · 第二本示例漫画', exact: true }).click()
  await page.getByRole('dialog', { name: '第二本示例漫画的文件操作', exact: true }).getByRole('button', { name: '删除整本漫画', exact: true }).click()
  await confirmation().getByRole('button', { name: '确认删除', exact: true }).click()
  await page.getByText('书库还没有漫画，下载完成后会显示在这里', { exact: true }).waitFor()
  if (errors.length) throw new Error(errors.join('\n'))
  console.log('PASS: compact 320/390px cards; scrollable landscape menu; cancel; independent PDF/CBZ deletion; re-export; active-task rejection; duplicate guard; selection cleanup; whole deletion and empty library.')
  return { status: 'passed', deletedKinds: [...kinds, 'all'], pageErrors: errors.length, sizes: ['320x568', '390x844', '844x390'] }
}
