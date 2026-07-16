import type { UserConfig } from 'vite';

import { transform } from 'lightningcss';
import { expect, test } from 'vitest';

import viteConfig from '../vite.config';

const fixture = `
:root {
  color-scheme: light dark;
  --color-amber-500: oklch(76.9% 0.188 70.08);
  --color-amber-950: oklch(27.9% 0.077 45.635);
  --color-warning-surface: light-dark(var(--color-amber-500), var(--color-amber-950));
}

:root[data-theme='light'] {
  color-scheme: light;
}

:root[data-theme='dark'] {
  color-scheme: dark;
}

.fixture {
  & .nested {
    color: var(--color-warning-surface);
  }
}
`;

test('compiles compatibility-sensitive CSS for supported webviews', () => {
  let config = viteConfig as UserConfig;

  expect(config.css?.transformer).toBe('lightningcss');
  expect(config.build?.cssMinify).toBe('lightningcss');

  let lightningcss = config.css?.lightningcss;
  expect(lightningcss).toBeDefined();

  let result = transform({
    ...lightningcss,
    filename: 'compatibility.css',
    code: Buffer.from(fixture),
  });

  expect(result.code.toString()).toMatchInlineSnapshot(`
    ":root {
      --lightningcss-light: initial;
      --lightningcss-dark: ;
      color-scheme: light dark;
      --color-amber-500: #f99c00;
      --color-amber-950: #461901;
      --color-warning-surface: var(--lightningcss-light, var(--color-amber-500)) var(--lightningcss-dark, var(--color-amber-950));
    }

    @media (prefers-color-scheme: dark) {
      :root {
        --lightningcss-light: ;
        --lightningcss-dark: initial;
      }
    }

    @supports (color: color(display-p3 0 0 0)) {
      :root {
        --color-amber-500: color(display-p3 .93994 .620584 .0585367);
        --color-amber-950: color(display-p3 .252662 .109091 .026881);
      }
    }

    @supports (color: lab(0% 0 0)) {
      :root {
        --color-amber-500: lab(72.7183% 31.8672 97.9407);
        --color-amber-950: lab(15.8111% 20.9107 23.3752);
      }
    }

    :root[data-theme="light"] {
      --lightningcss-light: initial;
      --lightningcss-dark: ;
      color-scheme: light;
    }

    :root[data-theme="dark"] {
      --lightningcss-light: ;
      --lightningcss-dark: initial;
      color-scheme: dark;
    }

    .fixture .nested {
      color: var(--color-warning-surface);
    }
    "
  `);
});
