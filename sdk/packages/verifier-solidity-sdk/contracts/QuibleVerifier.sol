// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts ^5.0.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

library QuibleVerifier {
    function bytesToHexString(bytes memory data) private pure returns (string memory) {
        bytes memory converted = new bytes(data.length * 2);

        bytes memory _base = "0123456789abcdef";

        for (uint256 i = 0; i < data.length; i++) {
            converted[i * 2] = _base[uint8(data[i]) / _base.length];
            converted[i * 2 + 1] = _base[uint8(data[i]) % _base.length];
        }

        return string(abi.encodePacked("0x", converted));
    }

	  function verifyProof(bytes32 quirkleRoot, address member, uint64 expires_at, bytes memory signature) internal view {
        string memory hexAddress = bytesToHexString(abi.encodePacked(bytes20(member)));
        bytes memory message = abi.encodePacked(quirkleRoot, hexAddress, expires_at);
        bytes32 hash = MessageHashUtils.toEthSignedMessageHash(message);
        address signer = ECDSA.recover(hash, signature);

        // TODO: use custom error objects with a revert statement
        require(signer == 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266, "Quible: signature invalid");
        require(block.timestamp < expires_at, "Quible: signature expired");
	  }
}
