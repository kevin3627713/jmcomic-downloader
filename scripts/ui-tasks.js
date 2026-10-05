// Continue after ui-smoke.js with the fictional bridge installed.
// prettier-ignore
async (page) => {
  await page.getByRole('button', { name: '暂停', exact: true }).click()
  await page.getByRole('button', { name: '继续', exact: true }).click()
  await page.evaluate(() => window.__JM_TEST_EMIT__('download-task-event', {
    event: 'Update', data: { chapterId: 999104, state: 'Failed', downloadedImgCount: 4, totalImgCount: 24 }
  }))
  await page.getByRole('button', { name: '重试', exact: true }).click()
  await page.getByRole('button', { name: '暂停', exact: true }).waitFor()
  await page.screenshot({ path: 'output/playwright/tasks-390.png' })
  await page.getByRole('button', { name: '取消任务', exact: true }).click()
  await page.getByText('没有进行中的任务', { exact: true }).waitFor()
  const calls = await page.evaluate(() => window.__JM_TEST_CALLS__)
  for (const command of ['pause_download_task', 'resume_download_task', 'cancel_download_task']) {
    if (!calls.some(c => c.command === command && c.args.chapterId === 999104)) throw new Error(command + ' was not called')
  }
  if (calls.filter(c => c.command === 'create_download_task' && c.args.chapterId === 999104).length !== 2) throw new Error('Retry did not recreate the failed task')
  await page.getByRole('button', { name: '书库', exact: true }).click()
  await page.getByRole('button', { name: '更多操作 · 第二本示例漫画', exact: true }).waitFor()
  await page.screenshot({ path: 'output/playwright/library-390.png' })
  console.log('Task pause/resume/retry/cancel passed; ready to verify library bulk export.')
}
