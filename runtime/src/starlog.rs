use parity_codec_derive::{Decode, Encode};
use rstd::vec::Vec;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap};
use system::ensure_signed;

const BYTEARRAY_LIMIT: usize = 1000;
const IPFS_SHA256_LENGTH: usize = 46;
const IPFS_SHA256_FIRST_BYTE: u8 = 81;
const IPFS_SHA256_SECOND_BYTE: u8 = 109;

#[derive(Debug, Encode, Decode, Default, Clone, PartialEq)]
pub struct Metadata<Time, Balance> {
	// ipfs_hash as Vec<u8> instead of T::Hash to support sha256 base58 and potentially multihash
	ipfs_hash: Vec<u8>,
	time: Time,
	price: Balance, // price 0 = open source
	availability: bool,
	meta_json: Vec<u8>,
	// Contains tag, text, title, filetype as Vec<u8>?
	// searchable?
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
		OwnedMetaIndex: map Vec<u8> => u64;

		HashMeta get(meta_for_hash): map Vec<u8> => Metadata<T::Moment, T::Balance>;
		HashOwner get(owner_of_hash): map Vec<u8> => Option<T::AccountId>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn store_meta(origin, ipfs_hash: Vec<u8>, meta_json: Vec<u8>, price: T::Balance) -> Result {
			// Signature
			let sender = ensure_signed(origin)?;

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

			Self::_user_store(sender, new_metadata)?;

			Ok(())
		}

		fn transfer(origin, receiver: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_ownership_rights_check(sender.clone(), ipfs_hash.clone())?;

			Self::_transfer(sender, receiver, ipfs_hash)?;

			Ok(())
		}

		fn change_price(origin, ipfs_hash: Vec<u8>, price: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_ownership_rights_check(sender.clone(), ipfs_hash.clone())?;

			let mut metadata = Self::meta_for_hash(&ipfs_hash);
			metadata.price = price;

			let meta_index = <OwnedMetaIndex<T>>::get(&ipfs_hash);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<HashMeta<T>>::insert(ipfs_hash, metadata);

			Self::deposit_event(RawEvent::PriceSet(sender, price));
			Ok(())
		}

		//TODO: buy

		//TODO: some kind of voting system for the availibility of data
		fn unavailable(origin, ipfs_hash: Vec<u8>) -> Result{
			let sender = ensure_signed(origin)?;
			Self::_ownership_rights_check(sender.clone(), ipfs_hash.clone())?;

			let mut metadata = Self::meta_for_hash(&ipfs_hash);
			metadata.availability = false;

			let meta_index = <OwnedMetaIndex<T>>::get(&ipfs_hash);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<HashMeta<T>>::insert(ipfs_hash, metadata);

			Self::deposit_event(RawEvent::AvailabilityUpdate(sender, false));
			Ok(())
		}

		//TODO: should the owner have the right to remove it?
	}
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as timestamp::Trait>::Moment,
		<T as balances::Trait>::Balance
    {
        Stored(AccountId, Moment, Vec<u8>),
		TransferOwnership(AccountId, AccountId),
		PriceSet(AccountId, Balance),
		AvailabilityUpdate(AccountId, bool),
    }
);

impl<T: Trait> Module<T> {
	fn _ownership_rights_check(sender: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
		//TODO: Fix always triggers hash doesn't exist
		ensure!(
			<HashMeta<T>>::exists(&ipfs_hash),
			"This hash does not exist"
		);
		let owner = Self::owner_of_hash(ipfs_hash).ok_or("No owner for this hash!")?;
		ensure!(owner == sender, "You are not the owner");
		Ok(())
	}

	fn _transfer(sender: T::AccountId, receiver: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
		let receiver_total_count = Self::user_meta_count(&receiver);
		let new_receiver_count = receiver_total_count
			.checked_add(1)
			.ok_or("Transfer causes overflow of metadata count")?;

		let sender_total_count = Self::user_meta_count(&sender);
		let new_sender_count = sender_total_count
			.checked_sub(1)
			.ok_or("Transfer causes underflow of metadata count")?;

		let meta_index = <OwnedMetaIndex<T>>::get(&ipfs_hash);
		let meta_object = <OwnedMetaArray<T>>::get((sender.clone(), new_sender_count));
		if meta_index != new_sender_count {
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index), &meta_object);
			<OwnedMetaIndex<T>>::insert(&meta_object.ipfs_hash, meta_index);
		}

		<HashOwner<T>>::insert(&ipfs_hash, &receiver);
		<OwnedMetaIndex<T>>::insert(ipfs_hash, receiver_total_count);

		<OwnedMetaArray<T>>::remove((sender.clone(), new_sender_count));
		<OwnedMetaArray<T>>::insert((receiver.clone(), receiver_total_count), meta_object);

		<OwnedMetaCount<T>>::insert(&sender, new_sender_count);
		<OwnedMetaCount<T>>::insert(&receiver, new_receiver_count);

		Self::deposit_event(RawEvent::TransferOwnership(sender, receiver));

		Ok(())
	}

	fn _user_store(user: T::AccountId, metadata: Metadata<T::Moment, T::Balance>) -> Result {
		let count = Self::user_meta_count(&user);
		let updated_count = count.checked_add(1).ok_or("Overflow adding new metadata")?;

		<OwnedMetaArray<T>>::insert((user.clone(), count), &metadata);
		<OwnedMetaCount<T>>::insert(&user, updated_count);
		<OwnedMetaIndex<T>>::insert(&metadata.ipfs_hash, updated_count);

		<HashOwner<T>>::insert(&metadata.ipfs_hash, &user);
		<HashMeta<T>>::insert(&metadata.ipfs_hash, &metadata);

		Self::deposit_event(RawEvent::Stored(user, metadata.time, metadata.ipfs_hash));

		Ok(())
	}
}

//TODO: Refactor/Tests
