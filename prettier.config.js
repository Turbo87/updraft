/** @type {import("prettier").Config} */
const config = {
  useTabs: false,
  singleQuote: true,
  trailingComma: 'all',
  printWidth: 100,
  plugins: ['@ianvs/prettier-plugin-sort-imports', 'prettier-plugin-svelte'],
  importOrder: [
    '<TYPES>^(node:)',
    '<TYPES>',
    '<TYPES>^\\$lib/',
    '<TYPES>^[.]',
    '',
    '<BUILTIN_MODULES>',
    '',
    '^svelte($|/)',
    '^\\$(app|env)/',
    '<THIRD_PARTY_MODULES>',
    '',
    '^\\$lib/',
    '^[.]',
  ],
  importOrderParserPlugins: ['typescript'],
  importOrderTypeScriptVersion: '6.0.3',
  overrides: [{ files: '*.svelte', options: { parser: 'svelte' } }],
};

export default config;
