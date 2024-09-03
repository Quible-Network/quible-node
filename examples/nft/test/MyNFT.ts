import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { expect } from "chai";
import hre from "hardhat";

describe("MyNFT", function () {
  async function deployFixture() {
    const [owner, otherAccount] = await hre.ethers.getSigners();

    const MyNFTFactory = await hre.ethers.getContractFactory("MyNFT");
    const myNFT = await MyNFTFactory.deploy(owner);

    return { myNFT, owner, otherAccount };
  }

  it("should handle safeMint", async function () {
    const { myNFT, otherAccount } = await loadFixture(deployFixture);

    await myNFT.safeMint(otherAccount);
    expect(await myNFT.balanceOf(otherAccount)).to.equal(1);
  });
});
