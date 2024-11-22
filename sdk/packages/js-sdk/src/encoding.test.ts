import { encodeTransaction } from './encoding'
import { TransactionContents } from './types'

const hexStringToUint8Array = (hex: string): Uint8Array => {
  if (hex.length % 2 !== 0) {
    throw new Error('Hex string must have an even length')
  }

  if (hex.startsWith('0x')) {
    hex = hex.slice(2)
  }

  const length = hex.length / 2
  const result = new Uint8Array(length)

  for (let i = 0; i < length; i++) {
    const byte = hex.slice(i * 2, i * 2 + 2)
    result[i] = parseInt(byte, 16)
  }

  return result
}

describe('encoder', () => {
  it('should encode successfully', () => {
    const sampleTransaction: TransactionContents = {
      inputs: [
        {
          outpoint: {
            txid: hexStringToUint8Array(
              '0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff',
            ) as Uint8Array & { length: 32 },
            index: 0n,
          },

          signatureScript: [],
        },
      ],

      outputs: [
        {
          type: 'Value',
          data: {
            value: 5n,
            pubkeyScript: [
              { code: 'DUP' },
              {
                code: 'PUSH',
                data: hexStringToUint8Array(
                  '0x7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f',
                ) as Uint8Array & { length: 32 },
              },
              { code: 'EQUALVERIFY' },
              { code: 'CHECKSIGVERIFY' },
            ],
          },
        },
      ],

      locktime: 0n,
    }

    expect(encodeTransaction(sampleTransaction).toHexString()).toBe(
      '0x0001ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00000000000000000001000500000000000000040200147f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f03010000000000000000',
    )
  })
})
