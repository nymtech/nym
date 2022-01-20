export type TPages =
  | 'welcome'
  | 'create account'
  | 'verify mnemonic part 1'
  | 'verify mnemonic part 2'
  | 'create password'

export type TMnemonicWord = {
  name: string
  index: number
}
export type TMnemonicWords = TMnemonicWord[]

export type THiddenMnemonicWord = { hidden: boolean } & TMnemonicWord

export type THiddenMnemonicWords = THiddenMnemonicWord[]
