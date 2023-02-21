import { createContext, useEffect, useState } from 'react'
import ClientValidator, {
  nativeToPrintable,
} from '@nymproject/nym-validator-client'
import { handleError } from '../utils'
import { useCookie } from '..//hooks/useCookie'

const { VALIDATOR_ADDRESS, MNEMONIC, TESTNET_URL_1, ACCOUNT_ADDRESS } = process.env;

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
  tokensAreAvailable: boolean
}

type TLoadingState = {
  isLoading: boolean
  requestType?: EnumRequestType
}

export enum EnumRequestType {
  balance = 'balance',
  tokens = 'tokens',
}

export const GlobalContext = createContext({} as TGlobalContext)

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
    getValidator()
  }, [])

  useEffect(() => {
    if (error) console.error(error)
  }, [error])

  const [hasMadePreviousRequest, setHasMadePreviousRequest] = useCookie(
    'hasUsedFaucet',
    false
  )

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

  useEffect(() => {
    if (validator || tokenTransfer) getBalance()
  }, [validator, tokenTransfer])

  const resetGlobalState = () => {
    setError(undefined)
    setTokenTransfer(undefined)
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
    resetGlobalState()
    setLoadingState({ isLoading: true, requestType: EnumRequestType.tokens })

    if (!hasMadePreviousRequest) {
      try {
        await validator?.send(ACCOUNT_ADDRESS, address, [
          { amount: unymts, denom: 'unymt' },
        ])
        setTokenTransfer({ address, amount: nymts })
        setHasMadePreviousRequest(true, 1)
      } catch (e) {
        setError(handleError(e as Error))
      }
    } else {
      setError('Tokens are not currently available')
    }

    setLoadingState({ isLoading: false, requestType: undefined })
  }

  const tokensAreAvailable =
    !hasMadePreviousRequest && Boolean(balance && +balance >= 101)

  return (
    <GlobalContext.Provider
      value={{
        getBalance,
        requestTokens,
        loadingState,
        balance,
        tokenTransfer,
        error,
        tokensAreAvailable,
      }}
    >
      {children}
    </GlobalContext.Provider>
  )
}
