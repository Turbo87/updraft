import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Brand from './Brand.svelte';

describe('Brand.svelte', () => {
	it('renders the app name', async () => {
		render(Brand);

		await expect.element(page.getByRole('heading', { level: 1 })).toHaveTextContent('Updraft');
	});

	it('renders a custom tagline', async () => {
		render(Brand, { tagline: 'Test tagline' });

		await expect.element(page.getByText('Test tagline')).toBeInTheDocument();
	});
});
