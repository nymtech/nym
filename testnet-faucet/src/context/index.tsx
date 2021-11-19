import { createContext, useEffect, useState } from 'react'
import ClientValidator, { Coin } from '@nymproject/nym-validator-client'

type TGlobalContext = {
  getBalance: () => void
  requestTokens: ({
    address,
    amount,
  }: {
    address: string
    amount: string
  }) => void
  loadingState: TLoadingState
  balance?: string
  tokenTransfer?: string
  error?: string
}

export const GlobalContext = createContext({} as TGlobalContext)

const {
  VALIDATOR_ADDRESS,
  MNEMONIC,
  TESTNET_URL_1,
  TESTNET_URL_2,
  ACCOUNT_ADDRESS,
} = process.env

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
  const [tokenTransfer, setTokenTransfer] = useState<string>()

  const getValidator = async () => {
    const Validator = await ClientValidator.connect(
      VALIDATOR_ADDRESS,
      MNEMONIC,
      [TESTNET_URL_1, TESTNET_URL_2],
      'punk'
    )
    setValidator(Validator)
  }

  useEffect(() => {
    if (loadingState.isLoading) {
      setError(undefined)
      setBalance(undefined)
      setTokenTransfer(undefined)
    }
  }, [loadingState])

  useEffect(() => {
    getValidator()
  }, [])

  const getBalance = async () => {
    setLoadingState({ isLoading: true, requestType: EnumRequestType.balance })
    try {
      const balance = await validator?.getBalance(ACCOUNT_ADDRESS)
      setBalance(balance?.amount)
    } catch (e) {
      setError(`An error occured while getting the balance: ${e}`)
    } finally {
      setLoadingState({ isLoading: false, requestType: undefined })
    }
  }

  const requestTokens = async ({
    address,
    amount,
  }: {
    address: string
    amount: string
  }) => {
    setLoadingState({ isLoading: true, requestType: EnumRequestType.tokens })
    try {
      const res = await validator?.send(VALIDATOR_ADDRESS, address, [
        { amount, denom: 'upunk' },
      ])
      console.log(res)
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
