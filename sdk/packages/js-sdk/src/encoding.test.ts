import { encodeTransaction } from './encoding'
import { TransactionContents } from './types'
import { convertHexStringToUint8Array } from './utils'

describe('encoder', () => {
  it('should encode successfully', () => {
    const sampleTransaction: TransactionContents = {
      inputs: [
        {
          outpoint: {
            txid: convertHexStringToUint8Array(
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
                data: convertHexStringToUint8Array(
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
