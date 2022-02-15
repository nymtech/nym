import { createContext, useEffect, useState } from 'react'
import ClientValidator, {
  nativeToPrintable,
} from '@nymproject/nym-validator-client'

export const urls = {
  blockExplorer: 'https://sandbox-blocks.nymtech.net',
}

type TGlobalContext = {
  getBalance: () => void
  requestTokens: ({
    address,
    unymts,
  }: {
    address: string
    unymts: string
    nymts: string
  }) => void
  loadingState: TLoadingState
  balance?: string
  tokenTransfer?: { address: string; amount: string }
  error?: string
}

export const GlobalContext = createContext({} as TGlobalContext)

const { VALIDATOR_ADDRESS, MNEMONIC, TESTNET_URL_1, ACCOUNT_ADDRESS } = process.env

export enum EnumRequestType {
  balance = 'balance',
  tokens = 'tokens',
}

type TLoadingState = {
  isLoading: boolean
  requestType?: EnumRequestType
}

export const GlobalContextProvider: React.FC = ({ children }) => {
  const [validator, setValidator] = useState<ClientValidator>()
  const [loadingState, setLoadingState] = useState<TLoadingState>({
    isLoading: false,
    requestType: undefined,
  })
  const [balance, setBalance] = useState<string>()
  const [error, setError] = useState<string>()
  const [tokenTransfer, setTokenTransfer] =
    useState<{ address: string; amount: string }>()

  const getValidator = async () => {
    const Validator = await ClientValidator.connect(
      VALIDATOR_ADDRESS,
      MNEMONIC,
      [TESTNET_URL_1],
      'nymt'
    )
    setValidator(Validator)
  }

  useEffect(() => {
    if (loadingState.isLoading) {
      setError(undefined)
    }
  }, [loadingState])

  useEffect(() => {
    getValidator()
  }, [])

  useEffect(() => {
    if (validator || tokenTransfer) getBalance()
  }, [validator, tokenTransfer])

  const getBalance = async () => {
    setLoadingState({ isLoading: true, requestType: EnumRequestType.balance })
    try {
      const balance = await validator?.getBalance(ACCOUNT_ADDRESS)
      const tokens = nativeToPrintable(balance?.amount || '')
      setBalance(tokens)
    } catch (e) {
      setError(`An error occured while getting the balance: ${e}`)
    } finally {
      setLoadingState({ isLoading: false, requestType: undefined })
    }
  }

  const requestTokens = async ({
    address,
    unymts,
    nymts,
  }: {
    address: string
    unymts: string
    nymts: string
  }) => {
    setTokenTransfer(undefined)
    setLoadingState({ isLoading: true, requestType: EnumRequestType.tokens })
    try {
      await validator?.send(ACCOUNT_ADDRESS, address, [
        { amount: unymts, denom: 'unymt' },
      ])
      setTokenTransfer({ address, amount: nymts })
    } catch (e) {
      setError(`An error occured during the transfer request: ${e}`)
    } finally {
      setLoadingState({ isLoading: false, requestType: undefined })
    }
  }

  return (
    <GlobalContext.Provider
      value={{
        getBalance,
        requestTokens,
        loadingState,
        balance,
        tokenTransfer,
        error,
      }}
    >
      {children}
    </GlobalContext.Provider>
  )
}
