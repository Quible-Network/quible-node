import { useCallback, useState } from "react";
import { waitForTransactionReceipt } from "@wagmi/core";
import { useDeployContract, useConfig, useSignMessage } from "wagmi";
import MyNFTArtifacts from "../../artifacts/contracts/MyNFT.sol/MyNFT.json";
import Link from "next/link";
import ReactEditList, * as REL from "react-edit-list";
import { keccak256 } from "viem";
import { getQuirkleRoot } from "../utils/getQuirkleRoot";

const schema: REL.Schema = [
  { name: "id", type: "id" },
  { name: "address", type: "string" },
];

const getTransactionHash = (members: string[], slug: string) => {
  /*
  const memberBytes = members.map((hex) => {
    return hex.slice(2).match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16))
  }).reduce((result, member) => [...result, ...member], []);
   */
  const encodedMembers = members.map((hex) => new TextEncoder().encode(hex))
  const memberBytesLength = encodedMembers.reduce((acc, curr) => acc + curr.length, 0)
  const memberBytes = new Uint8Array(memberBytesLength);

  let offset = 0;
  for (const member of encodedMembers) {
    memberBytes.set(member, offset);
    offset += member.length;
  }

  // const proofTTLBytes = '00000000000150A0'.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16))
  const proofTTLBytes: number[] = []

  const slugBytes = new TextEncoder().encode(slug);

  const leftBytes = new Uint8Array([...memberBytes, ...proofTTLBytes]);
  const bytes = new Uint8Array(leftBytes.length + slugBytes.length)
  bytes.set(leftBytes, 0)
  bytes.set(slugBytes, leftBytes.length)
  return {
    hash: keccak256(bytes),
    contentBytes: bytes,
    content: Array.from(bytes)
        .map(byte => byte.toString(16).padStart(2, '0'))
        .join('')
  };
}

const LaunchToken = (props: {
  accountAddress: string;
}) => {
  const [contractAddress, setContractAddress] = useState<string | null>(null);
  const [isPending, setIsPending] = useState(false);
  const [accessList, setAccessList] = useState<string[]>([props.accountAddress]);
  const config = useConfig();

  const { signMessageAsync } = useSignMessage()
  const { deployContractAsync } = useDeployContract();

  const handleDeployContract = useCallback(async () => {
    const slug = `${Math.random()}`
    setIsPending(true);

    const { content, contentBytes, hash: quibleTransactionHash } = getTransactionHash(accessList, '');
    console.log(
      'got hash content',
      content
    );

    console.log(
      'got hash',
      quibleTransactionHash
    );

    console.log('requesting signature', quibleTransactionHash);
    const signature = await signMessageAsync({
      message: { raw: contentBytes }
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
                {name: 'CreateQuirkle', members: accessList, proof_ttl: 86400, slug}
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
      args: [props.accountAddress, getQuirkleRoot(props.accountAddress, slug)],
    });

    const { contractAddress: newContractAddress } =
      await waitForTransactionReceipt(config, { hash });

    setContractAddress(newContractAddress as unknown as string);
    setIsPending(false);
  }, [props.accountAddress, accessList, config, signMessageAsync, deployContractAsync]);

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
