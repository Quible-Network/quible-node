// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts ^5.0.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@quible/verifier-solidity-sdk/contracts/QuibleVerifier.sol";

contract MyNFT is ERC721, ERC721Enumerable, Ownable {
    uint256 private _nextTokenId;
    bytes32 public accessListIdentityId;

    constructor(address initialOwner, bytes32 _accessListIdentityId)
        ERC721("MyNFT", "QMNFT")
        Ownable(initialOwner)
    {
        accessListIdentityId = _accessListIdentityId;
    }

    modifier membersOnly(address to, uint64 expires_at, bytes memory signature) {
        QuibleVerifier.verifyProof(accessListIdentityId, to, expires_at, signature);
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

    function getAccessListIdentityId() public view returns (bytes32) {
        return accessListIdentityId;
    }
}
