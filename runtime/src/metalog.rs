//! Metalog runtime, see https://github.com/PACTCare/Stars-Network/blob/master/WHITEPAPER.md#--starlog--substrate-
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageValue, StorageMap, traits::Currency};
use parity_codec::{Encode, Decode};
use system::ensure_signed;
use runtime_primitives::traits::{As};
use rstd::vec::Vec;

const ERR_DID_ALREADY_CLAIMED: &str = "This DID has already been claimed.";
const ERR_DID_NOT_EXIST: &str = "This DID does not exist";
const ERR_DID_NO_OWNER: &str = "No one owens this did";

const ERR_UN_ALREADY_CLAIMED: &str = "This unique name has already been claimed.";

const ERR_LICENSE_INVALID: &str = "Invalid license code";

const ERR_OVERFLOW: &str = "Overflow adding new metadata";
const ERR_UNDERFLOW: &str = "Underflow removing metadata";

const ERR_NOT_OWNER: &str = "You are not the owner";

const ERR_OPEN_NAME_ACCOUNT_CLAIMED: &str = "Unique name account already claimed";

const ERR_BYTEARRAY_LIMIT: &str = "Bytearray is too large";

const BYTEARRAY_LIMIT_DID: usize = 80;
const BYTEARRAY_LIMIT_LOCATION: usize = 80;
const BYTEARRAY_LIMIT_NAME: usize = 40;

const DELETE_LICENSE: u16 = 1;

/// The module's configuration traits are timestamp and balance
pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Key metalog struct
//TODO: Vec<u8> max length?
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Metalog<Time> {
	pub did: Vec<u8>, //= primary key, can't be changed
	pub unique_name: Vec<u8>, // Default = 0
	pub license_code: u16, // 0 = no license code, 1 = delete request     
	pub storage_location: Vec<u8>,  
	pub time: Time,
}

decl_storage! {
	trait Store for Module<T: Trait> as Metalog {
		/// Array of personal owned metalog data
		OwnedMetaArray get(metadata_of_owner_by_index): map (T::AccountId, u64) => Metalog<T::Moment>;

		/// Number of stored metalogs per account 
		OwnedMetaCount get(owner_meta_count): map T::AccountId => u64;

		/// Index of DID
		OwnedMetaIndex: map Vec<u8> => u64;

		/// Query for unique names
		UnMeta get(meta_of_un): map Vec<u8> => Metalog<T::Moment>;
		UnOwner get(owner_of_un): map Vec<u8> => Option<T::AccountId>;

		/// Query by DIDs 
		DidMeta get(meta_of_did): map Vec<u8> => Metalog<T::Moment>;
		DidOwner get(owner_of_did): map Vec<u8> => Option<T::AccountId>;

		/// Account which gets all the money for the unique name
		UniqueNameAccount get(unique_name_account): T::AccountId;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// For deposit events
		fn deposit_event<T>() = default;

		/// Initialize unique name account
		pub fn init_unique_name_account(origin) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(!<UniqueNameAccount<T>>::exists(), ERR_OPEN_NAME_ACCOUNT_CLAIMED);
			<UniqueNameAccount<T>>::put(sender);
			Ok(())
		}

		/// Store initial metalog
        fn create_metalog(
			origin, 
			did: Vec<u8>, 
			license_code: u16, 
			storage_location: Vec<u8>) -> Result {

            let sender = ensure_signed(origin)?;

			ensure!(did.len() <= BYTEARRAY_LIMIT_DID, ERR_BYTEARRAY_LIMIT);
			ensure!(storage_location.len() <= BYTEARRAY_LIMIT_LOCATION, ERR_BYTEARRAY_LIMIT);

			ensure!(!<DidOwner<T>>::exists(&did), ERR_DID_ALREADY_CLAIMED);
			
			ensure!(license_code != DELETE_LICENSE, ERR_LICENSE_INVALID);

			let time = <timestamp::Module<T>>::now();

			let mut default_name = Vec::new();
			default_name.push(0);
			let new_metadata = Metalog {
				did,
				unique_name: default_name, 
				license_code,
				storage_location,
				time,
			};

			Self::_owner_store(sender.clone(), new_metadata.clone())?;
			Self::deposit_event(RawEvent::Stored(sender, new_metadata.time, new_metadata.did));
            Ok(())
        }

		/// Transfer the ownership, Payment will be implemented in smart contracts
		fn transfer_ownership(origin, receiver: T::AccountId, did: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_did_ownership(sender.clone(), &did)?;
			Self::_transfer(sender.clone(), receiver.clone(), &did)?;

			Self::deposit_event(RawEvent::TransferOwnership(sender, receiver, did));
			Ok(())
		}

		/// Buy a unique name
		pub fn buy_unique_name(origin, did: Vec<u8>, unique_name: Vec<u8>)-> Result{
			let sender = ensure_signed(origin)?;

			Self::_check_did_ownership(sender.clone(), &did)?;

			ensure!(did.len() <= BYTEARRAY_LIMIT_NAME, ERR_BYTEARRAY_LIMIT);

			ensure!(!<UnOwner<T>>::exists(&unique_name), ERR_UN_ALREADY_CLAIMED);
			Self::_pay_name(sender.clone())?;

			let mut metalog = Self::meta_of_did(&did);
			metalog.unique_name = unique_name.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metalog);
			<DidMeta<T>>::insert(&did, &metalog);

			<UnMeta<T>>::insert(&metalog.unique_name, &metalog);
			<UnOwner<T>>::insert(&metalog.unique_name, &sender);
			
			Self::deposit_event(RawEvent::NameUpdated(sender, did, unique_name));
			Ok(())
		}

		/// Change license code
		pub fn change_license_code(origin, did: Vec<u8>, license_code: u16)-> Result{
			let sender = ensure_signed(origin)?;

			Self::_check_did_ownership(sender.clone(), &did)?;
			let mut metadata = Self::meta_of_did(&did);
			metadata.license_code = license_code.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<DidMeta<T>>::insert(&did, &metadata);

			Self::deposit_event(RawEvent::LicenseUpdated(sender, did, license_code));
			Ok(())
		}

		/// Change storage location
		pub fn change_storage_location(origin, did: Vec<u8>, storage_location: Vec<u8>)-> Result{
			let sender = ensure_signed(origin)?;

			ensure!(did.len() <= BYTEARRAY_LIMIT_LOCATION, ERR_BYTEARRAY_LIMIT);

			Self::_check_did_ownership(sender.clone(), &did)?;
			let mut metadata = Self::meta_of_did(&did);
			metadata.storage_location = storage_location.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<DidMeta<T>>::insert(&did, &metadata);

			Self::deposit_event(RawEvent::LocationUpdated(sender, did, storage_location));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as timestamp::Trait>::Moment, 
    {
        Stored(AccountId, Moment, Vec<u8>),
		TransferOwnership(AccountId, AccountId, Vec<u8>),
		LicenseUpdated(AccountId, Vec<u8>,u16),
		LocationUpdated(AccountId, Vec<u8>,Vec<u8>),
		NameUpdated(AccountId, Vec<u8>,Vec<u8>),
	}
);

impl<T: Trait> Module<T> { 
	/// store metalog
	fn _owner_store(sender: T::AccountId, metalog: Metalog<T::Moment>) -> Result {
		let count = Self::owner_meta_count(&sender);
		let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		<OwnedMetaArray<T>>::insert((sender.clone(), count), &metalog);
		<OwnedMetaCount<T>>::insert(&sender, updated_count);
		<OwnedMetaIndex<T>>::insert(&metalog.did, updated_count);

		<DidMeta<T>>::insert(&metalog.did, &metalog);
		<DidOwner<T>>::insert(&metalog.did, &sender);

		Ok(())
	}

	/// Checks the ownership rights
	fn _check_did_ownership(sender: T::AccountId, did: &Vec<u8>) -> Result {
		ensure!(<DidMeta<T>>::exists(did), ERR_DID_NOT_EXIST);
		let owner = Self::owner_of_did(did).ok_or(ERR_DID_NO_OWNER)?;
		ensure!(owner == sender, ERR_NOT_OWNER);

		Ok(())
	}

	/// Transfer ownership
	fn _transfer(sender: T::AccountId, receiver: T::AccountId, did: &Vec<u8>) -> Result {
		let receiver_total_count = Self::owner_meta_count(&receiver);
		let new_receiver_count = receiver_total_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let sender_total_count = Self::owner_meta_count(&sender);
		let new_sender_count = sender_total_count.checked_sub(1).ok_or(ERR_UNDERFLOW)?;

		let meta_index = <OwnedMetaIndex<T>>::get(did);
		let meta_object = <OwnedMetaArray<T>>::get((sender.clone(), new_sender_count));

		if meta_index != new_sender_count {
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index), &meta_object);
			<OwnedMetaIndex<T>>::insert(&meta_object.did, meta_index);
		}

		// if un is not the default un
		let mut default_name = Vec::new();
		default_name.push(0);
		if meta_object.unique_name != default_name {
			<UnOwner<T>>::insert(did, &receiver);
		}

		<DidOwner<T>>::insert(did, &receiver);

		<OwnedMetaIndex<T>>::insert(did, receiver_total_count);

		<OwnedMetaArray<T>>::remove((sender.clone(), new_sender_count));
		<OwnedMetaArray<T>>::insert((receiver.clone(), receiver_total_count), meta_object);

		<OwnedMetaCount<T>>::insert(&sender, new_sender_count);
		<OwnedMetaCount<T>>::insert(&receiver, new_receiver_count);

		Ok(())
	}

	/// Payment for unique name
	fn _pay_name(sender: T::AccountId)-> Result{
		let price = <T::Balance as As<u64>>::sa(1000);
		let name_account = Self::unique_name_account();

		// transfer() function verifies and writes
		<balances::Module<T> as Currency<_>>::transfer(&sender, &name_account, price)?;

		Ok(())
	}
}

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

	type Metalog = Module<Test>;

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap()
			.0
			.into()
	}

	#[test]
	fn create_metalog_works() {
		with_externalities(&mut new_test_ext(), || {
			let did = vec![
				81, 109, 97, 71, 54, 103, 67, 80, 72, 66, 75, 69, 118, 81, 116, 67, 84, 71, 55, 69,
				76, 97, 49, 74, 49, 102, 104, 57, 75, 55, 105, 105, 116, 99, 67, 119, 114, 87, 112,
				111, 110, 120, 70, 121, 100, 121,
			];
			let storage_location = vec![105, 112, 102, 115, 46, 105, 111];
			assert_ok!(Metalog::create_metalog(
				Origin::signed(20),
				did.clone(),
				did.clone(),
				0,
				did.clone(),
			));
			assert_eq!(Metalog::owner_of_did(did), Some(20));
		});
	}
}

