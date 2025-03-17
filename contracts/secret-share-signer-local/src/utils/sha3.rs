use digest::Digest;
use sha3::Keccak256;

#[allow(dead_code)]
pub fn hasher<D: Digest>(input: &[u8]) -> Vec<u8> {
    let mut hasher = D::new();
    hasher.update(input);
    let result = &hasher.finalize()[..];
    result.to_vec()
}

#[allow(dead_code)]
pub fn keccak256(input: &[u8]) -> Vec<u8> {
    hasher::<Keccak256>(input)
}
