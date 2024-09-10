import { useCallback, useState } from "react";
import { waitForTransactionReceipt } from "@wagmi/core";
import { useDeployContract, useConfig, useSignMessage } from "wagmi";
import MyNFTArtifacts from "../../ignition/deployments/chain-31337/artifacts/MyNFTModule#MyNFT.json";
import Link from "next/link";
import ReactEditList, * as REL from "react-edit-list";
import { keccak256 } from "viem";

const schema: REL.Schema = [
  { name: "id", type: "id" },
  { name: "address", type: "string" },
];

const getTransactionHash = (members: string[]) => {
  const memberBytes = members.map((hex) => {
    return hex.slice(2).match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16))
  }).reduce((result, member) => [...result, ...member], []);

  const proofTTLBytes = '00000000000150A0'.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16))

  const bytes = new Uint8Array([...memberBytes, ...proofTTLBytes]);
  return keccak256(bytes);
}

const LaunchToken = (props: {
  accountAddress: string;
  tokenAddress: string;
}) => {
  const [contractAddress, setContractAddress] = useState<string | null>(null);
  const [isPending, setIsPending] = useState(false);
  const [accessList, setAccessList] = useState<string[]>([]);
  const config = useConfig();

  const { signMessageAsync } = useSignMessage()
  const { deployContractAsync } = useDeployContract();

  const handleDeployContract = useCallback(async () => {
    setIsPending(true);

    const quibleTransactionHash = getTransactionHash(accessList);

    console.log('requesting signature', quibleTransactionHash);
    const signature = await signMessageAsync({
      message: { raw: quibleTransactionHash }
    })

    console.log('using signature', signature);

    const response = await fetch(
      'http://localhost:9013',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'quible_sendTransaction',
          id: 67,
          params: [
            {
              signature,
              events: [
                {name: 'CreateQuirkle', members: accessList, proof_ttl: 86400}
              ]
            }
          ]
        })
      }
    )

    const body = await response.json();

    console.log('got body', body);

    const hash = await deployContractAsync({
      abi: MyNFTArtifacts.abi,
      bytecode: MyNFTArtifacts.bytecode as unknown as `0x${string}`,
      args: [props.accountAddress],
    });

    const { contractAddress: newContractAddress } =
      await waitForTransactionReceipt(config, { hash });

    setContractAddress(newContractAddress as unknown as string);
    setIsPending(false);
  }, [props.accountAddress, accessList]);

  const handleAccessListChange = (list: REL.Row[]) => {
    setAccessList(list.map((row) => row.id as string));
  };

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
                style={{ textDecoration: "underline", color: "blue" }}
              >
                Contract deployed at <code>{contractAddress}</code>
              </Link>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default LaunchToken;
