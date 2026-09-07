import { createRawSnippet, mount, unmount } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import Card from './Card.svelte';
import ResponsiveCard from './ResponsiveCard.svelte';
import ScreenScaffold from './ScreenScaffold.svelte';

const children = createRawSnippet(() => ({ render: () => '<p>Card content</p>' }));

describe('Card', () => {
  it('has no inner padding and shows errors without changing its geometry', async () => {
    let view = await render(Card, { children });
    let card = page.getByText('Card content').element().parentElement!;
    let before = card.getBoundingClientRect().toJSON();
    expect(getComputedStyle(card).padding).toBe('0px');
    expect(getComputedStyle(card).borderRadius).toBe('12px');
    await view.rerender({ children, error: true });
    expect(card.getBoundingClientRect().toJSON()).toEqual(before);
    expect(getComputedStyle(card).outlineWidth).toBe('2px');
  });

  it('reaches screen edges while preserving row safe areas', async () => {
    let oldWidth = window.innerWidth;
    let oldHeight = window.innerHeight;
    let root = document.documentElement;
    let previousStyle = root.getAttribute('style');
    try {
      await page.viewport(413, 600);
      root.style.setProperty('--safe-area-left', '24px');
      root.style.setProperty('--safe-area-right', '12px');
      let row = createRawSnippet(() => ({
        render: () =>
          '<div style="padding-inline:calc(20px + var(--card-safe-area-start)) calc(16px + var(--card-safe-area-end))">Safe row</div>',
      }));
      let content = createRawSnippet(() => ({
        render: () => '<div></div>',
        setup(element) {
          let component = mount(ResponsiveCard, { target: element, props: { children: row } });
          return () => {
            void unmount(component);
          };
        },
      }));
      await render(ScreenScaffold, {
        title: 'Cards',
        backLabel: 'Back',
        backHref: '/settings',
        children: content,
      });
      let element = page.getByText('Safe row').element();
      let bounds = element.parentElement!.getBoundingClientRect();
      expect([bounds.left, bounds.right]).toEqual([0, 413]);
      expect([
        getComputedStyle(element).paddingLeft,
        getComputedStyle(element).paddingRight,
      ]).toEqual(['44px', '28px']);
    } finally {
      if (previousStyle === null) root.removeAttribute('style');
      else root.setAttribute('style', previousStyle);
      await page.viewport(oldWidth, oldHeight);
    }
  });

  it.each([413, 543, 544, 545, 915])(
    'uses responsive edges at viewport width %s',
    async (width) => {
      let oldWidth = window.innerWidth;
      let oldHeight = window.innerHeight;
      try {
        await page.viewport(width, 600);
        render(ResponsiveCard, {
          children,
          style: '--content-inset-start: 27px; --content-inset-end: 31px',
        });
        let card = page.getByText('Card content').element().parentElement!;
        let style = getComputedStyle(card);
        expect([style.marginLeft, style.marginRight, style.borderRadius]).toEqual(
          width <= 544 ? ['-27px', '-31px', '0px'] : ['0px', '0px', '12px'],
        );
      } finally {
        await page.viewport(oldWidth, oldHeight);
      }
    },
  );
});
