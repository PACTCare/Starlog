use rstd::vec::Vec;
use support::{dispatch::Result, ensure};

const ERR_NO_VALID_META: &str = "No valid metadata";

const BYTEARRAY_LIMIT: usize = 1000;

const ERR_BYTEARRAY_LIMIT: &str = "Bytearray is too large";

// TODO: Improve metadata check,
// but empty metadata for private encrypted files is also useful = logging
pub fn check_valid_meta(meta_json: &Vec<u8>) -> Result {
    ensure!(meta_json.len() <= BYTEARRAY_LIMIT, ERR_BYTEARRAY_LIMIT);
    const META_FIRST_BYTE: u8 = 123; // = {
    ensure!(meta_json[0] == META_FIRST_BYTE, ERR_NO_VALID_META);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use support::assert_ok;

    #[test]
    fn check_valid_meta_works() {
        let meta = vec![123, 116, 104, 105, 115, 125];
        assert_ok!(check_valid_meta(&meta));
    }
}
