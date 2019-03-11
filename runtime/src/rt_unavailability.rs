//! Availibility runtim
use rstd::vec::Vec;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap};
use system::ensure_signed;

use super::metadata_checks;

const ERR_NO_NA_ENTRY: &str = "No unavailibility entry";
const ERR_ALREADY_MARKET: &str = "Already market as unvailable";
const ERR_OVERFLOW: &str = "Overflow adding new unavailibility data";

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as UnavailableStorage {
        // Subscribtion based system load availibility information by accountId
        UnavailabledataArray get(unavailable_data_by_index): map (T::AccountId, u64) => Vec<u8>;
        UnavailabledataCount get(user_unavailable_count): map T::AccountId => u64;
        UnavailabledataHashCount get(user_unavailable_hash_count): map (T::AccountId, Vec<u8>) => u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn mark_as_unavailable(origin, ipfs_hash: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
            metadata_checks::check_valid_hash(&ipfs_hash)?;

            ensure!(!<UnavailabledataHashCount<T>>::exists((who.clone(),ipfs_hash.clone())), ERR_ALREADY_MARKET);

            let count = Self::user_unavailable_count(&who);
            let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

            <UnavailabledataHashCount<T>>::insert((who.clone(),ipfs_hash.clone()), &count);
            <UnavailabledataArray<T>>::insert((who.clone(), count), &ipfs_hash);
            <UnavailabledataCount<T>>::insert(&who, updated_count);


            Self::deposit_event(RawEvent::UnavailableStored(who, ipfs_hash));
            Ok(())
        }

        //TODO: test
        pub fn remove_unavailable_entry(origin, ipfs_hash: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<UnavailabledataHashCount<T>>::exists((sender.clone(),ipfs_hash.clone())), ERR_NO_NA_ENTRY);
            let count = Self::user_unavailable_hash_count((sender.clone(), ipfs_hash.clone()));

            <UnavailabledataArray<T>>::remove((sender.clone(), count));
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

    impl balances::Trait for Test {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type EnsureAccountLiquid = ();
        type Event = ();
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
    fn mark_as_unavailable_works() {
        with_externalities(&mut new_test_ext(), || {
            let hash = vec![
                81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
                76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
                111, 110, 120, 70, 121, 100, 121,
            ];
            assert_ok!(UnavailableStorage::mark_as_unavailable(
                Origin::signed(1),
                hash.clone()
            ));
            // assert_eq!(
            //     UnavailableStorage::user_unavailable_hash_count((Origin::signed(1), hash)),
            //     0
            // );
        });
    }

    #[test]
    fn remove_unavailable_entry_works() {
        with_externalities(&mut new_test_ext(), || {
            let hash = vec![
                81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
                76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
                111, 110, 120, 70, 121, 100, 121,
            ];
            assert_ok!(UnavailableStorage::mark_as_unavailable(
                Origin::signed(1),
                hash.clone()
            ));
            assert_ok!(UnavailableStorage::remove_unavailable_entry(
                Origin::signed(1),
                hash.clone()
            ));
        });
    }
}
