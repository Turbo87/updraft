import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../../app.css';

import type { FlightBoundaryQuestion } from '#lib/protocol/generated/FlightBoundaryQuestion.js';

import { FakeClient } from '#lib/client/fake.js';
import FlightBoundaryDialog from './FlightBoundaryDialog.svelte';

const HOUR = 60 * 60 * 1000;

function renderQuestion(question: FlightBoundaryQuestion | null, client = new FakeClient()) {
  return render(FlightBoundaryDialog, { question, distanceUnit: 'km', client });
}

describe('FlightBoundaryDialog.svelte', () => {
  it('stays closed without a question', async () => {
    await renderQuestion(null);

    await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  });

  it.each<[FlightBoundaryQuestion, string]>([
    [{ cause: 'positionJump', meters: 120_400 }, 'Position jumped 120 km.'],
    [{ cause: 'fixUtcJump', milliseconds: 5 * HOUR + 1 }, 'GPS time jumped forward 5 hours.'],
    [{ cause: 'fixUtcJump', milliseconds: -3 * 24 * HOUR }, 'GPS time jumped back 3 days.'],
    [{ cause: 'fixUtcJump', milliseconds: -30_001 }, 'GPS time jumped back 30 seconds.'],
  ])('states the cause of %o', async (question, cause) => {
    await renderQuestion(question);

    let dialog = page.getByRole('alertdialog', { name: 'New flight?' });
    await expect.element(dialog).toBeVisible();
    await expect
      .element(dialog)
      .toHaveAccessibleDescription(`${cause} New flight deletes the current flight recording.`);
    await expect.element(page.getByRole('button', { name: 'Continue flight' })).toHaveFocus();
  });

  it('answers New flight and stays open until the core closes the question', async () => {
    let client = new FakeClient();
    let question: FlightBoundaryQuestion = { cause: 'positionJump', meters: 120_400 };
    let view = await renderQuestion(question, client);

    await page.getByRole('button', { name: 'New flight' }).click();

    expect(client.flightCommands).toEqual(['startNewFlight']);
    await expect.element(page.getByRole('alertdialog')).toBeInTheDocument();
    await view.rerender({ question: null });
    await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('answers Continue flight and stays open until the core closes the question', async () => {
    let client = new FakeClient();
    let question: FlightBoundaryQuestion = { cause: 'fixUtcJump', milliseconds: -30_001 };
    let view = await renderQuestion(question, client);

    await page.getByRole('button', { name: 'Continue flight' }).click();

    expect(client.flightCommands).toEqual(['continueFlight']);
    await expect.element(page.getByRole('alertdialog')).toBeInTheDocument();
    await view.rerender({ question: null });
    await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('reports an answer that fails', async () => {
    let client = new FakeClient();
    client.continueFlight = () => Promise.reject(new Error('driver stopped'));
    await renderQuestion({ cause: 'fixUtcJump', milliseconds: -30_001 }, client);

    await page.getByRole('button', { name: 'Continue flight' }).click();

    await expect.element(page.getByRole('alert')).toHaveTextContent('Could not save the answer.');
    await expect.element(page.getByRole('alertdialog')).toBeInTheDocument();
  });
});
