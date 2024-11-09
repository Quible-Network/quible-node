---
sidebar_position: 3
slug: /json-rpc
---

# JSON-RPC API

In order for an application to interact with the Quible Network, either by reading blockchain data or by sending transactions to the network— it must connect to a Quible Node. All nodes provide a JSON-RPC API which can be accessed over HTTP.

[JSON-RPC](https://www.jsonrpc.org/specification) is a stateless, light-weight remote procedure call (RPC) protocol. It defines several data structures and the rules around their processing. It is transport agnostic in that the concepts can be used within the same process, over sockets, over HTTP, or in many various message passing environments. It uses JSON (RFC 4627) as data format.

# Conventions

### Hex value encoding

Two key data types get passed over JSON: unformatted byte arrays and quantities. Both are passed with a hex encoding but with different requirements for formatting.

#### Quantities

When encoding quantities (integers, numbers): encode as hex, prefix with "0x", the most compact representation (slight exception: zero should be represented as "0x0").

Here are some examples:

- 0x41 (65 in decimal)
- 0x400 (1024 in decimal)
- WRONG: 0x (should always have at least one digit - zero is "0x0")
- WRONG: 0x0400 (no leading zeroes allowed)
- WRONG: ff (must be prefixed 0x)

### Unformatted data

When encoding unformatted data (byte arrays, account addresses, hashes, bytecode arrays): encode as hex, prefix with "0x", two hex digits per byte.

Here are some examples:

- 0x41 (size 1, "A")
- 0x004200 (size 3, "0B0")
- 0x (size 0, "")
- WRONG: 0xf0f0f (must be even number of digits)
- WRONG: 004200 (must be prefixed 0x)

### The default block parameter

The following options are possible for the defaultBlock parameter:

- `HEX String` - an integer block number
- `String "earliest"` for the earliest/genesis block
- `String "latest"` - for the latest proposed block
- `String "safe"` - for the latest safe head block
- `String "finalized"` - for the latest finalized block
- `String "pending"` - for the pending state/transactions

# JSON-RPC API Methods

Below, all of the available methods are listed.

### net_version

Returns the current network id.

**Parameters**

None

**Returns**

`String` - The current network id.

The full list of current network IDs is available at [chainlist.org](https://chainlist.org).

- `901313`: Quible Mainnet
- `9990131333`: Quible Testnet

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"net_version","params":[],"id":67}'
// Result
{
  "id":67,
  "jsonrpc": "2.0",
  "result": "3"
}
```

### quible_protocolVersion

Returns the current Quible protocol version.

**Parameters**

None

**Returns**

`String` - The current Quible protocol version

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_protocolVersion","params":[],"id":67}'
// Result
{
  "id":67,
  "jsonrpc": "2.0",
  "result": "54"
}
```

### quible_blockHeight

Returns the height of most recent block.

**Parameters**

None

**Returns**

`QUANTITY` - integer of the current block height the client is on.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_blockHeight","params":[],"id":83}'
// Result
{
  "id":83,
  "jsonrpc": "2.0",
  "result": "0x4b7" // 1207
}
```

### quible_getBlockTransactionCountByHash

Returns the number of transactions in a block from a block matching the given block hash.

**Parameters**

1. `DATA`, 32 Bytes - hash of a block

```js
params: ["0xd03ededb7415d22ae8bac30f96b2d1de83119632693b963642318d87d1bece5b"]
```

**Returns**

`QUANTITY` - integer of the number of transactions in this block.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getBlockTransactionCountByHash","params":["0xd03ededb7415d22ae8bac30f96b2d1de83119632693b963642318d87d1bece5b"],"id":1}'
// Result
{
  "id":1,
  "jsonrpc": "2.0",
  "result": "0x8b" // 139
}
```

### quible_getBlockTransactionCountByNumber

Returns the number of transactions in a block matching the given block height.

**Parameters**

1. `QUANTITY|TAG` - integer of a block height, or the string `"earliest"`, `"latest"`, `"pending"`, `"safe"` or `"finalized"`, as in the [default block parameter](/developers/docs/apis/json-rpc/#default-block).

```js
params: [
  "0x13738ca", // 20396234
]
```

**Returns**

`QUANTITY` - integer of the number of transactions in this block.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getBlockTransactionCountByNumber","params":["0x13738ca"],"id":1}'
// Result
{
  "id":1,
  "jsonrpc": "2.0",
  "result": "0x8b" // 139
}
```

### quible_getUncleCountByBlockHash

Returns the number of uncles in a block from a block matching the given block hash.

**Parameters**

1. `DATA`, 32 Bytes - hash of a block

```js
params: ["0x1d59ff54b1eb26b013ce3cb5fc9dab3705b415a67127a003c3e61eb445bb8df2"]
```

**Returns**

`QUANTITY` - integer of the number of uncles in this block.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getUncleCountByBlockHash","params":["0x1d59ff54b1eb26b013ce3cb5fc9dab3705b415a67127a003c3e61eb445bb8df2"],"id":1}'
// Result
{
  "id":1,
  "jsonrpc": "2.0",
  "result": "0x1" // 1
}
```

### quible_sendRawTransaction

Creates new message call transaction or a contract creation for signed transactions.

**Parameters**

1. `DATA`, The signed transaction data.

```js
params: [
  "0xd46e8dd67c5d32be8d46e8dd67c5d32be8058bb8eb970870f072445675058bb8eb970870f072445675",
]
```

**Returns**

`DATA`, 32 Bytes - the transaction hash, or the zero hash if the transaction is not yet available.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_sendRawTransaction","params":[{see above}],"id":1}'
// Result
{
  "id":1,
  "jsonrpc": "2.0",
  "result": "0xe670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"
}
```

### quible_getBlockByHash

Returns information about a block by hash.

**Parameters**

1. `DATA`, 32 Bytes - Hash of a block.
2. `Boolean` - If `true` it returns the full transaction objects, if `false` only the hashes of the transactions.

```js
params: [
  "0xdc0818cf78f21a8e70579cb46a43643f78291264dda342ae31049421c82d21ae",
  false,
]
```

**Returns**

`Object` - A block object, or `null` when no block was found:

- `number`: `QUANTITY` - the block height. `null` when its pending block.
- `hash`: `DATA`, 32 Bytes - hash of the block. `null` when its pending block.
- `parentHash`: `DATA`, 32 Bytes - hash of the parent block.
- `nonce`: `DATA`, 8 Bytes - hash of the generated proof-of-work. `null` when its pending block.
- `sha3Uncles`: `DATA`, 32 Bytes - SHA3 of the uncles data in the block.
- `logsBloom`: `DATA`, 256 Bytes - the bloom filter for the logs of the block. `null` when its pending block.
- `transactionsRoot`: `DATA`, 32 Bytes - the root of the transaction trie of the block.
- `stateRoot`: `DATA`, 32 Bytes - the root of the final state trie of the block.
- `receiptsRoot`: `DATA`, 32 Bytes - the root of the receipts trie of the block.
- `miner`: `DATA`, 20 Bytes - the address of the beneficiary to whom the mining rewards were given.
- `difficulty`: `QUANTITY` - integer of the difficulty for this block.
- `totalDifficulty`: `QUANTITY` - integer of the total difficulty of the chain until this block.
- `extraData`: `DATA` - the "extra data" field of this block.
- `size`: `QUANTITY` - integer the size of this block in bytes.
- `gasLimit`: `QUANTITY` - the maximum gas allowed in this block.
- `gasUsed`: `QUANTITY` - the total used gas by all transactions in this block.
- `timestamp`: `QUANTITY` - the unix timestamp for when the block was collated.
- `transactions`: `Array` - Array of transaction objects, or 32 Bytes transaction hashes depending on the last given parameter.
- `uncles`: `Array` - Array of uncle hashes.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getBlockByHash","params":["0xdc0818cf78f21a8e70579cb46a43643f78291264dda342ae31049421c82d21ae", false],"id":1}'
// Result
{
{
"jsonrpc": "2.0",
"id": 1,
"result": {
    "difficulty": "0x4ea3f27bc",
    "extraData": "0x476574682f4c5649562f76312e302e302f6c696e75782f676f312e342e32",
    "gasLimit": "0x1388",
    "gasUsed": "0x0",
    "hash": "0xdc0818cf78f21a8e70579cb46a43643f78291264dda342ae31049421c82d21ae",
    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "miner": "0xbb7b8287f3f0a933474a79eae42cbca977791171",
    "mixHash": "0x4fffe9ae21f1c9e15207b1f472d5bbdd68c9595d461666602f2be20daf5e7843",
    "nonce": "0x689056015818adbe",
    "number": "0x1b4",
    "parentHash": "0xe99e022112df268087ea7eafaf4790497fd21dbeeb6bd7a1721df161a6657a54",
    "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "size": "0x220",
    "stateRoot": "0xddc8b0234c2e0cad087c8b389aa7ef01f7d79b2570bccb77ce48648aa61c904d",
    "timestamp": "0x55ba467c",
    "totalDifficulty": "0x78ed983323d",
    "transactions": [
    ],
    "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "uncles": [
    ]
}
}
```

### quible_getBlockByNumber

Returns information about a block by block height.

**Parameters**

1. `QUANTITY|TAG` - integer of a block height, or the string `"earliest"`, `"latest"`, `"pending"`, `"safe"` or `"finalized"`, as in the [default block parameter](/developers/docs/apis/json-rpc/#default-block).
2. `Boolean` - If `true` it returns the full transaction objects, if `false` only the hashes of the transactions.

```js
params: [
  "0x1b4", // 436
  true,
]
```

**Returns**
See [quible_getBlockByHash](#quible_getblockbyhash)

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getBlockByNumber","params":["0x1b4", true],"id":1}'
```

Result see [quible_getBlockByHash](#quible_getblockbyhash)

### quible_getTransactionByHash

Returns the information about a transaction requested by transaction hash.

**Parameters**

1. `DATA`, 32 Bytes - hash of a transaction

```js
params: ["0x88df016429689c079f3b2f6ad39fa052532c56795b733da78a91ebe6a713944b"]
```

**Returns**

`Object` - A transaction object, or `null` when no transaction was found:

- `blockHash`: `DATA`, 32 Bytes - hash of the block where this transaction was in. `null` when its pending.
- `blockHeight`: `QUANTITY` - block height where this transaction was in. `null` when its pending.
- `from`: `DATA`, 20 Bytes - address of the sender.
- `gas`: `QUANTITY` - gas provided by the sender.
- `gasPrice`: `QUANTITY` - gas price provided by the sender in Wei.
- `hash`: `DATA`, 32 Bytes - hash of the transaction.
- `input`: `DATA` - the data send along with the transaction.
- `nonce`: `QUANTITY` - the number of transactions made by the sender prior to this one.
- `to`: `DATA`, 20 Bytes - address of the receiver. `null` when its a contract creation transaction.
- `transactionIndex`: `QUANTITY` - integer of the transactions index position in the block. `null` when its pending.
- `value`: `QUANTITY` - value transferred in Wei.
- `v`: `QUANTITY` - ECDSA recovery id
- `r`: `QUANTITY` - ECDSA signature r
- `s`: `QUANTITY` - ECDSA signature s

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getTransactionByHash","params":["0x88df016429689c079f3b2f6ad39fa052532c56795b733da78a91ebe6a713944b"],"id":1}'
// Result
{
  "jsonrpc":"2.0",
  "id":1,
  "result":{
    "blockHash":"0x1d59ff54b1eb26b013ce3cb5fc9dab3705b415a67127a003c3e61eb445bb8df2",
    "blockHeight":"0x5daf3b", // 6139707
    "from":"0xa7d9ddbe1f17865597fbd27ec712455208b6b76d",
    "gas":"0xc350", // 50000
    "gasPrice":"0x4a817c800", // 20000000000
    "hash":"0x88df016429689c079f3b2f6ad39fa052532c56795b733da78a91ebe6a713944b",
    "input":"0x68656c6c6f21",
    "nonce":"0x15", // 21
    "to":"0xf02c1c8e6114b1dbe8937a39260b5b0a374432bb",
    "transactionIndex":"0x41", // 65
    "value":"0xf3dbb76162000", // 4290000000000000
    "v":"0x25", // 37
    "r":"0x1b5e176d927f8e9ab405058b2d2457392da3e20f328b16ddabcebc33eaac5fea",
    "s":"0x4ba69724e8f69de52f0125ad8b3c5c2cef33019bac3249e2c0a2192766d1721c"
  }
}
```

### quible_getTransactionByBlockHashAndIndex

Returns information about a transaction by block hash and transaction index position.

**Parameters**

1. `DATA`, 32 Bytes - hash of a block.
2. `QUANTITY` - integer of the transaction index position.

```js
params: [
  "0x1d59ff54b1eb26b013ce3cb5fc9dab3705b415a67127a003c3e61eb445bb8df2",
  "0x0", // 0
]
```

**Returns**
See [quible_getTransactionByHash](#quible_gettransactionbyhash)

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getTransactionByBlockHashAndIndex","params":["0x1d59ff54b1eb26b013ce3cb5fc9dab3705b415a67127a003c3e61eb445bb8df2", "0x0"],"id":1}'
```

Result see [quible_getTransactionByHash](#quible_gettransactionbyhash)

### quible_getTransactionByBlockHeightAndIndex

Returns information about a transaction by block height and transaction index position.

**Parameters**

1. `QUANTITY|TAG` - a block height, or the string `"earliest"`, `"latest"`, `"pending"`, `"safe"` or `"finalized"`, as in the [default block parameter](/developers/docs/apis/json-rpc/#default-block).
2. `QUANTITY` - the transaction index position.

```js
params: [
  "0x9c47cf", // 10241999
  "0x24", // 36
]
```

**Returns**
See [quible_getTransactionByHash](#quible_gettransactionbyhash)

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getTransactionByBlockHeightAndIndex","params":["0x9c47cf", "0x24"],"id":1}'
```

Result see [quible_getTransactionByHash](#quible_gettransactionbyhash)


Returns information about a uncle of a block by number and uncle index position.

**Parameters**

1. `QUANTITY|TAG` - a block height, or the string `"earliest"`, `"latest"`, `"pending"`, `"safe"`, `"finalized"`, as in the [default block parameter](/developers/docs/apis/json-rpc/#default-block).
2. `QUANTITY` - the uncle's index position.

```js
params: [
  "0x29c", // 668
  "0x0", // 0
]
```

**Returns**
See [quible_getBlockByHash](#quible_getblockbyhash)

**Note**: An uncle doesn't contain individual transactions.

**Example**

```js
// Request
curl -X POST --data '{"jsonrpc":"2.0","method":"quible_getUncleByBlockHeightAndIndex","params":["0x29c", "0x0"],"id":1}'
```

Result see [quible_getBlockByHash](#quible_getblockbyhash)

