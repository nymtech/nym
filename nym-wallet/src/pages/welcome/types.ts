import { type } from '@tauri-apps/api/os'

export type TMnemonicWord = {
  name: string
  index: number
}
export type TMnemonicWords = TMnemonicWord[]

export type THiddenMnemonicWord = { hidden: boolean } & TMnemonicWord

export type THiddenMnemonicWords = THiddenMnemonicWord[]
