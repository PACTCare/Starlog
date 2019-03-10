//! Metadata runtim
use super::{ipfs_hashes, metadata};
use parity_codec_derive::{Decode, Encode};
use rstd::vec::Vec;
use runtime_primitives::traits::{Hash, Zero};
use support::{
	decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

const ERR_HASH_NOT_EXIST: &str = "This hash does not exist";
const ERR_ALREADY_CLAIMED: &str = "This IPFS hash has already been claimed";

const ERR_PRICE_NOT_ZERO: &str = "Free to use content needs to have a price of zero";

const ERR_OPEN_LICENSE_ACCOUNT_CLAIMED: &str = "Open license account already claimed";

const ERR_ALREADY_OWNER: &str = "You are already the owner of this hash.";
const ERR_NOT_OWNER: &str = "You are not the owner";
const ERR_NO_OWNER: &str = "No one ownes this hash";

const ERR_NOT_FOR_SALE: &str = "The hash is not for sale";
const ERR_OVERFLOW: &str = "Overflow adding new metadata";
const ERR_UNDERFLOW: &str = "Underflow removing metadata";

#[derive(Debug, Encode, Decode, Default, Clone, PartialEq)]
pub struct Metadata<Time, AccountId, Balance, Hash> {
	ipfs_hash: Vec<u8>,
	time: Time,
	owner: AccountId, // Specific user accounts for specific licenses, e.g. Alice = free license
	// someone else owner = only right to use it
	// TODO: multiple owners = multiple signatures system
	// https://github.com/paritytech/substrate/pull/1795
	// multi signature system can search for groups
	price: Balance, // price 0 not for sale
	meta_hash: Hash, // https://blog.enjincoin.io/erc-1155-the-crypto-item-standard-ac9cf1c5a226
	                // Contains tag, text, title, filetype as Vec<u8>?
	                // searchable?
	                // because storing an additional metadata hash on IPFS is probably too slow
}

pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MetadataStorage {
		// TODO: Is there a better solution for query for metadata without subscription
		MetadataArray get(metadata_by_index): map u64 => Vec<u8>;
		MetadataCount get(metadata_count): u64;
		MetadataIndex: map Vec<u8> => u64;

		MetaHash get(meta_of_metahash): map T::Hash => Vec<u8>;

		OwnedMetaArray get(metadata_of_user_by_index): map (T::AccountId, u64) => Metadata<T::Moment, T::AccountId, T::Balance, T::Hash>;
		OwnedMetaCount get(user_meta_count): map T::AccountId => u64;
		OwnedMetaIndex: map Vec<u8> => u64;

		HashMeta get(meta_for_hash): map Vec<u8> => Metadata<T::Moment, T::AccountId, T::Balance, T::Hash>;
		HashOwner get(owner_of_hash): map Vec<u8> => Option<T::AccountId>;

		OpenlicenseAccount get(open_license_account): T::AccountId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		//TODO: probably hard-code later
		/// Sets open license account address
		pub fn init_open_license_account(origin) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(!<OpenlicenseAccount<T>>::exists(), ERR_OPEN_LICENSE_ACCOUNT_CLAIMED);
			<OpenlicenseAccount<T>>::put(sender);
			Ok(())
		}

		//TODO: multiple owners = multiple signatures
		/// Store metadata
		pub fn store_meta(origin, ipfs_hash: Vec<u8>, owner: T::AccountId, meta_json: Vec<u8>, price: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;
			ipfs_hashes::check_valid_hash(&ipfs_hash)?;

			metadata::check_valid_meta(&meta_json)?;

			ensure!(!<HashOwner<T>>::exists(&ipfs_hash), ERR_ALREADY_CLAIMED);

			if owner == Self::open_license_account() {
				ensure!(price.is_zero(), ERR_PRICE_NOT_ZERO);
			}
			//hashing of metadata to reduce the size of the chain
			let meta_hash = <T as system::Trait>::Hashing::hash_of(&meta_json);
			//if doesn't exist create metadata entry
			if !<MetaHash<T>>::exists(&meta_hash) {
				<MetaHash<T>>::insert(&meta_hash, &meta_json);
			}

			let time = <timestamp::Module<T>>::now();
			let new_metadata = Metadata {
				ipfs_hash,
				time,
				owner,
				price,
				meta_hash,
			};

			Self::_user_store(sender, new_metadata)?;

			Ok(())
		}

		//TODO: update metadata?

		//TODO: claim ownership?

		//TODO: add additional owner

		/// Transfer the ownership without paying for it
		fn transfer_ownership(origin, receiver: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_ownership_rights(sender.clone(), ipfs_hash.clone())?;
			ipfs_hashes::check_valid_hash(&ipfs_hash)?;

			// open source content needs no ownership transfer
			let metadata = Self::meta_for_hash(&ipfs_hash);
			ensure!(metadata.owner == sender, ERR_NOT_OWNER);

			Self::_transfer(sender, receiver, ipfs_hash)?;

			Ok(())
		}

		/// Change the price
		fn change_price(origin, ipfs_hash: Vec<u8>, price: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_ownership_rights(sender.clone(), ipfs_hash.clone())?;
			ipfs_hashes::check_valid_hash(&ipfs_hash)?;

			let mut metadata = Self::meta_for_hash(&ipfs_hash);
			ensure!(metadata.owner == sender, ERR_NOT_OWNER);
			ensure!(metadata.owner != Self::open_license_account(), ERR_PRICE_NOT_ZERO);

			metadata.price = price;

			let meta_index = <OwnedMetaIndex<T>>::get(&ipfs_hash);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<HashMeta<T>>::insert(ipfs_hash, metadata);

			Self::deposit_event(RawEvent::PriceSet(sender, price));
			Ok(())
		}

		fn buy_meta(origin, ipfs_hash: Vec<u8>, new_owner: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			ipfs_hashes::check_valid_hash(&ipfs_hash)?;
			ensure!(<HashMeta<T>>::exists(&ipfs_hash), ERR_HASH_NOT_EXIST);

			let owner = Self::owner_of_hash(&ipfs_hash).ok_or(ERR_NO_OWNER)?;
			ensure!(owner != sender, ERR_ALREADY_OWNER );

			let metadata = Self::meta_for_hash(&ipfs_hash);
			ensure!(metadata.owner == sender, ERR_NOT_OWNER);

			let price = metadata.price;
			ensure!(!price.is_zero(), ERR_NOT_FOR_SALE);
			 <balances::Module<T>>::make_transfer(&sender, &owner, price)?;

			Self::_transfer(owner.clone(), new_owner, ipfs_hash)?;

			Self::deposit_event(RawEvent::Bought(sender, owner, price));

			Ok(())
		}
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
		Bought(AccountId, AccountId, Balance),
    }
);

impl<T: Trait> Module<T> {
	fn _check_ownership_rights(sender: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
		ensure!(<HashMeta<T>>::exists(&ipfs_hash), ERR_HASH_NOT_EXIST);
		let owner = Self::owner_of_hash(ipfs_hash).ok_or(ERR_NO_OWNER)?;
		ensure!(owner == sender, ERR_NOT_OWNER);

		Ok(())
	}

	fn _transfer(sender: T::AccountId, receiver: T::AccountId, ipfs_hash: Vec<u8>) -> Result {
		let receiver_total_count = Self::user_meta_count(&receiver);
		let new_receiver_count = receiver_total_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let sender_total_count = Self::user_meta_count(&sender);
		let new_sender_count = sender_total_count.checked_sub(1).ok_or(ERR_UNDERFLOW)?;

		let meta_index = <OwnedMetaIndex<T>>::get(&ipfs_hash);
		let mut meta_object = <OwnedMetaArray<T>>::get((sender.clone(), new_sender_count));

		//TODO: tranfer multiple owners
		meta_object.owner = receiver.clone();
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

	fn _user_store(
		user: T::AccountId,
		metadata: Metadata<T::Moment, T::AccountId, T::Balance, T::Hash>,
	) -> Result {
		let count = Self::user_meta_count(&user);
		let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let metadata_count = Self::metadata_count();
		let new_metadata_count = metadata_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		<MetadataArray<T>>::insert(metadata_count, &metadata.ipfs_hash);
		<MetadataCount<T>>::put(new_metadata_count);
		<MetadataIndex<T>>::insert(&metadata.ipfs_hash, metadata_count);

		<OwnedMetaArray<T>>::insert((user.clone(), count), &metadata);
		<OwnedMetaCount<T>>::insert(&user, updated_count);
		<OwnedMetaIndex<T>>::insert(&metadata.ipfs_hash, updated_count);

		<HashMeta<T>>::insert(&metadata.ipfs_hash, &metadata);
		<HashOwner<T>>::insert(&metadata.ipfs_hash, &user);

		Self::deposit_event(RawEvent::Stored(user, metadata.time, metadata.ipfs_hash));

		Ok(())
	}
}

//TODO: Refactor/Tests
/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use primitives::{Blake2Hasher, H256};
	use runtime_io::with_externalities;
	use runtime_primitives::{
		testing::{Digest, DigestItem, Header, UintAuthorityId},
		traits::{BlakeTwo256, IdentityLookup},
		BuildStorage,
	};
	use support::{assert_ok, impl_outer_origin};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	impl consensus::Trait for Test {
		type SessionKey = UintAuthorityId;
		type InherentOfflineReport = ();
		type Log = DigestItem;
	}

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

	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
	}

	impl Trait for Test {
		type Event = ();
	}

	type MetadataStorage = Module<Test>;

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap()
			.0
			.into()
	}

	#[test]
	fn store_meta_works() {
		with_externalities(&mut new_test_ext(), || {
			let owner = 20 as u64;
			let hash = vec![
				81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
				76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
				111, 110, 120, 70, 121, 100, 121,
			];
			let meta = vec![123, 116, 104, 105, 115, 125];
			assert_ok!(MetadataStorage::store_meta(
				Origin::signed(20),
				hash.clone(),
				owner,
				meta,
				4200
			));
			assert_eq!(MetadataStorage::owner_of_hash(hash), Some(owner));
		});
	}

	#[test]
	fn change_price_works() {
		with_externalities(&mut new_test_ext(), || {
			let owner = 20 as u64;
			let hash = vec![
				81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
				76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
				111, 110, 120, 70, 121, 100, 121,
			];
			let meta = vec![123, 116, 104, 105, 115, 125];
			assert_ok!(MetadataStorage::store_meta(
				Origin::signed(20),
				hash.clone(),
				owner,
				meta,
				4200
			));
			assert_ok!(MetadataStorage::change_price(
				Origin::signed(20),
				hash,
				5555
			));
		});
	}

}
