export const convertUint8ArrayToHexString = (
  uint8Array: Uint8Array,
): string => {
  const hexBytes = Array.from(uint8Array)
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('')

  return `0x${hexBytes}`
}

export const convertHexStringToUint8Array = (hex: string): Uint8Array => {
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
