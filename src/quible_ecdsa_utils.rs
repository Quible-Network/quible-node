use alloy_primitives::{keccak256, Address, B256, U256};

use secp256k1::{
    Error,
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, SecretKey, Secp256k1,
};

/// Recovers the address of the sender using secp256k1 pubkey recovery.
///
/// Converts the public key into an ethereum address by hashing the public key with keccak256.
///
/// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
/// underlying secp256k1 library.
pub fn recover_signer_unchecked(sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, Error> {
    let sig = RecoverableSignature::from_compact(&sig[0..64], RecoveryId::from_i32((sig[64] as i32) - 27)?)?;

    let public = Secp256k1::new().recover_ecdsa(&Message::from_digest(*msg), &sig)?;
    Ok(public_key_to_address(public))
}

/// Signs message with the given secret key.
/// Returns the corresponding signature.
pub fn sign_message(secret: B256, message: B256) -> Result<[u8; 65], Error> {
    let sec = SecretKey::from_slice(secret.as_ref())?;
    let s = Secp256k1::new().sign_ecdsa_recoverable(&Message::from_digest(message.0), &sec);
    let (rec_id, data) = s.serialize_compact();

    let r =  U256::try_from_be_slice(&data[..32]).expect("The slice has at most 32 bytes");
    let s = U256::try_from_be_slice(&data[32..64]).expect("The slice has at most 32 bytes");
    let odd_y_parity = rec_id.to_i32() != 0;

    let mut sig = [0u8; 65];
    sig[0..32].copy_from_slice(&r.to_be_bytes::<32>());
    sig[32..64].copy_from_slice(&s.to_be_bytes::<32>());
    sig[64] = (odd_y_parity as u8) + 27;

    Ok(sig)
}

/// Converts a public key into an ethereum address by hashing the encoded public key with
/// keccak256.
pub fn public_key_to_address(public: PublicKey) -> Address {
    let hash = keccak256(&public.serialize_uncompressed()[1..]);
    Address::from_slice(&hash[12..])
}

#[cfg(test)]
mod tests {
    use crate::quible_ecdsa_utils::{
        public_key_to_address, recover_signer_unchecked, sign_message,
    };
    use alloy_primitives::{keccak256, B256};
    use secp256k1::{SecretKey, Secp256k1};
    use rand;

    #[test]
    fn sanity_ecrecover_call_k256() {
        let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
        let signer = public_key_to_address(public);

        let message = b"hello world";
        let hash = keccak256(message);
        let signature =
            sign_message(B256::from_slice(&secret.secret_bytes()[..]), hash).expect("sign message");

        assert_eq!(recover_signer_unchecked(&signature, &hash), Ok(signer));
    }

    #[test]
    fn web_wallet_signature_compatibility() {
        let web_signature = hex_literal::hex!("0830f316c982a7fd4ff050c8fdc1212a8fd92f6bb42b2337b839f2b4e156f05a359ef8f4acd0b57cdedec7874a865ee07076ab2c81dc9f9de28ced55228587f81c");
        let signer_secret = SecretKey::from_slice(&hex_literal::hex!(
                "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        )).expect("32 bytes, within curve order");
        let web_content = hex_literal::hex!("307866333946643665353161616438384636463463653661423838323732373963666646623932323636");
        let web_content_length = web_content.len();
        let prefix_str = format!("\x19Ethereum Signed Message:\n{}", web_content_length);
        let prefix = prefix_str.as_bytes();
        let mut web_content_prefixed = Vec::with_capacity(prefix.len() + web_content.len());
        web_content_prefixed.extend_from_slice(prefix);
        web_content_prefixed.extend_from_slice(&web_content);
        let public = signer_secret.public_key(&Secp256k1::new());
        let signer = public_key_to_address(public);
        let hash = keccak256(web_content_prefixed);
        let recovered_signer = recover_signer_unchecked(&web_signature, &hash).unwrap();
        assert_eq!(hex::encode(signer), hex::encode(recovered_signer));

        let sig =
            sign_message(B256::from_slice(&signer_secret.secret_bytes()[..]), hash).expect("sign message");

        assert_eq!(recover_signer_unchecked(&sig, &hash).ok(), Some(signer));
        assert_eq!(sig, web_signature);
    }
}
