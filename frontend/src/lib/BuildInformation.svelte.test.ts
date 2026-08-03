import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import BuildInformation from './BuildInformation.svelte';

const BUILD_TIMESTAMP = '2026-08-03T12:34:56.000Z';
const FULL_COMMIT_SHA = '0123456789abcdef0123456789abcdef01234567';

describe('BuildInformation.svelte', () => {
  it('links an abbreviated commit and formats the build instant', async () => {
    render(BuildInformation, {
      commitSha: FULL_COMMIT_SHA,
      timestamp: BUILD_TIMESTAMP,
      locale: 'en',
    });

    await expect
      .element(page.getByRole('link', { name: '0123456' }))
      .toHaveAttribute('href', `https://github.com/Turbo87/updraft/commit/${FULL_COMMIT_SHA}`);
    await expect.element(page.getByText('Commit')).toBeInTheDocument();
    await expect.element(page.getByText('Built')).toBeInTheDocument();

    let buildTime = page.getByText(
      new Intl.DateTimeFormat('en', {
        dateStyle: 'medium',
        timeStyle: 'medium',
      }).format(new Date(BUILD_TIMESTAMP)),
    );
    await expect.element(buildTime).toHaveAttribute('datetime', BUILD_TIMESTAMP);
  });

  it('shows an unlinked fallback when the commit is unavailable', async () => {
    render(BuildInformation, {
      commitSha: undefined,
      timestamp: BUILD_TIMESTAMP,
      locale: 'en',
    });

    await expect.element(page.getByText('Unknown version')).toBeInTheDocument();
    await expect.element(page.getByRole('link')).not.toBeInTheDocument();
  });
});
