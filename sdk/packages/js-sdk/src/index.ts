import * as secp256k1 from '@noble/secp256k1'
import { keccak_256 } from '@noble/hashes/sha3'
import { privateKeyToAccount } from 'viem/accounts'
import { EIP191Signer } from '@lukso/eip191-signer.js'
import { RawSignedTransaction, Signer } from './signing'
import {
  convertHexStringToUint8Array,
  convertUint8ArrayToBigInt,
  convertUint8ArrayToHexString,
  encodeUnsigned64BitIntegerLE,
} from './utils'
import {
  TransactionContents,
  TransactionOpCode,
  TransactionOutpoint,
} from './types'

const eip191Signer = new EIP191Signer()

export class Identity {
  public id: { toBytes: () => Uint8Array; toHexString: () => string }

  constructor(id: Uint8Array) {
    this.id = {
      toBytes() {
        return id
      },
      toHexString() {
        return convertUint8ArrayToHexString(id)
      },
    }
  }
}

export type CreateIdentityParams = {
  claims: string[] // TODO: https://linear.app/quible/issue/QUI-114/support-non-string-claim-values-in-sdk
  certificateLifespan: number
}

export { Signer as QuibleSigner }

export class QuibleProvider {
  constructor(public url: string) {}

  getWallet(signer: Signer): QuibleWallet {
    return new QuibleWallet(this, signer)
  }

  // TODO(QUI-36): gracefully handle both node.js and browser environments
  async sendTransaction(rawTransaction: RawSignedTransaction): Promise<void> {
    const response = await fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_sendRawTransaction',
        id: 67,
        params: [rawTransaction.toHexString().slice(2)],
      }),
    })

    const body = await response.json()

    if (body.error) {
      throw new Error(body.error.message)
    }
  }

  async fetchFaucetOutput(): Promise<{
    signer: Signer
    signingKey: Uint8Array
    outpoint: TransactionOutpoint
  }> {
    const response = await fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_requestFaucetOutput',
        id: 67,
        params: [],
      }),
    })

    const { result } = await response.json()

    const signingKey = new Uint8Array(result.owner_signing_key)

    const outpoint: TransactionOutpoint = {
      txid: new Uint8Array(result.outpoint.txid) as Uint8Array & { length: 32 },
      index: convertUint8ArrayToBigInt(result.outpoint.index),
    }

    const { address } = privateKeyToAccount(
      convertUint8ArrayToHexString(signingKey) as `0x${string}`,
    )

    const signer = Signer.fromAddress({ raw: address }, async (message) => {
      const hash = eip191Signer.hashEthereumSignedMessage(
        convertUint8ArrayToHexString(message),
      )

      const signature = await secp256k1.signAsync(hash.slice(2), signingKey)
      return new Uint8Array([
        ...signature.toCompactRawBytes(),
        signature.recovery + 27,
      ])
    })

    return { signer, signingKey, outpoint }
  }
}

export class QuibleWallet {
  constructor(
    public provider: QuibleProvider,
    public signer: Signer,
  ) {}

  async createIdentity(params: CreateIdentityParams): Promise<Identity> {
    const { signer: faucetSigner, outpoint: faucetOutpoint } =
      await this.provider.fetchFaucetOutput()

    const objectId = keccak_256(
      new Uint8Array([
        ...faucetOutpoint.txid,
        ...encodeUnsigned64BitIntegerLE(faucetOutpoint.index),
        ...encodeUnsigned64BitIntegerLE(0n),
      ]),
    ) as Uint8Array & { length: 32 }

    const identityTransaction: TransactionContents = {
      inputs: [{ outpoint: faucetOutpoint, signatureScript: [] }],
      outputs: [
        {
          type: 'Object',
          data: {
            objectId: {
              raw: objectId,
              mode: { type: 'Fresh' },
            },
            dataScript: [
              // { code: 'SETCERTTTL', data: BigInt(params.certificateLifespan) },
              ...(params.claims.map((claim) => ({
                code: 'INSERT',
                data: claim.startsWith('0x')
                  ? convertHexStringToUint8Array(claim)
                  : new TextEncoder().encode(claim),
              })) as TransactionOpCode[]),
            ],
            pubkeyScript: [
              { code: 'DUP' },
              {
                code: 'PUSH',
                data: this.signer.address.toBytes(),
              },
              { code: 'EQUALVERIFY' },
              { code: 'CHECKSIGVERIFY' },
            ],
          },
        },
      ],
      locktime: 0n,
    }

    const encodedSignedIdentityTransaction =
      await faucetSigner.signTransaction(identityTransaction)

    await this.provider.sendTransaction(encodedSignedIdentityTransaction)

    return new Identity(objectId)
  }
}
