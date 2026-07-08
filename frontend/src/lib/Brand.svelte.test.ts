import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { localStorageKey } from '$lib/paraglide/runtime.js';
import Brand from './Brand.svelte';

describe('Brand.svelte', () => {
	afterEach(() => {
		localStorage.removeItem(localStorageKey);
	});

	it('renders the app name', async () => {
		render(Brand);

		await expect.element(page.getByRole('heading', { level: 1 })).toHaveTextContent('Updraft');
	});

	it('renders the localized tagline by default', async () => {
		localStorage.setItem(localStorageKey, 'de');
		render(Brand);

		await expect.element(page.getByText('Segelflugrechner')).toBeInTheDocument();
	});

	it('renders a custom tagline', async () => {
		render(Brand, { tagline: 'Test tagline' });

		await expect.element(page.getByText('Test tagline')).toBeInTheDocument();
	});
});
