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

export const isValidHostname = (value: string) => {
  const hostnameRegex =
    /((^\s*((([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]))\s*$)|(^\s*((([0-9A-Fa-f]{1,4}:){7}([0-9A-Fa-f]{1,4}|:))|(([0-9A-Fa-f]{1,4}:){6}(:[0-9A-Fa-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){5}(((:[0-9A-Fa-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9A-Fa-f]{1,4}:){4}(((:[0-9A-Fa-f]{1,4}){1,3})|((:[0-9A-Fa-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){3}(((:[0-9A-Fa-f]{1,4}){1,4})|((:[0-9A-Fa-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){2}(((:[0-9A-Fa-f]{1,4}){1,5})|((:[0-9A-Fa-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9A-Fa-f]{1,4}:){1}(((:[0-9A-Fa-f]{1,4}){1,6})|((:[0-9A-Fa-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9A-Fa-f]{1,4}){1,7})|((:[0-9A-Fa-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))(%.+)?\s*$))/g

  return hostnameRegex.test(value)
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
