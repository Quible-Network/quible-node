import { encodeTransaction } from './encoding'
import { TransactionContents } from './types'
import {
  convertHexStringToUint8Array,
  convertUint8ArrayToHexString,
} from './utils'

export type RawSignedTransaction = {
  toBytes: () => Uint8Array
  toHexString: () => string
}

export class Address {
  constructor(private bytes: Uint8Array) {}

  toBytes(): Uint8Array {
    return this.bytes
  }

  toHex(): string {
    return convertUint8ArrayToHexString(this.bytes)
  }
}

export class Signer {
  static fromAddress(
    address: Uint8Array | { raw: string },
    signMessage: (message: Uint8Array) => Promise<Uint8Array>,
  ): Signer {
    if (address instanceof Uint8Array) {
      return makeSigner(address, signMessage)
    }

    return makeSigner(convertHexStringToUint8Array(address.raw), signMessage)
  }

  constructor(
    public address: Address,
    public signTransaction: (
      transaction: TransactionContents,
    ) => Promise<RawSignedTransaction>,
  ) {}
}

export const makeSigner = (
  address: Uint8Array,
  signMessage: (message: Uint8Array) => Promise<Uint8Array>,
): Signer => {
  const signTransaction = async (
    transactionContents: TransactionContents,
  ): Promise<RawSignedTransaction> => {
    const encodedUnsignedTransaction = encodeTransaction(transactionContents)
    const signature = await signMessage(encodedUnsignedTransaction.toBytes())
    const signedTransactionContents: TransactionContents = {
      inputs: transactionContents.inputs.map((input) => ({
        ...input,
        signatureScript: [
          { code: 'PUSH', data: signature },
          { code: 'PUSH', data: address },
        ],
      })),
      outputs: transactionContents.outputs,
      locktime: transactionContents.locktime,
    }

    console.log(signedTransactionContents)

    return encodeTransaction(signedTransactionContents)
  }

  return new Signer(new Address(address), signTransaction)
}
