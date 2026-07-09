import type { Change } from './generated/Change';
import type { Snapshot } from './generated/Snapshot';

interface StateSubscription {
  onSnapshot(snapshot: Snapshot): void;
  onChanges(changes: Change[]): void;
}

export class StateClient {
  subscribe(subscription: StateSubscription) {
    let eventSource = new EventSource('/api/state');

    function handleSnapshot(event: MessageEvent<string>) {
      subscription.onSnapshot(JSON.parse(event.data) as Snapshot);
    }

    function handleChanges(event: MessageEvent<string>) {
      subscription.onChanges(JSON.parse(event.data) as Change[]);
    }

    eventSource.addEventListener('snapshot', handleSnapshot);
    eventSource.addEventListener('changes', handleChanges);

    return function unsubscribe() {
      eventSource.close();
    };
  }
}
