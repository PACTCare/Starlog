use parity_codec_derive::{Decode, Encode};
use rstd::vec::Vec;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap};
use system::ensure_signed;

const BYTEARRAY_LIMIT: usize = 1000;
const IPFS_SHA256_LENGTH: usize = 46;
const IPFS_SHA256_FIRST_BYTE: u8 = 81;
const IPFS_SHA256_SECOND_BYTE: u8 = 109;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Metadata<Time, Balance> {
	// ipfs_hash as Vec<u8> instead of T::Hash to support sha256 base58 and potentially multihash
	ipfs_hash: Vec<u8>,
	time: Time,
	// price 0 = open source
	price: Balance,
	availability: bool,
	meta_json: Vec<u8>,
	// Contains tag, text, title, filetype as Vec<u8>?
	// because storing an additional metadata hash on IPFS is probably too slow
	// TODO: availability system
}

pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as StarlogStorage {
		// TODO: query for multiple entries
		OwnedMetaArray get(metadata_of_user_by_index): map (T::AccountId, u64) => Metadata<T::Moment, T::Balance>;
		OwnedMetaCount get(user_meta_count): map T::AccountId => u64;

		HashOwner get(owner_of_hash): map Vec<u8> => Option<T::AccountId>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn store_meta(origin, ipfs_hash: Vec<u8>, meta_json: Vec<u8>, price: T::Balance) -> Result {
			// Signature
			let owner = ensure_signed(origin)?;

			// TODO: multihash support
			ensure!(ipfs_hash.len() == IPFS_SHA256_LENGTH, "Not a valid IPFS Hash");
			ensure!(ipfs_hash[0] == IPFS_SHA256_FIRST_BYTE, "Not a valid IPFS Hash");
			ensure!(ipfs_hash[1] == IPFS_SHA256_SECOND_BYTE, "Not a valid IPFS Hash");

			// TODO: What is the optimal size?
			ensure!(meta_json.len() <= BYTEARRAY_LIMIT, "Bytearray is too large");
			// Checking for Collision
			ensure!(!<HashOwner<T>>::exists(&ipfs_hash), "This IPFS hash has already been claimed");

			let time = <timestamp::Module<T>>::now();
			let new_metadata = Metadata {
				ipfs_hash,
				time,
				price,
				availability: true, //IPFS upload is available, localhost?
				meta_json,
			};

			Self::_user_store(owner, new_metadata)?;

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
        <T as timestamp::Trait>::Moment
    {
        Stored(AccountId, Moment, Vec<u8>),
    }
);

impl<T: Trait> Module<T> {
	fn _user_store(user: T::AccountId, metadata: Metadata<T::Moment, T::Balance>) -> Result {
		let count = Self::user_meta_count(&user);
		let updated_count = count.checked_add(1).ok_or("Overflow adding new metadata")?;

		<OwnedMetaArray<T>>::insert((user.clone(), count), &metadata);
		<OwnedMetaCount<T>>::insert(&user, updated_count);

		<HashOwner<T>>::insert(&metadata.ipfs_hash, &user);

		Self::deposit_event(RawEvent::Stored(user, metadata.time, metadata.ipfs_hash));

		Ok(())
	}
}
