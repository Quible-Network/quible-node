import { useCallback } from 'react'
import { waitForTransactionReceipt } from '@wagmi/core'
import { useReadContract, useWriteContract, useConfig } from 'wagmi'
import MyNFTArtifacts from '../../ignition/deployments/chain-31337/artifacts/MyNFTModule#MyNFT.json'

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
    const hash = await writeContractAsync({
      abi: MyNFTArtifacts.abi,
      address: props.tokenAddress as unknown as `0x${string}`,
      functionName: 'safeMint',
      args: [props.accountAddress]
    })

    await waitForTransactionReceipt(config, { hash })
    refetch()
  }, []);

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
