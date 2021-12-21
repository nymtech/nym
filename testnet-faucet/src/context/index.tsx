import { createContext, useEffect, useState } from 'react'
import ClientValidator, {
  nativeToPrintable,
} from '@nymproject/nym-validator-client'
import { config } from '../config'

export const { MAJOR_CURRENCY, MINOR_CURRENCY, VALIDATOR_ADDRESS, ACCOUNT_ADDRESS, NETWORK , TESTNET_URL, MNEMONIC } = config

export const urls = {
  blockExplorer: `https://${NETWORK}-blocks.nymtech.net`,
}

type TGlobalContext = {
  getBalance: () => void
  requestTokens: ({
    address,
    minorcurrency,
  }: {
    address: string
    minorcurrency: string
    majorcurrency: string
  }) => void
  loadingState: TLoadingState
  balance?: string
  tokenTransfer?: { address: string; amount: string }
  error?: string
}

export const GlobalContext = createContext({} as TGlobalContext)

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
      [TESTNET_URL],
      MAJOR_CURRENCY
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
      const punks = nativeToPrintable(balance?.amount || '')
      setBalance(punks)
    } catch (e) {
      setError(`An error occured while getting the balance: ${e}`)
    } finally {
      setLoadingState({ isLoading: false, requestType: undefined })
    }
  }

  const requestTokens = async ({
    address,
    minorcurrency,
    majorcurrency,
  }: {
    address: string
    minorcurrency: string
    majorcurrency: string
  }) => {
    setTokenTransfer(undefined)
    setLoadingState({ isLoading: true, requestType: EnumRequestType.tokens })
    try {
      await validator?.send(ACCOUNT_ADDRESS, address, [
        { amount: minorcurrency, denom: MINOR_CURRENCY},
      ])
      setTokenTransfer({ address, amount: majorcurrency })
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
