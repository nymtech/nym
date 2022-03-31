export type TPages =
  | 'welcome'
  | 'create mnemonic'
  | 'verify mnemonic'
  | 'create password'
  | 'existing account'
  | 'select network'
  | 'legacy create account'
  | 'sign in with mnemonic'
  | 'sign in with password';

export type TMnemonicWord = {
  name: string;
  index: number;
  disabled: boolean;
};
export type TMnemonicWords = TMnemonicWord[];

export type THiddenMnemonicWord = { hidden: boolean } & TMnemonicWord;

export type THiddenMnemonicWords = THiddenMnemonicWord[];
