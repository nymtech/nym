import { validateRawPort } from '../../common/helpers'
import {
  isValidHostname,
  validateAmount,
  validateKey,
  validateVersion,
} from './utils'

test('correctly validates ipv4', () => {
  expect(isValidHostname('0.0.0.0')).toBe(true)
  expect(isValidHostname('192.168.1.1')).toBe(true)
  expect(isValidHostname('255.255.255.255')).toBe(true)

  expect(isValidHostname('10.168.0001.100')).toBe(false)
  expect(isValidHostname('192.168.224.0 1')).toBe(false)
  expect(isValidHostname('256.0.0.0')).toBe(false)
})

test('correctly validates ipv6', () => {
  expect(isValidHostname('2001:0db8:0000:85a3:0000:0000:ac1f:8001')).toBe(true)
  expect(isValidHostname('0000:0000:0000:0000:0000:0000:0000:0000')).toBe(true)
  expect(isValidHostname('ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff')).toBe(true)

  expect(isValidHostname('2001:0000:1234: 0000:0000:C1C0:ABCD:0876')).toBe(
    false
  )
  expect(isValidHostname('::1111:2222:3333:4444:5555:6666::')).toBe(false)
  expect(isValidHostname('3ffe:b00::1::a')).toBe(false)
})

test('correctly validates hostnames', () => {
  expect(isValidHostname('nymtech.net')).toBe(true)
  expect(isValidHostname('foo.com')).toBe(true)

  expect(isValidHostname('nymtech.?')).toBe(false)
})

test('correctly validates an amount', () => {
  expect(validateAmount('100.0625', '100000000')).toBe(true)

  expect(validateAmount('100.12343445', '100000000')).toBe(false)
  expect(validateAmount('99', '100000000')).toBe(false)
})

test('correctly validates a key', () => {
  expect(validateKey('ABCEdoLgy4ETLRa11uDDmtff9tFZZVoKAW4wneQuEyR1')).toBe(true)

  expect(validateKey('Agy4ETLRa11uDDmtff9tFZZVoKAeQuEyR1')).toBe(false)
})

test('correctly validates a version', () => {
  expect(validateVersion('0.11.0')).toBe(true)

  expect(validateVersion('1.2.3')).toBe(false)
})

test('correctly validates a raw port', () => {
  expect(validateRawPort(1)).toBe(true)
  expect(validateRawPort(3000)).toBe(true)

  expect(validateRawPort(9000000)).toBe(false)
})
