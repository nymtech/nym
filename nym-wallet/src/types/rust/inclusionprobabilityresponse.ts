import { SelectionChance } from './selectionchance';

export interface InclusionProbabilityResponse {
  in_active: SelectionChance;
  in_reserve: SelectionChance;
}
