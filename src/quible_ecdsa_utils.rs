use alloy_primitives::{keccak256, Address, B256};
pub(crate) use k256::ecdsa::Error;
use k256::{
    ecdsa::{RecoveryId, SigningKey, VerifyingKey},
    elliptic_curve::bigint::{ArrayDecoding, Encoding},
};

/// Recovers the address of the sender using secp256k1 pubkey recovery.
///
/// Converts the public key into an ethereum address by hashing the public key with keccak256.
///
/// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
/// underlying secp256k1 library.
pub fn recover_signer_unchecked(sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, Error> {
    let mut signature = k256::ecdsa::Signature::from_slice(&sig[0..64])?;
    let mut recid = sig[64] - 27;

    // normalize signature and flip recovery id if needed.
    if let Some(sig_normalized) = signature.normalize_s() {
        signature = sig_normalized;
        recid ^= 1;
    }
    let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");

    // recover key
    let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &signature, recid)?;
    Ok(public_key_to_address(recovered_key))
}

/// Signs message with the given secret key.
/// Returns the corresponding signature.
pub fn sign_message(secret: B256, message: B256) -> Result<[u8; 65], Error> {
    let sec = SigningKey::from_slice(secret.as_ref())?;
    let (sig_object, rec_id) = sec.sign_prehash_recoverable(&message.0)?;
    let (r, s) = sig_object.split_bytes();
    let odd_y_parity = rec_id.is_y_odd();
    let v = u8::from(odd_y_parity) + 27;

    let mut sig = [0u8; 65];
    sig[..32].copy_from_slice(&r.into_uint_be().to_be_bytes());
    sig[32..64].copy_from_slice(&s.into_uint_be().to_be_bytes());
    sig[64] = v;

    Ok(sig)
}

/// Converts a public key into an ethereum address by hashing the encoded public key with
/// keccak256.
pub fn public_key_to_address(public: VerifyingKey) -> Address {
    let hash = keccak256(&public.to_encoded_point(/* compress = */ false).as_bytes()[1..]);
    Address::from_slice(&hash[12..])
}

#[cfg(test)]
mod tests {
    use crate::quible_ecdsa_utils::{
        public_key_to_address, recover_signer_unchecked, sign_message,
    };
    use alloy_primitives::{keccak256, B256};
    use rand;

    #[test]
    fn sanity_ecrecover_call_k256() {
        let secret = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
        let public = *secret.verifying_key();
        let signer = public_key_to_address(public);

        let message = b"hello world";
        let hash = keccak256(message);
        let sig =
            sign_message(B256::from_slice(&secret.to_bytes()[..]), hash).expect("sign message");

        assert_eq!(recover_signer_unchecked(&sig, &hash).ok(), Some(signer));
    }
}
