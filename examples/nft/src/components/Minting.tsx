import { useCallback } from 'react'
import { waitForTransactionReceipt, readContract } from '@wagmi/core'
import { useReadContract, useWriteContract, useConfig } from 'wagmi'
import MyNFTArtifacts from '../../artifacts/contracts/MyNFT.sol/MyNFT.json'

const Minting = (props: { accountAddress: string, tokenAddress: string }) => {
  const config = useConfig()
  const { data: hash, writeContractAsync } = useWriteContract()

  const { data, isSuccess, refetch } = useReadContract({
    abi: MyNFTArtifacts.abi,
    address: props.tokenAddress as unknown as `0x${string}`,
    functionName: 'balanceOf',
    args: [props.accountAddress]
  })

  const handleMint = useCallback(async () => {
    console.log('querying quirkle root', props.tokenAddress);
    const quirkleRoot = await readContract(config, {
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as `0x${string}`,
      functionName: 'getQuirkleRoot'
    })

    console.log('got quirkle root', quirkleRoot);

    const response = await fetch(
      'http://localhost:9013',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'quible_requestProof',
          id: 67,
          params: [quirkleRoot, props.accountAddress.toLowerCase(), 0]
        })
      }
    )

    const body = await response.json();

    console.log('got body', body);

    if (body.error) {
      throw new Error(JSON.stringify(body.error));
    }

    const {signature, expires_at} = body.result;

    const hash = await writeContractAsync({
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as unknown as `0x${string}`,
      functionName: 'safeMint',
      args: [props.accountAddress, expires_at, `0x${signature}`]
    })

    await waitForTransactionReceipt(config, { hash })
    refetch()
  }, [props.accountAddress, props.tokenAddress]);

  if (!isSuccess) { return <div>Loading...</div> }

  return (
    <div>
      <button onClick={handleMint}>Mint</button>
      <p>
        total NFT count: {`${data}`}
      </p>
      {hash && <p>Transaction hash: {hash}</p>}
    </div>
  );
};

export default Minting;
