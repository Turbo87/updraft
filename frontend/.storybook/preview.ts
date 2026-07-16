import type { Preview } from '@storybook/sveltekit';

import '../src/app.css';
import 'virtual:uno.css';

const preview: Preview = {
  tags: ['autodocs'],
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
};

export default preview;
