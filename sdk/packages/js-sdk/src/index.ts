import { keccak_256 } from '@noble/hashes/sha3'

export type CreateQuirkleParams = {
  members: string[]
  proofTtl: number
  signMessage: (message: Uint8Array) => Promise<string>
}

export type QuirkleRoot = {
  raw: Uint8Array
  toHex: () => string
}

export type TransactionEvent = {
  name: 'CreateQuirkle'
  members: string[]
  proof_ttl: number
  slug?: string
}

export class QuibleProvider {
  constructor(public url: string) {}

  getWallet(address: string): QuibleWallet {
    return new QuibleWallet(this, address)
  }

  // TODO(QUI-36): gracefully handle both node.js and browser environments
  async sendTransaction(transaction: SignedTransaction): Promise<void> {
    const response = await fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_sendTransaction',
        id: 67,
        params: [transaction],
      }),
    })

    await response.json()
  }
}

export class QuibleWallet {
  constructor(
    public provider: QuibleProvider,
    public address: string,
  ) {}

  prepareCreateQuirkleEvent(
    params: Pick<CreateQuirkleParams, 'members' | 'proofTtl'>,
    slug: string = `${Math.random()}`,
  ): TransactionEvent {
    const { members, proofTtl } = params

    return {
      name: 'CreateQuirkle',
      members,
      proof_ttl: proofTtl,
      slug,
    }
  }

  prepareTransaction(events: TransactionEvent[]): UnsignedTransaction {
    return new UnsignedTransaction(events)
  }

  async createQuirkle(
    params: CreateQuirkleParams,
    slug: string = `${Math.random()}`,
  ): Promise<QuirkleRoot> {
    const event = this.prepareCreateQuirkleEvent(params, slug)
    const unsignedTransaction = this.prepareTransaction([event])
    const signedTransaction = await unsignedTransaction.sign(params.signMessage)

    await this.provider.sendTransaction(signedTransaction)
    const quirkleRoot = this.computeQuirkleRootRaw(slug)

    return {
      raw: quirkleRoot,

      // TODO: reduce duplication with computeQuirkleRoot
      toHex: () =>
        Array.from(quirkleRoot)
          .map((byte) => byte.toString(16).padStart(2, '0'))
          .join(''),
    }
  }

  computeQuirkleRoot(slug: string): string {
    return Array.from(this.computeQuirkleRootRaw(slug))
      .map((byte) => byte.toString(16).padStart(2, '0'))
      .join('')
  }

  computeQuirkleRootRaw(slug: string): Uint8Array {
    const authorBytes = new Uint8Array(
      this.address
        .slice(2)
        .match(/.{1,2}/g)!
        .map((byte) => parseInt(byte, 16)),
    )
    const slugBytes = new TextEncoder().encode(slug)
    const bytes = new Uint8Array(authorBytes.length + slugBytes.length)
    bytes.set(authorBytes, 0)
    bytes.set(slugBytes, authorBytes.length)
    return keccak_256(bytes)
  }
}

export class UnsignedTransaction {
  constructor(public events: TransactionEvent[]) {}

  async sign(
    signer: (message: Uint8Array) => Promise<string>,
  ): Promise<SignedTransaction> {
    // TODO(QUI-35): handle non-broken transaction hashing
    const { members /*, slug */ } = this.events[0]
    const slug = ''
    const encodedMembers = members.map((hex) => new TextEncoder().encode(hex))
    const memberBytesLength = encodedMembers.reduce(
      (acc, curr) => acc + curr.length,
      0,
    )
    const memberBytes = new Uint8Array(memberBytesLength)

    let offset = 0
    for (const member of encodedMembers) {
      memberBytes.set(member, offset)
      offset += member.length
    }

    // TODO(QUI-35): incorporate proof TTL into transaction hash
    // const proofTTLBytes = '00000000000150A0'.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16))
    const proofTTLBytes: number[] = []

    // TODO(QUI-35): incorporate slug into transaction hash
    const slugBytes = new TextEncoder().encode(slug)

    const leftBytes = new Uint8Array([...memberBytes, ...proofTTLBytes])
    const bytes = new Uint8Array(leftBytes.length + slugBytes.length)
    bytes.set(leftBytes, 0)
    bytes.set(slugBytes, leftBytes.length)
    const message = bytes
    const signature = await signer(message)
    return new SignedTransaction(this.events, signature)
  }
}

export class SignedTransaction {
  constructor(
    public events: TransactionEvent[],
    public signature: string,
  ) {}

  toJSON(): any {
    const { signature, events } = this
    return { signature, events }
  }
}
