import { resolveDeleteModalRecovery } from './deleteAccountModalRecovery';

describe('resolveDeleteModalRecovery', () => {
  it('returns to the warning step after a failed removal so the user keeps cancel and back actions', () => {
    expect(resolveDeleteModalRecovery('removal_failed')).toStrictEqual({ action: 'return_to_warning' });
  });

  it('exits the delete flow when password confirmation is cancelled', () => {
    expect(resolveDeleteModalRecovery('confirm_password_cancel')).toStrictEqual({ action: 'exit_flow' });
  });

  it('exits the delete flow after a successful removal', () => {
    expect(resolveDeleteModalRecovery('removal_succeeded')).toStrictEqual({ action: 'exit_flow' });
  });
});
