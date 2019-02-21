use support::{decl_storage, decl_module, StorageMap, dispatch::Result, ensure, decl_event};
use system::ensure_signed;
use rstd::vec::Vec;
use parity_codec_derive::{Encode, Decode};

const BYTEARRAY_LIMIT: usize = 1000;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Metadata<Hash, Time> {
    ipfs_hash: Hash,
	time: Time,
	meta_json: Vec<u8>,
	// Contains tag, text, title, filetype as Vec<u8>?
	// because storing an additional metadata hash on IPFS is probably too slow
	// Later maybe price: Balance,
}

pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
} 

decl_storage! {
	trait Store for Module<T: Trait> as StarlogStorage {
		// TODO: ipfs hash sha256 base58 vs BlakeTwo256 ?
		// how to best link both system ? 
		HashOwner get(owner_of_hash): map T::Hash => Option<T::AccountId>;
		//TODO: owning multiple metadata?
		OwnedMeta get(metadata_of_user): map T::AccountId => Metadata<T::Hash, T::Moment>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn store_hash(origin, ipfs_hash: T::Hash, meta_json: Vec<u8>) -> Result {
			// Signature
			let owner = ensure_signed(origin)?;

			// TODO: What is the optimal size?
			ensure!(meta_json.len() <= BYTEARRAY_LIMIT, "Bytearray is too large");
			// Checking for Collision
			ensure!(!<HashOwner<T>>::exists(&ipfs_hash), "This IPFS hash has already been claimed");

			let time = <timestamp::Module<T>>::now();
			let new_metadata = Metadata {
                ipfs_hash,
				time,
				meta_json,
            };

			<OwnedMeta<T>>::insert(&owner, new_metadata);
			<HashOwner<T>>::insert(ipfs_hash, &owner);

			Self::deposit_event(RawEvent::Stored(owner, ipfs_hash));
			Ok(())
		}

		//TODO: should the owner have the right to remove it?
		
		//TODO: Transfer ownership

		//TODO: availability information
	}
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Stored(AccountId, Hash),
    }
);
