import { useCallback, useState } from 'react'
import { waitForTransactionReceipt, readContract } from '@wagmi/core'
import { useReadContract, useWriteContract, useConfig } from 'wagmi'
import MyNFTArtifacts from '../../artifacts/contracts/MyNFT.sol/MyNFT.json'
import { convertHexStringToUint8Array } from '@quible/js-sdk/lib/utils'

const Minting = (props: { accountAddress: string; tokenAddress: string }) => {
  const [accessListUpdateIsPending, setAccessListUpdateIsPending] =
    useState(false)
  const [accessList, setAccessList] = useState<string[]>([])
  const config = useConfig()
  const { data: hash, writeContractAsync } = useWriteContract()

  const { data: ownerAddressData, isSuccess: ownerAddressDataIsSuccess } =
    useReadContract({
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as unknown as `0x${string}`,
      functionName: 'owner',
      args: [],
    })

  const { data, isSuccess, refetch } = useReadContract({
    abi: MyNFTArtifacts.abi,
    address: props.tokenAddress as unknown as `0x${string}`,
    functionName: 'balanceOf',
    args: [props.accountAddress],
  })

  const handleAccessListUpdate = useCallback(async () => {
    setAccessListUpdateIsPending(true)
    setAccessListUpdateIsPending(false)
  }, [])

  const handleMint = useCallback(async () => {
    console.log(`querying object id tokenAddress=${props.tokenAddress}`)
    const identityId = await readContract(config, {
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as `0x${string}`,
      functionName: 'getAccessListIdentityId',
    })

    console.log('got identity id', identityId)

    const response = await fetch('http://localhost:9013', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'quible_requestCertificate',
        id: 67,
        params: [
          [...convertHexStringToUint8Array(identityId as string)],
          [...convertHexStringToUint8Array(props.accountAddress.toLowerCase())],
        ],
      }),
    })

    const body = await response.json()

    console.log('got body', body)

    if (body.error) {
      throw new Error(JSON.stringify(body.error))
    }

    const {
      signature,
      details: { expires_at },
    } = body.result

    const hash = await writeContractAsync({
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as unknown as `0x${string}`,
      functionName: 'safeMint',
      args: [props.accountAddress, BigInt(expires_at), `0x${signature}`],
    })

    await waitForTransactionReceipt(config, { hash })
    refetch()
  }, [
    props.accountAddress,
    props.tokenAddress,
    config,
    refetch,
    writeContractAsync,
  ])

  if (!isSuccess || !ownerAddressDataIsSuccess) {
    return <div>Loading...</div>
  }

  return (
    <div>
      {ownerAddressData === props.accountAddress && <h1>You are the owner</h1>}
      <button onClick={handleMint}>Mint</button>
      <p>total NFT count: {`${data}`}</p>
      {hash && <p>Transaction hash: {hash}</p>}
    </div>
  )
}

export default Minting
