import { delegationQueryKeys } from './delegationQueryKeys';

describe('delegationQueryKeys', () => {
  it('builds a stable summary key per client address', () => {
    expect(delegationQueryKeys.summary('nyc1test')).toStrictEqual(['delegation', 'summary', 'nyc1test']);
  });

  it('uses a stable disabled summary key without empty address', () => {
    expect(delegationQueryKeys.summaryDisabled).toStrictEqual(['delegation', 'summary', '__disabled__']);
  });
});
