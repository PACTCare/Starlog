use rstd::vec::Vec;
use support::{dispatch::Result, ensure};

const ERR_NO_VALID_HASH: &str = "Not a valid IPFS Hash";

//TODO: support different hash encodings
pub fn check_valid_hash(ipfs_hash: &Vec<u8>) -> Result {
    const IPFS_SHA256_BASE58_LENGTH: usize = 46;
    const IPFS_SHA256_BASE58_FIRST_BYTE: u8 = 81; // = Q
    const IPFS_SHA256_BASE58_SECOND_BYTE: u8 = 109; // = m

    ensure!(
        ipfs_hash.len() == IPFS_SHA256_BASE58_LENGTH,
        ERR_NO_VALID_HASH
    );
    ensure!(
        ipfs_hash[0] == IPFS_SHA256_BASE58_FIRST_BYTE,
        ERR_NO_VALID_HASH
    );
    ensure!(
        ipfs_hash[1] == IPFS_SHA256_BASE58_SECOND_BYTE,
        ERR_NO_VALID_HASH
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use support::assert_ok;

    #[test]
    fn check_valid_hash_works() {
        let hash = vec![
            81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69, 76,
            97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112, 111,
            110, 120, 70, 121, 100, 121,
        ];
        assert_ok!(check_valid_hash(&hash));
    }
}
