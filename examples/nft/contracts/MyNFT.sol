// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts ^5.0.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract MyNFT is ERC721, ERC721Enumerable, Ownable {
    uint256 private _nextTokenId;
    uint160 public quirkleRoot;

    constructor(address initialOwner, uint160 _quirkleRoot)
        ERC721("MyNFT", "QMNFT")
        Ownable(initialOwner)
    {
        quirkleRoot = _quirkleRoot;
    }

    modifier membersOnly(address to, uint64 expires_at, bytes memory signature) {
        bytes memory data = abi.encodePacked(quirkleRoot, to, expires_at);
        bytes32 hash = keccak256(data);
        bytes32 signedMessage = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(hash, signature);
        require(signer == 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266, "signature invalid");
        _;
    }

    function safeMint(address to, uint64 expires_at, bytes memory signature) membersOnly(to, expires_at, signature) public {
        uint256 tokenId = _nextTokenId++;
        _safeMint(to, tokenId);

    }

    // The following functions are overrides required by Solidity.

    function _update(address to, uint256 tokenId, address auth)
        internal
        override(ERC721, ERC721Enumerable)
        returns (address)
    {
        return super._update(to, tokenId, auth);
    }

    function _increaseBalance(address account, uint128 value)
        internal
        override(ERC721, ERC721Enumerable)
    {
        super._increaseBalance(account, value);
    }

    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC721, ERC721Enumerable)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}
