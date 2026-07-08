/** @type {import("prettier").Config} */
const config = {
  useTabs: false,
  singleQuote: true,
  trailingComma: 'all',
  printWidth: 100,
  plugins: ['prettier-plugin-svelte'],
  overrides: [{ files: '*.svelte', options: { parser: 'svelte' } }],
};

export default config;
