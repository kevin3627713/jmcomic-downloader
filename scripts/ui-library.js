// Continue after ui-tasks.js. Use ?books=30 to check pagination.
// prettier-ignore
async (page) => {
  const errors = []
  page.on('pageerror', error => errors.push(String(error)))
  await page.getByRole('button', { name: '管理', exact: true }).click()
  await page.getByRole('checkbox', { name: '选择 漫长旅途中的故事：用于检查手机布局的示例漫画', exact: true }).click()
  await page.getByRole('checkbox', { name: '选择 第二本示例漫画', exact: true }).click()
  const before = await page.evaluate(() => window.__JM_TEST_CALLS__.filter(c => c.command === 'export_pdf').length)
  await page.getByRole('button', { name: '导出所选 PDF', exact: true }).click()
  await page.waitForFunction(before => window.__JM_TEST_CALLS__.filter(c => c.command === 'export_pdf').length === before + 2, before)
  await page.getByRole('button', { name: '书库', exact: true }).click()
  await page.setViewportSize({ width: 320, height: 568 })
  const actionBox = await page.getByRole('button', { name: '导出所选 CBZ', exact: true }).boundingBox()
  if (!actionBox || actionBox.y < 0 || actionBox.y + actionBox.height > 568) throw new Error('Bulk actions are outside the small viewport')
  if (new URL(page.url()).searchParams.get('books') === '30') {
    const pagination = page.locator('.n-pagination').filter({ visible: true })
    await pagination.getByRole('textbox').fill('2')
    await pagination.getByRole('textbox').press('Enter')
    await page.getByRole('button', { name: '示例漫画 21', exact: true }).waitFor()
    const scrollTop = await page.getByRole('button', { name: '示例漫画 21', exact: true }).evaluate(e => e.closest('.selection-container').scrollTop)
    if (scrollTop !== 0) throw new Error('Library pagination did not reset scroll')
  }
  if (errors.length) throw new Error(errors.join('\n'))
  await page.setViewportSize({ width: 390, height: 844 })
  await page.getByRole('button', { name: '账号登录', exact: true }).click()
  await page.getByRole('dialog').waitFor()
  console.log('Library two-book bulk export, narrow actions, and pagination passed; login dialog is ready.')
}
