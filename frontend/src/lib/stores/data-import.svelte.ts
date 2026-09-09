import type { SelectedDataFile, UpdraftClient } from '$lib/client';
import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
import type { WaypointStatus } from '$lib/protocol/generated/WaypointStatus';

import { tick } from 'svelte';

import { m } from '$lib/paraglide/messages.js';

type ImportContext = {
  importer: Pick<UpdraftClient, 'selectDataFile' | 'importDataFile' | 'discardDataFile'>;
  airspace: AirspaceStatus;
  waypoints: WaypointStatus;
};

export class DataImport {
  pending = $state(false);
  error = $state('');
  selection = $state<SelectedDataFile>();
  scrollTarget = $state<SelectedDataFile & { generation: number }>();
  #disposed = false;
  #opener: HTMLButtonElement | undefined;

  constructor(private context: () => ImportContext) {}

  destroy() {
    this.#disposed = true;
    if (this.selection) void this.#discardSelection(this.selection);
  }

  async #finishImport() {
    this.pending = false;
    await tick();
    if (this.#disposed || this.selection) return;
    let target = this.#opener?.checkVisibility()
      ? this.#opener
      : this.#opener?.closest('.screen-scaffold')?.querySelector('main');
    target?.focus({ preventScroll: true });
  }

  async #discardSelection(selection: SelectedDataFile) {
    try {
      await this.context().importer.discardDataFile(selection.selectionId);
    } catch (error) {
      if (this.#disposed) console.warn('Could not discard selected data file.', error);
      else this.error = m.data_cancel_failed();
    }
  }

  async cancelImport() {
    let selection = this.selection;
    this.selection = undefined;
    if (!selection) return;
    this.pending = true;
    await this.#discardSelection(selection);
    await this.#finishImport();
  }

  async importFile(selection: SelectedDataFile) {
    this.selection = undefined;
    this.pending = true;
    let generation = this.context()[selection.dataType].generation;
    try {
      let imported = await this.context().importer.importDataFile(selection.selectionId);
      this.scrollTarget = { ...imported, generation };
    } catch {
      this.error = m.data_import_failed();
    } finally {
      await this.#finishImport();
    }
  }

  async selectFile(opener: HTMLButtonElement) {
    this.#opener = opener;
    this.scrollTarget = undefined;
    this.pending = true;
    this.error = '';
    try {
      let selection = await this.context().importer.selectDataFile();
      if (!selection) return;
      if (this.#disposed) {
        await this.#discardSelection(selection);
        return;
      }
      let sources = this.context()[selection.dataType].sources;
      if (sources.some((source) => source.sourceName === selection.sourceName)) {
        this.selection = selection;
      } else {
        await this.importFile(selection);
      }
    } catch {
      this.error = m.data_select_failed();
    } finally {
      await this.#finishImport();
    }
  }
}
