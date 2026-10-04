// Continue after ui-library.js with the login dialog open.
// prettier-ignore
async (page) => {
  await page.getByRole('textbox', { name: '用户名', exact: true }).fill('fixture-user')
  await page.getByRole('textbox', { name: '密码', exact: true }).fill('fixture-password')
  await page.getByRole('button', { name: '登录', exact: true }).click()
  await page.getByRole('dialog').waitFor({ state: 'hidden' })
  await page.getByRole('button', { name: '收藏', exact: true }).click()
  await page.getByRole('button', { name: '收藏不对点我', exact: true }).waitFor()
  if (await page.getByRole('button', { name: '更新漫画', exact: true }).isVisible()) throw new Error('Obsolete update control should not use mobile space')
  await page.getByRole('button', { name: '发现', exact: true }).click()
  await page.getByRole('button', { name: '每周必看', exact: true }).click()
  await page.getByText('本周推荐', { exact: true }).waitFor()
  await page.getByRole('button', { name: '更多选项', exact: true }).click()
  await page.getByText('运行日志', { exact: true }).click()
  await page.getByRole('dialog').waitFor()
  await page.evaluate(() => window.__JM_TEST_EMIT__('log-event', {
    timestamp: '2026-10-04 20:00:00', level: 'INFO', fields: { message: '手机界面验证日志' },
    target: 'jm::ui', filename: 'fixture', line_number: 1
  }))
  await page.getByText(/手机界面验证日志/).waitFor()
  const dialog = await page.getByRole('dialog').boundingBox()
  if (!dialog || dialog.x < 0 || dialog.y < 0 || dialog.y + dialog.height > 844) throw new Error('Log dialog is outside the viewport')
  await page.screenshot({ path: 'output/playwright/logs-390.png' })
  await page.getByRole('button', { name: 'close', exact: true }).click()
  console.log('Fictional login, favorites, weekly list and log dialog passed.')
}
