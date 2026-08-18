import type { StorybookConfig } from '@storybook/sveltekit';

const config: StorybookConfig = {
  stories: ['../src/lib/**/*.stories.svelte', '../src/storybook/**/*.{stories.svelte,mdx}'],
  addons: ['@storybook/addon-svelte-csf', '@storybook/addon-a11y', '@storybook/addon-docs'],
  features: {
    backgrounds: false,
  },
  framework: '@storybook/sveltekit',
  staticDirs: ['../static'],
};
export default config;
