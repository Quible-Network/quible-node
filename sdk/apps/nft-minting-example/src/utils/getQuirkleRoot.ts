import { keccak256 } from "viem";

export const getQuirkleRoot = (author: string, slug: string) => {
  const authorBytes = new Uint8Array(author.slice(2).match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)))

  const slugBytes = new TextEncoder().encode(slug)

  const bytes = new Uint8Array(authorBytes.length + slugBytes.length)
  bytes.set(authorBytes, 0)
  bytes.set(slugBytes, authorBytes.length)
  console.log('author', author);
  console.log('quirkle root hex', Array.from(bytes)
              .map((byte) => byte.toString(16).padStart(2, '0')).join(''));
  console.log('quirkle root bytes', bytes);
  return keccak256(bytes);
}
