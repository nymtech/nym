export type TPages = 'welcome' | 'create account' | 'verify mnemonic' | 'create password'

export type TMnemonicWord = {
  name: string
  index: number
  disabled: boolean
}
export type TMnemonicWords = TMnemonicWord[]

export type THiddenMnemonicWord = { hidden: boolean } & TMnemonicWord

export type THiddenMnemonicWords = THiddenMnemonicWord[]
