import type { FlightBoundaryQuestion } from '#lib/protocol/generated/FlightBoundaryQuestion.js';
import type { Topic } from '#lib/protocol/generated/Topic.js';

export class FlightBoundaryQuestionStore {
  current = $state.raw<FlightBoundaryQuestion | null>(null);

  apply(topic: Topic): void {
    if (topic.topic === 'flightBoundaryQuestion') this.current = topic.value;
  }
}
