import * as secp256k1 from '@noble/secp256k1'
import { keccak_256 } from '@noble/hashes/sha3'
import { privateKeyToAccount } from 'viem/accounts'
import { EIP191Signer } from '@lukso/eip191-signer.js'
import { RawSignedTransaction, Signer } from './signing'
import {
  convertHexStringToFixedLengthUint8Array,
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
import { encodeTransaction } from './encoding'

const eip191Signer = new EIP191Signer()

export type QuibleClaim = { hex: string } | { raw: Uint8Array } | string

export type QuibleIdentityUpdateParams = {
  wallet: QuibleWallet
  insert?: QuibleClaim[]
  delete?: QuibleClaim[]
  certificateLifespan?: bigint
}

export type IdentityId = {
  toBytes: () => Uint8Array & { length: 32 }
  toHexString: () => string
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

  async fetchOutputsByObjectId(objectId: Uint8Array & { length: 32 }): Promise<{
    outpoint: TransactionOutpoint
  }> {
    const response = await fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_getUnspentObjectOutputsByObjectId',
        id: 67,
        params: [objectId],
      }),
    })

    const { result } = await response.json()

    if (result.outpoints.length === 0) {
      throw new Error('failed to fetch outputs by object id: no outputs')
    }

    const outpoint: TransactionOutpoint = {
      txid: new Uint8Array(result.outpoints[0].txid) as Uint8Array & {
        length: 32
      },
      index: convertUint8ArrayToBigInt(result.outpoints[0].index),
    }

    return { outpoint }
  }

  async fetchClaimsByObjectId(objectId: Uint8Array & { length: 32 }): Promise<{
    claims: Uint8Array[]
  }> {
    const response = await fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_getClaimsByObjectId',
        id: 67,
        params: [objectId],
      }),
    })

    const {
      result: { claims },
    } = await response.json()

    return { claims }
  }
}

export type GetCertificateParams = {
  claims: QuibleClaim[]
}

export class Identity {
  public id: IdentityId

  async update(params: QuibleIdentityUpdateParams) {
    const { wallet } = params
    const { signer: faucetSigner, outpoint: faucetOutpoint } =
      await wallet.provider.fetchFaucetOutput()

    const { outpoint } = await wallet.provider.fetchOutputsByObjectId(
      this.id.toBytes(),
    )

    const transaction: TransactionContents = {
      inputs: [
        { outpoint: faucetOutpoint, signatureScript: [] },
        { outpoint, signatureScript: [] },
      ],
      outputs: [
        {
          type: 'Object',
          data: {
            objectId: {
              raw: this.id.toBytes(),
              mode: { type: 'Existing', permitIndex: 0n },
            },
            dataScript: [
              // { code: 'SETCERTTTL', data: BigInt(params.certificateLifespan) },
              ...((params.insert ?? []).map((claim) => {
                let data: Uint8Array

                if (typeof claim === 'string') {
                  data = new TextEncoder().encode(claim)
                } else if ('hex' in claim) {
                  data = convertHexStringToUint8Array(claim.hex)
                } else {
                  data = claim.raw
                }

                return {
                  code: 'INSERT',
                  data,
                }
              }) as TransactionOpCode[]),
            ],
            pubkeyScript: [
              { code: 'DUP' },
              {
                code: 'PUSH',
                data: wallet.signer.address.toBytes(),
              },
              { code: 'EQUALVERIFY' },
              { code: 'CHECKSIGVERIFY' },
            ],
          },
        },
      ],
      locktime: 0n,
    }

    const faucetSignedTransaction =
      await faucetSigner.signTransaction(transaction)
    const walletSignedTransaction =
      await wallet.signer.signTransaction(transaction)

    faucetSignedTransaction.contents.inputs[1].signatureScript =
      walletSignedTransaction.contents.inputs[1].signatureScript

    await wallet.provider.sendTransaction(
      encodeTransaction(faucetSignedTransaction.contents),
    )
  }

  public static fromHexString(identityId: string) {
    return new Identity(convertHexStringToFixedLengthUint8Array(identityId, 32))
  }

  public static fromUint8Array(identityId: Uint8Array) {
    if (identityId.length === 32) {
      return new Identity(identityId as Uint8Array & { length: 32 })
    }

    throw new Error('Identity.fromUint8Array: expected length 32')
  }

  private constructor(id: Uint8Array & { length: 32 }) {
    this.id = {
      toBytes() {
        return id
      },
      toHexString() {
        return convertUint8ArrayToHexString(id)
      },
    }
  }

  async getCertificate(params: GetCertificateParams) {
    if (params.claims.length !== 1) {
      throw new Error(
        'Identity#getCertificate: only one claim per certificate allowed',
      )
    }
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

    const signedIdentityTransaction =
      await faucetSigner.signTransaction(identityTransaction)

    await this.provider.sendTransaction(signedIdentityTransaction.encode())

    return Identity.fromUint8Array(objectId)
  }
}
