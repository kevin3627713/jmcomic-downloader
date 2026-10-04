// Run after ui-fixtures.js at the iOS preview. No real comic data is used.
// Reproduce a WKWebView visual viewport that initially excludes safe areas.
// prettier-ignore
async (page) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.addInitScript(() => {
    const viewport = Object.assign(new EventTarget(), {
      height: 763, width: 390, offsetTop: 0, offsetLeft: 0, scale: 1,
    })
    Object.defineProperty(window, 'visualViewport', { configurable: true, value: viewport })
    window.__JM_VIEWPORT__ = (values, fire = true) => {
      Object.assign(viewport, values)
      if (fire) viewport.dispatchEvent(new Event('resize'))
    }
  })
  await page.reload()
  await page.getByRole('button', { name: '搜索漫画', exact: true }).waitFor()
  // Chromium has no iPhone safe area. Supply its normal padding for this fixture.
  await page.addStyleTag({ content: '.mobile-header { padding-top: 47px; } .mobile-nav { padding-bottom: 34px; }' })
  const fullHeight = async (height) => {
    await page.waitForFunction((height) => {
      const shell = document.querySelector('.app-shell').getBoundingClientRect()
      const nav = document.querySelector('.mobile-nav').getBoundingClientRect()
      return Math.abs(shell.top) < 1 && Math.abs(shell.bottom - height) < 1 &&
        Math.abs(nav.bottom - height) < 1 && !document.documentElement.classList.contains('keyboard-open')
    }, height)
  }

  // The initial 81 px discrepancy must not become a permanent bottom gap.
  await fullHeight(844)
  await page.screenshot({ path: 'output/playwright/viewport-cold-start.png' })

  // App resume / pageshow must also ignore a stale smaller visual viewport.
  await page.evaluate(() => {
    window.__JM_VIEWPORT__({ height: 640 })
    window.dispatchEvent(new Event('pageshow'))
  })
  await fullHeight(844)

  // Rotation must remain correct even before visual viewport metrics catch up.
  await page.setViewportSize({ width: 844, height: 390 })
  await page.evaluate(() => window.dispatchEvent(new Event('orientationchange')))
  await fullHeight(390)
  await page.setViewportSize({ width: 390, height: 844 })
  await fullHeight(844)

  // A focused text field and a real viewport reduction should avoid the keyboard.
  const input = page.getByRole('textbox', { name: '搜索关键词 / JM 号', exact: true })
  await input.focus()
  await page.evaluate(() => window.__JM_VIEWPORT__({ height: 500, width: 390, offsetTop: 20, scale: 1 }))
  await page.waitForFunction(() => {
    const shell = document.querySelector('.app-shell').getBoundingClientRect()
    return document.documentElement.classList.contains('keyboard-open') &&
      Math.abs(shell.top - 20) < 1 && Math.abs(shell.height - 500) < 1 &&
      getComputedStyle(document.querySelector('.mobile-nav')).display === 'none'
  })

  // Blur restores the full layout even if WebKit does not update or emit resize.
  await input.blur()
  await fullHeight(844)

  // Pinch zoom and stale orientation metrics are not a software keyboard.
  await page.evaluate(() => window.__JM_VIEWPORT__({ height: 422, width: 195, scale: 2 }))
  await input.focus()
  await fullHeight(844)
  await input.blur()
  await page.evaluate(() => window.__JM_VIEWPORT__({ height: 844, width: 390, offsetTop: 0, scale: 1 }))
  await fullHeight(844)
  await page.screenshot({ path: 'output/playwright/viewport-restored.png' })
  // Browsers without VisualViewport still use the layout viewport safely.
  await page.addInitScript(() => Object.defineProperty(window, 'visualViewport', { configurable: true, value: undefined }))
  await page.reload()
  await page.getByRole('button', { name: '搜索漫画', exact: true }).waitFor()
  await fullHeight(844)
  console.log('Viewport regression passed: stale cold start, app resume, rotation, keyboard resize, stale blur, pinch zoom, and no VisualViewport.')
}
