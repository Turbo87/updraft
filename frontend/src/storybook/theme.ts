export type StorybookTheme = 'light' | 'dark';

type ThemeRoot = {
  dataset: {
    theme?: string;
  };
};

export function applyTheme(root: ThemeRoot, theme: StorybookTheme): void {
  root.dataset.theme = theme;
}

export function resolveTheme(theme: unknown): StorybookTheme {
  return theme === 'dark' ? 'dark' : 'light';
}
