import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { Gateway, MixNode } from '@nymproject/nym-validator-client/dist/types'
import bs58 from 'bs58'
import semver from 'semver'
import { basicRawCoinValueValidation } from '../../common/helpers'
import { NodeType } from '../../common/node'
import { BondingInformation } from './NodeBond'

export const validateAmount = (rawValue: string, minimum: string): boolean => {
  // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
  if (!basicRawCoinValueValidation(rawValue)) {
    return false
  }

  // this conversion seems really iffy but I'm not sure how to better approach it
  let nativeValueString = printableBalanceToNative(rawValue)
  let nativeValue = parseInt(nativeValueString)
  console.log(nativeValue, minimum)
  return nativeValue >= parseInt(minimum)
}

export const validateKey = (key: string): boolean => {
  // it must be a valid base58 key
  try {
    const bytes = bs58.decode(key)
    // of length 32
    return bytes.length === 32
  } catch {
    return false
  }
}

export const isValidHostname = (value) => {
  if (typeof value !== 'string') return false

  const validHostnameChars = /^[a-zA-Z0-9-.]{1,253}\.?$/g
  if (!validHostnameChars.test(value)) {
    return false
  }

  if (value.endsWith('.')) {
    value = value.slice(0, value.length - 1)
  }

  if (value.length > 253) {
    return false
  }

  const labels = value.split('.')

  const isValid = labels.every(function (label) {
    const validLabelChars = /^([a-zA-Z0-9-]+)$/g

    const validLabel =
      validLabelChars.test(label) &&
      label.length < 64 &&
      !label.startsWith('-') &&
      !label.endsWith('-')

    return validLabel
  })

  return isValid
}

// check if its a valid semver
export const validateVersion = (version: string): boolean =>
  semver.valid(version) && semver.minor(version) >= 11

export const validateLocation = (location: string): boolean => {
  // right now only perform the stupid check of whether the user copy-pasted the tooltip... (with or without brackets)
  return !location.trim().includes('physical location of your node')
}

export const formatDataForSubmission = <T>(
  {
    amount,
    host,
    identityKey,
    sphinxKey,
    clientsPort,
    httpApiPort,
    mixPort,
    verlocPort,
    location,
    version,
  }: { [key: string]: string | undefined },
  nodeType: NodeType
) => {
  let data = {
    amount,
    nodeDetails: {
      identity_key: identityKey,
      sphinx_key: sphinxKey,
      host,
      version,
      mix_port: parseInt(mixPort),
    },
  }

  if (nodeType === NodeType.Mixnode) {
    data = {
      ...data,
      nodeDetails: {
        ...data.nodeDetails,
        verloc_port: parseInt(verlocPort),
        http_api_port: parseInt(httpApiPort),
      } as MixNode,
    }
  } else {
    data = {
      ...data,
      nodeDetails: {
        ...data.nodeDetails,
        location,
        clients_port: parseInt(clientsPort),
      } as Gateway,
    }
  }
  return data as BondingInformation
}
