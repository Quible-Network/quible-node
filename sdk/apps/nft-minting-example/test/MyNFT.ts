import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { expect } from "chai";
import hre from "hardhat";

describe("MyNFT", function () {
  async function deployFixture() {
    const [owner, otherAccount] = await hre.ethers.getSigners();

    const MyNFTFactory = await hre.ethers.getContractFactory("MyNFT");
    const myNFT = await MyNFTFactory.deploy(owner, '0x289acd8eac89dba64b50df6342ec1d79d15994a0bd622023f5a51a1b2ab96386');

    return { myNFT, owner, otherAccount };
  }

  it("should handle safeMint", async function () {
    const { myNFT } = await loadFixture(deployFixture);

    const memberAddress = '0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266'

    await myNFT.safeMint(memberAddress, 9999999999n, "0x8b244d180756558fb47cdd3f5e17abb13badea298c17f1fa84659b6e29ef4ef822415b1288219fddd549ed8df83951f55ee9f929a136bf8f82b6f7f81b00d4b51c");
    expect(await myNFT.balanceOf(memberAddress)).to.equal(1);
  });
});
