import {
  ObjectIdentifier,
  TransactionContents,
  TransactionOpCode,
} from './types'
import {
  convertUint8ArrayToHexString,
  encodeUnsigned64BitIntegerLE,
} from './utils'

export class EncodedTransaction {
  constructor(public raw: Uint8Array) {}

  toHexString(): string {
    return convertUint8ArrayToHexString(this.raw)
  }

  toBytes(): Uint8Array {
    return this.raw
  }
}

const encodeVarint = (value: bigint): Uint8Array => {
  const result: number[] = []

  do {
    // Extract the least significant 7 bits
    let byte = Number(value & 0x7fn)
    value >>= 7n // Shift value right by 7 bits

    // If there's more to encode, set the MSB to 1
    if (value > 0n) {
      byte |= 0x80
    }

    result.push(byte)
  } while (value > 0n)

  return new Uint8Array(result)
}

const encodeOpCode = (opcode: TransactionOpCode): number[] => {
  switch (opcode.code) {
    case 'PUSH':
      return [0, ...encodeVarint(BigInt(opcode.data.length)), ...opcode.data]
    case 'CHECKSIGVERIFY':
      return [1]
    case 'DUP':
      return [2]
    case 'EQUALVERIFY':
      return [3]
    case 'INSERT':
      return [4, ...encodeVarint(BigInt(opcode.data.length)), ...opcode.data]
    case 'DELETE':
      return [5, ...encodeVarint(BigInt(opcode.data.length)), ...opcode.data]
    case 'DELETEALL':
      return [6]
    case 'SETCERTTTL':
      return [7, ...encodeUnsigned64BitIntegerLE(opcode.data)]
  }
}

const encodeScript = (script: TransactionOpCode[]): Uint8Array => {
  let result: number[] = []

  for (const opcode of script) {
    result = [...result, ...encodeOpCode(opcode)]
  }

  return new Uint8Array(result)
}

const encodeObjectIdentifier = (objectId: ObjectIdentifier): Uint8Array => {
  switch (objectId.mode.type) {
    case 'Fresh':
      return new Uint8Array([...objectId.raw, 0])
    case 'Existing':
      return new Uint8Array([
        ...objectId.raw,
        1,
        ...encodeUnsigned64BitIntegerLE(objectId.mode.permitIndex),
      ])
  }
}

const joinUint8Arrays = (arrays: Uint8Array[]): Uint8Array => {
  const totalLength = arrays.reduce((sum, array) => sum + array.length, 0)

  const result = new Uint8Array(totalLength)

  let offset = 0
  for (const array of arrays) {
    result.set(array, offset)
    offset += array.length
  }

  return result
}

export const encodeTransaction = (transactionContents: TransactionContents) => {
  const { inputs, outputs, locktime } = transactionContents

  const versionNumber = new Uint8Array([0])

  const inputCount = encodeVarint(BigInt(inputs.length))
  const encodedInputs = joinUint8Arrays(
    inputs.map((input) => {
      const encodedSignatureScript = encodeScript(input.signatureScript)
      const encodedSignatureScriptLengthVarint = encodeVarint(
        BigInt(input.signatureScript.length),
      )
      const inputBytes = new Uint8Array(
        40 +
          encodedSignatureScriptLengthVarint.length +
          encodedSignatureScript.length,
      )
      inputBytes.set(input.outpoint.txid, 0)
      inputBytes.set(encodeUnsigned64BitIntegerLE(input.outpoint.index), 32)
      inputBytes.set(encodedSignatureScriptLengthVarint, 40)
      inputBytes.set(
        encodedSignatureScript,
        40 + encodedSignatureScriptLengthVarint.length,
      )

      return inputBytes
    }),
  )

  const outputCount = encodeVarint(BigInt(outputs.length))
  const encodedOutputs = joinUint8Arrays(
    outputs.map((output) => {
      const encodedPubkeyScript = encodeScript(output.data.pubkeyScript)
      const encodedPubkeyScriptLengthVarint = encodeVarint(
        BigInt(output.data.pubkeyScript.length),
      )
      switch (output.type) {
        case 'Value':
          return new Uint8Array([
            0,
            ...encodeUnsigned64BitIntegerLE(output.data.value),
            ...encodedPubkeyScriptLengthVarint,
            ...encodedPubkeyScript,
          ])

        case 'Object':
          const encodedDataScript = encodeScript(output.data.dataScript)
          const encodedDataScriptLengthVarint = encodeVarint(
            BigInt(output.data.dataScript.length),
          )
          return new Uint8Array([
            1,
            ...encodeObjectIdentifier(output.data.objectId),
            ...encodedDataScriptLengthVarint,
            ...encodedDataScript,
            ...encodedPubkeyScriptLengthVarint,
            ...encodedPubkeyScript,
          ])
      }
    }),
  )

  const encodedLocktime = encodeUnsigned64BitIntegerLE(locktime)

  const result = joinUint8Arrays([
    versionNumber,
    inputCount,
    encodedInputs,
    outputCount,
    encodedOutputs,
    encodedLocktime,
  ])

  return new EncodedTransaction(result)
}
