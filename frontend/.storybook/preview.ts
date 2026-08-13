import type { Preview } from '@storybook/sveltekit';

import '../src/app.css';
import 'virtual:uno.css';

import { applyTheme, resolveTheme } from '../src/storybook/theme';
import { ThemeDocsContainer } from './ThemeDocsContainer';

const preview: Preview = {
  decorators: [
    (Story, context) => {
      applyTheme(document.documentElement, resolveTheme(context.globals.theme));
      return Story();
    },
  ],
  globalTypes: {
    theme: {
      description: 'Theme for all stories',
      toolbar: {
        title: 'Theme',
        icon: 'paintbrush',
        items: [
          { value: 'light', title: 'Light', icon: 'sun' },
          { value: 'dark', title: 'Dark', icon: 'moon' },
        ],
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: {
    theme: 'light',
  },
  tags: ['autodocs'],
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    docs: {
      container: ThemeDocsContainer,
    },
    viewport: {
      options: {
        galaxyS23: {
          name: 'Galaxy S23',
          styles: {
            width: '360px',
            height: '780px',
          },
          type: 'mobile',
        },
        ipad: {
          name: 'iPad',
          styles: {
            width: '768px',
            height: '1024px',
          },
          type: 'tablet',
        },
      },
    },
  },
};

export default preview;
