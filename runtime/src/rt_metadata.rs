//! Metadata runtim
use super::metadata_checks;
use parity_codec::{Encode, Decode};
use rstd::vec::Vec;
use runtime_primitives::traits::Zero;
use support::{
	decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// TODO: support more than just IPFS

// TODO: resources used == price paid

// TODO: multiple owners = multiple signatures system
// https://github.com/paritytech/substrate/pull/1795
// multi signature system can search for groups

const ERR_HASH_NOT_EXIST: &str = "This hash does not exist";
const ERR_ALREADY_CLAIMED: &str = "This hash has already been claimed";
const ERR_DIFFERENT_HASHES: &str = "The file and meta hash cannot be the same";

const ERR_PRICE_NOT_ZERO: &str = "Free to use content needs to have a price of zero";
const ERR_INVALID_LICENSE: &str = "Invalid license code";

const ERR_ALREADY_OWNER: &str = "You are already the owner of this hash.";
const ERR_NOT_OWNER: &str = "You are not the owner";
const ERR_NO_OWNER: &str = "No one ownes this hash";

const ERR_NOT_FOR_SALE: &str = "The hash is not for sale";
const ERR_OVERFLOW: &str = "Overflow adding new metadata";
const ERR_UNDERFLOW: &str = "Underflow removing metadata";

pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// Key requirment for metadata entry it's not only hosted locally!
#[derive(Debug, Encode, Decode, Default, Clone, PartialEq)]
pub struct Metalog<Time, Price> {
	pub file_hash: Vec<u8>,
	pub time: Time,
	pub license_code: u16, // 0 = Copyright, 1 = free licence, 2 = Delete in network, 3 = Private, store only on one gateway  TODO: add more
	pub price: Price,      // price 0 not for sale
	pub gateway: Vec<u8>,  // gateway which pins the file (as well as the metadata?)
	pub meta_hash: Vec<u8>, // or https://github.com/multiformats/multiaddr
	                       // ERC721 Metadata JSON Schema
	                       // https://medium.com/blockchain-manchester/erc-721-metadata-standards-and-ipfs-94b01fea2a89
	                       // Contains tag, text, title, filetype as Vec<u8>?
	                       // searchable?
	                       // because storing an additional metadata hash on IPFS is probably too slow
	                       // unless you load metadata via the location based system!
	                       // https://github.com/mit-pdos/noria
}

decl_storage! {
	trait Store for Module<T: Trait> as MetadataStorage {
		//TODO: Is there a better solution for query for metadata without subscription
		MetadataArray get(metadata_by_index): map u64 => Vec<u8>;

		// TODO: what happens if multipe transactions at the same time?
		// integrate some way to make things parallel, see riot
		// idea income first block and assign count during second block
		MetadataCount get(metadata_count): u64;
		MetadataIndex: map Vec<u8> => u64;

		MetaHash get(metahash): map Vec<u8> => Vec<u8>;

		OwnedMetaArray get(metadata_of_user_by_index): map (T::AccountId, u64) => Metalog<T::Moment, T::Balance>;
		OwnedMetaCount get(user_meta_count): map T::AccountId => u64;
		OwnedMetaIndex: map Vec<u8> => u64;

		FileHashMeta get(meta_for_hash): map Vec<u8> => Metalog<T::Moment, T::Balance>;
		FileHashOwner get(owner_of_hash): map Vec<u8> => Option<T::AccountId>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// Store metadata
		pub fn store_meta(origin,
			file_hash: Vec<u8>,
			license_code: u16,
			price: T::Balance,
			gateway: Vec<u8>,
			meta_hash: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			metadata_checks::check_valid_hash(&file_hash)?;
			metadata_checks::check_valid_hash(&meta_hash)?;
			metadata_checks::check_valid_gateway(&gateway)?;

			ensure!(file_hash != meta_hash, ERR)
			ensure!(!<FileHashOwner<T>>::exists(&file_hash), ERR_ALREADY_CLAIMED);

			if license_code == 1 {
				ensure!(price.is_zero(), ERR_PRICE_NOT_ZERO);
			}

			//Initial upload can't have the delete license_code
			ensure!(license_code != 2, ERR_INVALID_LICENSE);

			//if doesn't exist create metadata entry
			if !<MetaHash<T>>::exists(&meta_hash) {
				<MetaHash<T>>::insert(&meta_hash, &meta_hash);
			}

			let time = <timestamp::Module<T>>::now();
			let new_metadata = Metalog {
				file_hash,
				time,
				license_code,
				price,
				gateway,
				meta_hash,
			};

			Self::_user_store(sender.clone(), new_metadata.clone())?;
			Self::deposit_event(RawEvent::Stored(sender, new_metadata.time, new_metadata.file_hash));
			Ok(())
		}

		//TODO: claim ownership?

		/// Transfer the ownership without paying for it
		fn transfer_ownership(origin, receiver: T::AccountId, file_hash: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_ownership_rights(sender.clone(), &file_hash)?;
			Self::_transfer(sender.clone(), receiver.clone(), &file_hash)?;

			Self::deposit_event(RawEvent::TransferOwnership(sender, receiver, file_hash));
			Ok(())
		}

		/// Change gateway
		pub fn change_gateway(origin, file_hash: Vec<u8>, gateway: Vec<u8>)-> Result{
			let sender = ensure_signed(origin)?;
			metadata_checks::check_valid_gateway(&gateway)?;
			Self::_check_ownership_rights(sender.clone(), &file_hash)?;
			let mut metadata = Self::meta_for_hash(&file_hash);
			metadata.gateway = gateway.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&file_hash);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<FileHashMeta<T>>::insert(&file_hash, &metadata);

			Self::deposit_event(RawEvent::GatewaySet(sender, file_hash, gateway));
			Ok(())
		}

		/// Change the price
		fn change_price(origin, file_hash: Vec<u8>, price: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_ownership_rights(sender.clone(), &file_hash)?;

			let mut metadata = Self::meta_for_hash(&file_hash);
			//once open source = forever open source
			ensure!(metadata.license_code != 1, ERR_PRICE_NOT_ZERO);

			metadata.price = price;

			let meta_index = <OwnedMetaIndex<T>>::get(&file_hash);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<FileHashMeta<T>>::insert(&file_hash, metadata);

			Self::deposit_event(RawEvent::PriceSet(sender, file_hash, price));
			Ok(())
		}

		//TODO: change metadata?

		fn buy_meta(origin, file_hash: Vec<u8>, new_owner: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(<FileHashMeta<T>>::exists(&file_hash), ERR_HASH_NOT_EXIST);

			let owner = Self::owner_of_hash(&file_hash).ok_or(ERR_NO_OWNER)?;
			ensure!(owner != sender, ERR_ALREADY_OWNER );

			let metadata = Self::meta_for_hash(&file_hash);
			let price = metadata.price;
			ensure!(!price.is_zero(), ERR_NOT_FOR_SALE);
			 <balances::Module<T>>::make_transfer(&sender, &owner, price)?;

			Self::_transfer(owner.clone(), new_owner, &file_hash)?;

			Self::deposit_event(RawEvent::Bought(sender, owner, price, file_hash));
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
		TransferOwnership(AccountId, AccountId, Vec<u8>),
		GatewaySet(AccountId, Vec<u8>,Vec<u8>),
		PriceSet(AccountId,Vec<u8>, Balance),
		Bought(AccountId, AccountId, Balance, Vec<u8>),
    }
);

impl<T: Trait> Module<T> {
	/// Checks the ownership rights, no need for an additional hash check
	fn _check_ownership_rights(sender: T::AccountId, file_hash: &Vec<u8>) -> Result {
		ensure!(<FileHashMeta<T>>::exists(file_hash), ERR_HASH_NOT_EXIST);
		let owner = Self::owner_of_hash(file_hash).ok_or(ERR_NO_OWNER)?;
		ensure!(owner == sender, ERR_NOT_OWNER);

		Ok(())
	}

	fn _transfer(sender: T::AccountId, receiver: T::AccountId, file_hash: &Vec<u8>) -> Result {
		let receiver_total_count = Self::user_meta_count(&receiver);
		let new_receiver_count = receiver_total_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let sender_total_count = Self::user_meta_count(&sender);
		let new_sender_count = sender_total_count.checked_sub(1).ok_or(ERR_UNDERFLOW)?;

		let meta_index = <OwnedMetaIndex<T>>::get(file_hash);
		let meta_object = <OwnedMetaArray<T>>::get((sender.clone(), new_sender_count));

		if meta_index != new_sender_count {
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index), &meta_object);
			<OwnedMetaIndex<T>>::insert(&meta_object.file_hash, meta_index);
		}

		<FileHashOwner<T>>::insert(file_hash, &receiver);

		<OwnedMetaIndex<T>>::insert(file_hash, receiver_total_count);

		<OwnedMetaArray<T>>::remove((sender.clone(), new_sender_count));
		<OwnedMetaArray<T>>::insert((receiver.clone(), receiver_total_count), meta_object);

		<OwnedMetaCount<T>>::insert(&sender, new_sender_count);
		<OwnedMetaCount<T>>::insert(&receiver, new_receiver_count);

		Ok(())
	}

	fn _user_store(user: T::AccountId, metadata: Metalog<T::Moment, T::Balance>) -> Result {
		let count = Self::user_meta_count(&user);
		let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let metadata_count = Self::metadata_count();
		let new_metadata_count = metadata_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		<MetadataArray<T>>::insert(metadata_count, &metadata.file_hash);
		<MetadataCount<T>>::put(new_metadata_count);
		<MetadataIndex<T>>::insert(&metadata.file_hash, metadata_count);

		<OwnedMetaArray<T>>::insert((user.clone(), count), &metadata);
		<OwnedMetaCount<T>>::insert(&user, updated_count);
		<OwnedMetaIndex<T>>::insert(&metadata.file_hash, updated_count);

		<FileHashMeta<T>>::insert(&metadata.file_hash, &metadata);
		<FileHashOwner<T>>::insert(&metadata.file_hash, &user);

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
			let hash = vec![
				81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
				76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
				111, 110, 120, 70, 121, 100, 121,
			];
			let gateway = vec![105, 112, 102, 115, 46, 105, 111];
			assert_ok!(MetadataStorage::store_meta(
				Origin::signed(20),
				hash.clone(),
				0,
				4200,
				gateway,
				hash.clone(),
			));
			assert_eq!(MetadataStorage::owner_of_hash(hash), Some(20));
		});
	}

	#[test]
	fn change_price_works() {
		with_externalities(&mut new_test_ext(), || {
			let hash = vec![
				81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
				76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
				111, 110, 120, 70, 121, 100, 121,
			];
			let gateway = vec![105, 112, 102, 115, 46, 105, 111];
			assert_ok!(MetadataStorage::store_meta(
				Origin::signed(20),
				hash.clone(),
				0,
				4200,
				gateway,
				hash.clone(),
			));
			assert_ok!(MetadataStorage::change_price(
				Origin::signed(20),
				hash,
				5555
			));
		});
	}
}
