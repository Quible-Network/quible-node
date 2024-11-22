export {}

type TransactionOpCode =
  | { code: 'PUSH'; data: Uint8Array }
  | { code: 'CHECKSIGVERIFY' }
  | { code: 'DUP' }
  | { code: 'EQUALVERIFY' }
  | { code: 'INSERT'; data: Uint8Array }
  | { code: 'DELETE'; data: Uint8Array }
  | { code: 'DELETEALL' }
  | { code: 'SETCERTTTL'; data: bigint }

type TransactionOutpoint = {
  txid: Uint8Array & { length: 32 }
  index: bigint
}

type TransactionInput = {
  outpoint: TransactionOutpoint
  signatureScript: TransactionOpCode[]
}

type ObjectIdentifier = {
  raw: Uint8Array & { length: 32 }
  mode: { type: 'Fresh' } | { type: 'Existing'; permitIndex: bigint }
}

type TransactionOutput =
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

type TransactionContents = {
  inputs: TransactionInput[]
  outputs: TransactionOutput[]
  locktime: bigint
}

class EncodedTransaction {
  constructor(public raw: Uint8Array) {}

  toHexString(): string {
    const hexBytes = Array.from(this.raw)
      .map((byte) => byte.toString(16).padStart(2, '0'))
      .join('')

    return `0x${hexBytes}`
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

const encodeUnsigned64BitIntegerLE = (value: bigint): Uint8Array => {
  const array = new Uint8Array(8)

  for (let i = 0; i < 8; i++) {
    array[i] = Number(value & 0xffn) // Get the least significant byte
    value >>= 8n // Shift right by 8 bits (1 byte)
  }

  return array
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

class Transaction {
  static of(contents: TransactionContents) {
    return new Transaction(contents)
  }

  constructor(public contents: TransactionContents) {}

  encode(): EncodedTransaction {
    const { inputs, outputs, locktime } = this.contents

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
              BigInt(encodedDataScript.length),
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
}

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

    expect(Transaction.of(sampleTransaction).encode().toHexString()).toBe(
      '0x0001ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00000000000000000001000500000000000000040200147f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f03010000000000000000',
    )
  })
})
