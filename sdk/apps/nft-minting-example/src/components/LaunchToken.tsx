import { useCallback, useState } from 'react'
import { waitForTransactionReceipt } from '@wagmi/core'
import { useDeployContract, useConfig, useSignMessage } from 'wagmi'
import MyNFTArtifacts from '../../artifacts/contracts/MyNFT.sol/MyNFT.json'
import Link from 'next/link'
import ReactEditList, * as REL from 'react-edit-list'
import { QuibleSigner, QuibleProvider } from '@quible/js-sdk'
import { convertHexStringToUint8Array } from '@quible/js-sdk/lib/utils'

const quibleProvider = new QuibleProvider('http://localhost:9013')

const schema: REL.Schema = [
  { name: 'id', type: 'id' },
  { name: 'address', type: 'string' },
]

const LaunchToken = (props: { accountAddress: string }) => {
  const [contractAddress, setContractAddress] = useState<string | null>(null)
  const [isPending, setIsPending] = useState(false)
  const [accessList, setAccessList] = useState<string[]>([props.accountAddress])
  const config = useConfig()

  const { signMessageAsync } = useSignMessage()
  const { deployContractAsync } = useDeployContract()

  const handleDeployContract = useCallback(async () => {
    setIsPending(true)
    const signer = QuibleSigner.fromAddress(
      { raw: props.accountAddress },
      (message) =>
        signMessageAsync({ message: { raw: message } }).then(
          convertHexStringToUint8Array,
        ),
    )

    const wallet = quibleProvider.getWallet(signer)

    const identity = await wallet.createIdentity({
      claims: accessList,
      certificateLifespan: 86400,
    })

    const hash = await deployContractAsync({
      abi: MyNFTArtifacts.abi,
      bytecode: MyNFTArtifacts.bytecode as unknown as `0x${string}`,
      args: [props.accountAddress, identity.id.toHexString()],
    })

    const { contractAddress: newContractAddress } =
      await waitForTransactionReceipt(config, { hash })

    setContractAddress(newContractAddress as unknown as string)
    setIsPending(false)
  }, [
    props.accountAddress,
    accessList,
    config,
    signMessageAsync,
    deployContractAsync,
  ])

  const handleAccessListChange = (list: REL.Row[]) => {
    setAccessList(list.map((row) => row.id as string))
  }

  return (
    <div>
      {isPending ? (
        <div>Loading...</div>
      ) : (
        <>
          <ReactEditList
            schema={schema}
            onLoad={() => [
              { id: props.accountAddress, address: props.accountAddress },
            ]}
            onChange={handleAccessListChange}
          />

          <button onClick={handleDeployContract}>Deploy</button>

          {contractAddress && (
            <div>
              <Link
                href={`/tokens/${contractAddress}`}
                style={{ textDecoration: 'underline', color: 'blue' }}
              >
                Contract deployed at <code>{contractAddress}</code>
              </Link>
            </div>
          )}
        </>
      )}
    </div>
  )
}

export default LaunchToken
