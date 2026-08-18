import { DocsContainer } from '@storybook/addon-docs/blocks';
import { jsx, themes } from 'storybook/theming';

import { applyTheme, resolveTheme } from '../src/storybook/theme';

type DocsContainerProps = Parameters<typeof DocsContainer>[0];

type DocsContextWithGlobals = DocsContainerProps['context'] & {
  store: {
    userGlobals: {
      get: () => { theme?: unknown };
    };
  };
};

export function ThemeDocsContainer({ context, children }: DocsContainerProps) {
  let contextWithGlobals = context as DocsContextWithGlobals;
  let theme = resolveTheme(contextWithGlobals.store.userGlobals.get().theme);

  applyTheme(document.documentElement, theme);

  return jsx(DocsContainer, { context, theme: themes[theme] }, children);
}
