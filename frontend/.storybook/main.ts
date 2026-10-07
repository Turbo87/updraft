import type { StorybookConfig } from '@storybook/sveltekit';

const config: StorybookConfig = {
  stories: ['../src/lib/**/*.stories.svelte', '../src/storybook/**/*.{stories.svelte,mdx}'],
  addons: ['@storybook/addon-svelte-csf', '@storybook/addon-a11y', '@storybook/addon-docs'],
  features: {
    backgrounds: false,
  },
  framework: '@storybook/sveltekit',
  staticDirs: ['../static'],
  viteFinal(config) {
    // Storybook removes the SvelteKit plugin that defines this global for builds, so
    // `$app/paths` throws on import in the static Storybook build. This workaround is
    // temporary until https://github.com/storybookjs/storybook/pull/36610 is released.
    config.define = { ...config.define, __SVELTEKIT_PAYLOAD__: 'undefined' };
    return config;
  },
};
export default config;
