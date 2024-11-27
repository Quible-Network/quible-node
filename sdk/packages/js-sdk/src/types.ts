export type TransactionOpCode =
  | { code: 'PUSH'; data: Uint8Array }
  | { code: 'CHECKSIGVERIFY' }
  | { code: 'DUP' }
  | { code: 'EQUALVERIFY' }
  | { code: 'INSERT'; data: Uint8Array }
  | { code: 'DELETE'; data: Uint8Array }
  | { code: 'DELETEALL' }
  | { code: 'SETCERTTTL'; data: bigint }

export type TransactionOutpoint = {
  txid: Uint8Array & { length: 32 }
  index: bigint
}

export type TransactionInput = {
  outpoint: TransactionOutpoint
  signatureScript: TransactionOpCode[]
}

export type ObjectIdentifier = {
  raw: Uint8Array & { length: 32 }
  mode: { type: 'Fresh' } | { type: 'Existing'; permitIndex: bigint }
}

export type TransactionOutput =
  | {
      type: 'Value'
      data: {
        value: bigint
        pubkeyScript: TransactionOpCode[]
      }
    }
  | {
      type: 'Object'
      data: {
        objectId: ObjectIdentifier
        dataScript: TransactionOpCode[]
        pubkeyScript: TransactionOpCode[]
      }
    }

export type TransactionContents = {
  inputs: TransactionInput[]
  outputs: TransactionOutput[]
  locktime: bigint
}
