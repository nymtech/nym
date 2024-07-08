export interface DelegateListItem {
  /** Node identity key */
  id: string;
  /** Date of delegation */
  delegationDate: Date;
  /** Delegated amount as a string including the currency, e.g. 1.05 NYM */
  amount: string; // TODO: fix up
  /** Reward amount as a string, e.g. 1.05 NYM on mainnet */
  reward?: string;
  /** A number between 0 and 1 */
  profitMarginPercentage?: number;
  /** A number between 0 and 1 */
  uptimePercentage?: number;
  /** Is pending */
  isPending?: DelegateListItemPending;
}

export interface DelegateListItemPending {
  /** Either the user is delegating or undelegating */
  actionType: 'delegate' | 'undelegate';
  /** Pending transaction */
  blockHeight: number;
}
