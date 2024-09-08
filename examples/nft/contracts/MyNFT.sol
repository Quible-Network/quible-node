// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts ^5.0.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import "@openzeppelin/contracts/utils/Strings.sol";

function bytesToHexString(bytes memory data) pure returns (string memory) {
    bytes memory converted = new bytes(data.length * 2);

    bytes memory _base = "0123456789abcdef";

    for (uint256 i = 0; i < data.length; i++) {
        converted[i * 2] = _base[uint8(data[i]) / _base.length];
        converted[i * 2 + 1] = _base[uint8(data[i]) % _base.length];
    }

    return string(abi.encodePacked("0x", converted));
}

contract MyNFT is ERC721, ERC721Enumerable, Ownable {
    uint256 private _nextTokenId;
    bytes32 public quirkleRoot;

    constructor(address initialOwner, bytes32 _quirkleRoot)
        ERC721("MyNFT", "QMNFT")
        Ownable(initialOwner)
    {
        quirkleRoot = _quirkleRoot;
    }

    modifier membersOnly(address to, uint64 expires_at, bytes memory signature) {
        string memory hexAddress = bytesToHexString(abi.encodePacked(bytes20(to)));
        bytes memory message = abi.encodePacked(quirkleRoot, hexAddress, expires_at);
        bytes memory data = abi.encodePacked("\x19Ethereum Signed Message:\n", Strings.toString(message.length), message);
        bytes32 hash = keccak256(data);
        bytes32 signedMessage = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(hash, signature);
        require(signer == 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266, "signature invalid");
        require(block.timestamp < expires_at, "signature expired");
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

    function getQuirkleRoot() public view returns (bytes32) {
        return quirkleRoot;
    }
}
