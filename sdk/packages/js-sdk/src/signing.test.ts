import { ethers } from 'ethers'
import { makeSigner } from './signing'
import { TransactionContents } from './types'
import {
  convertHexStringToFixedLengthUint8Array,
  convertUint8ArrayToHexString,
} from './utils'

describe('signer', () => {
  it('should sign successfully', async () => {
    const coinbaseHash =
      '0x1fcc126e748fb9c9b1344cbfbc8a506da9ea39d6ad4bae99028ea9b6dcbcda4d'

    const expected =
      '0x00011fcc126e748fb9c9b1344cbfbc8a506da9ea39d6ad4bae99028ea9b6dcbcda4d0000000000000000020041194f5f30986b282faf2d95d7778037c08aab7561e22d98693f48194d3fde56fb6660b808953fad3eff44a47bb704cc90e1802a893f4315f1eabcc76d7f264b5a1b0014fe2df36bc6ca517ebdf208d6e772ce4f4a7c4ce401000500000000000000000000000000000000'

    const signingKey = new ethers.SigningKey(
      '0x6cb79db826dbbb5fb2210ff383d2fb5ce050f2ff59039970387d59d46c5e1f96',
    )

    const sampleTransaction: TransactionContents = {
      inputs: [
        {
          outpoint: {
            txid: convertHexStringToFixedLengthUint8Array(coinbaseHash, 32),
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
            pubkeyScript: [],
          },
        },
      ],
      locktime: 0n,
    }

    const signMessage = async (
      message: Uint8Array,
    ): Promise<Uint8Array & { length: 65 }> => {
      const messageHash = ethers.keccak256(message)
      const signature = signingKey.sign(messageHash)

      return convertHexStringToFixedLengthUint8Array(signature.serialized, 65)
    }

    const { address } = new ethers.Wallet(signingKey)
    const signer = makeSigner(
      convertHexStringToFixedLengthUint8Array(address, 20),
      signMessage,
    )

    const result = await signer.signTransaction(sampleTransaction)
    expect(convertUint8ArrayToHexString(result.toBytes())).toBe(expected)
  })
})
