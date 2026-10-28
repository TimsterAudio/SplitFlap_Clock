// Makes the guide's PDFs, one with dark pages and one with light, from the
// built HTML (see build.py): node docs/guide/pdf.js
const { chromium } = require('playwright');
const path = require('path');
(async () => {
  const root = path.resolve(__dirname, '..', '..');
  const html = path.join(root, 'Split-Flap Clock User Guide.html');
  const dark = path.join(root, 'Split-Flap Clock User Guide (dark).pdf');
  const light = path.join(root, 'Split-Flap Clock User Guide (light).pdf');
  const browser = await chromium.launch();
  for (const [scheme, out] of [['dark', dark], ['light', light]]) {
    const page = await browser.newPage({ colorScheme: scheme });
    await page.goto('file://' + html);
    // The screen's styles, in pages.
    await page.emulateMedia({ media: 'screen', colorScheme: scheme });
    await page.evaluate(() => document.documentElement.classList.add('paged'));
    await page.evaluate(() => document.fonts.ready);
    await page.pdf({ path: out, format: 'A4', printBackground: true, preferCSSPageSize: true, outline: true, tagged: true });
    await page.close();
  }
  await browser.close();
})();
