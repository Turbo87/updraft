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
  },
};

export default preview;
