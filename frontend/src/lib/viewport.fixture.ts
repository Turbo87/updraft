import { page } from 'vitest/browser';

export async function withViewport(run: (root: HTMLElement) => Promise<void>): Promise<void> {
  let width = window.innerWidth;
  let height = window.innerHeight;
  let root = document.documentElement;
  let style = root.getAttribute('style');
  try {
    await run(root);
  } finally {
    if (style === null) root.removeAttribute('style');
    else root.setAttribute('style', style);
    await page.viewport(width, height);
  }
}
