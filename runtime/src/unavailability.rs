/// Availibility runtim
use rstd::vec::Vec;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap};
use system::ensure_signed;

const ERR_NO_VALID_HASH: &str = "Not a valid IPFS Hash";
const ERR_OVERFLOW: &str = "Overflow adding new unavailibility data";

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as UnavailableStorage {
        // Subscribtion based system load availibility information by accountId
        UnavailabledataArray get(unavailable_data_by_index): map (T::AccountId, u64) => Vec<u8>;
        UnavailabledataCount get(user_unavailable_count): map T::AccountId => u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn mark_as_unavailable(origin, ipfs_hash: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
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

            let count = Self::user_unavailable_count(&who);
            let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

            //TODO: only check if hash exists
            <UnavailabledataArray<T>>::insert((who.clone(), count), &ipfs_hash);
            <UnavailabledataCount<T>>::insert(&who, updated_count);

            Self::deposit_event(RawEvent::UnavailableStored(who, ipfs_hash));
            Ok(())
        }

        //TODO:
        pub fn remove_unavailable_entry(origin, ipfs_hash: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId
    {
        UnavailableStored(AccountId, Vec<u8>),
    }
);

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher, H256};
    use runtime_io::with_externalities;
    use runtime_primitives::{
        testing::{Digest, DigestItem, Header},
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_ok, impl_outer_origin};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<u64>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    impl Trait for Test {
        type Event = ();
    }
    type UnavailableStorage = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0
            .into()
    }

    #[test]
    fn it_works_for_default_value() {
        with_externalities(&mut new_test_ext(), || {
            // Just a dummy test for the dummy funtion `do_something`
            // calling the `do_something` function with a value 42
            assert_ok!(UnavailableStorage::do_something(Origin::signed(1), 42));
            // asserting that the stored value is equal to what we stored
            assert_eq!(UnavailableStorage::something(), Some(42));
        });
    }
}
