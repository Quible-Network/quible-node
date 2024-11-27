import { encodeTransaction } from './encoding'
import { TransactionContents } from './types'

export type RawSignedTransaction = {
  toBytes: () => Uint8Array
  toHexString: () => string
}

export type Signer = {
  signTransaction: (
    transaction: TransactionContents,
  ) => Promise<RawSignedTransaction>
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

    return encodeTransaction(signedTransactionContents)
  }

  return { signTransaction }
}
