import ClientValidator from '@nymproject/nym-validator-client'
import { useEffect, useState } from 'react'

const { VALIDATOR_ADDRESS, MNEMONIC, TESTNET_URL_1, ACCOUNT_ADDRESS } =
  process.env

export const useValidatorClient = () => {
  const [validator, setValidator] = useState<ClientValidator>()

  const getValidator = async () => {
    const Validator = await ClientValidator.connect(
      VALIDATOR_ADDRESS,
      MNEMONIC,
      [TESTNET_URL_1],
      ACCOUNT_ADDRESS
    )
    setValidator(Validator)
  }

  useEffect(() => {
    getValidator()
  }, [])

  const getBalance = async () => await validator?.getBalance(ACCOUNT_ADDRESS)

  return { getBalance }
}
