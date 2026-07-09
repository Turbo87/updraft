import type { Change } from './generated/Change';
import type { OwnshipPosition } from './generated/OwnshipPosition';
import type { Snapshot } from './generated/Snapshot';

export class ApplicationState {
  position = $state.raw<OwnshipPosition | null>(null);

  applySnapshot(snapshot: Snapshot) {
    this.position = snapshot.position;
  }

  applyChanges(changes: Change[]) {
    for (let change of changes) {
      this.position = change.flight.position_changed;
    }
  }
}
