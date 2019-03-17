use rstd::vec::Vec;
use support::{dispatch::Result, ensure};

const BYTEARRAY_LIMIT_GATE: usize = 100;

const ERR_BYTEARRAY_LIMIT: &str = "Bytearray is too large";

const ERR_NO_VALID_HASH: &str = "Not a valid IPFS Hash";

//TODO: support different hash encodings or remove Qm to reduce the storage size
pub fn check_valid_hash(file_hash: &Vec<u8>) -> Result {
    const IPFS_SHA256_BASE58_LENGTH: usize = 46;
    const IPFS_SHA256_BASE58_FIRST_BYTE: u8 = 81; // = Q
    const IPFS_SHA256_BASE58_SECOND_BYTE: u8 = 109; // = m

    ensure!(
        file_hash.len() == IPFS_SHA256_BASE58_LENGTH,
        ERR_NO_VALID_HASH
    );
    ensure!(
        file_hash[0] == IPFS_SHA256_BASE58_FIRST_BYTE,
        ERR_NO_VALID_HASH
    );
    ensure!(
        file_hash[1] == IPFS_SHA256_BASE58_SECOND_BYTE,
        ERR_NO_VALID_HASH
    );

    Ok(())
}

pub fn check_valid_gateway(gateway: &Vec<u8>) -> Result {
    ensure!(gateway.len() <= BYTEARRAY_LIMIT_GATE, ERR_BYTEARRAY_LIMIT);
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

    #[test]
    fn check_valid_gateway_works() {
        let meta = vec![123, 116, 104, 105, 115, 125];
        assert_ok!(check_valid_gateway(&meta));
    }
}
