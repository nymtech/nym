import type { SelectionChance } from './SelectionChance';

export interface InclusionProbabilityResponse {
  in_active: SelectionChance;
  in_reserve: SelectionChance;
}
