import { useEffect, useState } from 'react'
import { Coin } from '@cosmjs/amino'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { BondingInformation } from './NodeBond'
import {
  formatDataForSubmission,
  isValidHostname,
  validateAmount,
  validateKey,
  validateLocation,
  validateVersion,
} from './utils'
import { checkAllocationSize, validateRawPort } from '../../common/helpers'
import { NodeType } from '../../common/node'
import { useGetBalance } from '../../hooks/useGetBalance'

const DEFAULT_PORTS = {
  MIX_PORT: 1789,
  VERLOC_PORT: 1790,
  HTTP_API_PORT: 8000,
  CLIENTS_PORT: 9000,
}

const initialPorts = {
  mixPort: { value: DEFAULT_PORTS.MIX_PORT.toString(), isValid: true },
  verlocPort: {
    value: DEFAULT_PORTS.VERLOC_PORT.toString(),
    isValid: true,
  },
  clientsPort: {
    value: DEFAULT_PORTS.CLIENTS_PORT.toString(),
    isValid: true,
  },
  httpApiPort: {
    value: DEFAULT_PORTS.HTTP_API_PORT.toString(),
    isValid: true,
  },
}

const initialData = {
  amount: { value: '', isValid: undefined },
  identityKey: { value: '', isValid: undefined },
  sphinxKey: { value: '', isValid: undefined },
  host: { value: '', isValid: undefined },
  version: { value: '', isValid: undefined },
  location: { value: '', isValid: true },
  ...initialPorts,
}

type TDataField = { value: string; isValid?: boolean }

type TData = {
  amount: TDataField
  identityKey: TDataField
  sphinxKey: TDataField
  host: TDataField
  version: TDataField
  location: TDataField
  mixPort: TDataField
  verlocPort: TDataField
  clientsPort: TDataField
  httpApiPort: TDataField
}

export const useBondForm = ({
  type,
  minimumBond,
}: {
  type: NodeType
  minimumBond: Coin
}) => {
  const [formData, setFormData] = useState(initialData)
  const [isValidForm, setIsValidForm] = useState(false)
  const [allocationWarning, setAllocationWarning] = useState({
    error: false,
    message: undefined,
  })
  const { getBalance, accountBalance } = useGetBalance()

  useEffect(() => {
    getBalance()
  }, [getBalance])

  useEffect(() => {
    if (type === NodeType.Mixnode)
      setFormData((data) => ({ ...data, location: initialData.location }))
  }, [type])

  useEffect(() => {
    const keys = Object.keys(formData)
    const isValid = keys
      .map((key) => formData[key].isValid)
      .every((value) => value === true)
    setIsValidForm(isValid)
  }, [formData])

  const handleSubmit = (cb: (data: BondingInformation) => void) => {
    const keys = Object.keys(formData)
    const values = keys.reduce((a, c: keyof TData) => {
      return {
        ...a,
        [c]: formData[c].value,
      }
    }, {})
    const formatted = formatDataForSubmission(values, type)
    cb(formatted)
  }

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let isValid = false

    try {
      if (
        !isNaN(+e.target.value) &&
        validateAmount(e.target.value, minimumBond.amount)
      )
        isValid = true

      const allocationCheck = checkAllocationSize(
        +printableBalanceToNative(e.target.value),
        +accountBalance.amount,
        'bond'
      )
      setAllocationWarning(allocationCheck)
    } catch (e) {
      console.log(e)
    }

    setFormData((data) => ({
      ...data,
      amount: { value: e.target.value, isValid },
    }))
  }

  const handleIdentityKeyChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const isValid = validateKey(e.target.value)

    setFormData((data) => ({
      ...data,
      identityKey: { value: e.target.value, isValid },
    }))
  }

  const handleShinxKeyChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const isValid = validateKey(e.target.value)

    setFormData((data) => ({
      ...data,
      sphinxKey: { value: e.target.value, isValid },
    }))
  }

  const handleHostChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const isValid = isValidHostname(e.target.value)
    setFormData((data) => ({
      ...data,
      host: { value: e.target.value, isValid },
    }))
  }

  const handleVersionChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const isValid = validateVersion(e.target.value)
    setFormData((data) => ({
      ...data,
      version: { value: e.target.value, isValid },
    }))
  }

  const handleLocationChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const isValid = validateLocation(e.target.value)
    setFormData((data) => ({
      ...data,
      location: { value: e.target.value, isValid },
    }))
  }

  const handlePortChange = (
    port: 'mixPort' | 'verlocPort' | 'clientsPort' | 'httpApiPort',
    value: string
  ) => {
    const isValid = validateRawPort(value)
    setFormData((data) => ({
      ...data,
      [port]: { value, isValid },
    }))
  }

  const initialisePorts = () =>
    setFormData((data) => ({ ...data, ...initialPorts }))

  return {
    formData,
    isValidForm,
    allocationWarning,
    handleAmountChange,
    handleIdentityKeyChange,
    handleShinxKeyChange,
    handleHostChange,
    handleVersionChange,
    handleLocationChange,
    handlePortChange,
    handleSubmit,
    initialisePorts,
  }
}
