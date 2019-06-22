//! # Federation Module
//!
//!	The Federation module implements the governance system in form of a Layered TCR.
//!
//! For more information see https://github.com/PACTCare/Stars-Network/blob/master/WHITEPAPER.md#governance

//TODO: refactor
// remove clones

use support::{decl_module, 
	decl_storage, 
	decl_event, 
	StorageMap, 
	ensure,
	traits::{Currency, ExistenceRequirement, WithdrawReason}, 
	dispatch::Result};
use runtime_primitives::traits::As;
use parity_codec::{Decode, Encode};
use system::ensure_signed;

const ERR_RANK_LOWER: &str = "The intended rank needs to be lower than the maximum rank";

const ERR_VOTE_RANK: &str = "The intended rank of the candidate needs to be higher than the guest rank.";
const ERR_VOTE_EXIST: &str = "To cancel a vote, you need to have voted for the specific account";

const ERR_BALANCE_LOW: &str = "too few free funds in account";

const ERR_OVERFLOW: &str = "Overflow adding new candidate";

const ADMIRAL_RANK: u16 = 5;
const SECTION31_RANK: u16 = 4;
const CAPTAIN_RANK: u16 = 3;
const ENGINEER_RANK: u16 = 2;
const CREW_RANK: u16 = 1;
const GUEST_RANK: u16 = 0;

/// The module's configuration trait.
pub trait Trait: system::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Candidate<Balance> {       
    pub current_rank: u16, // 
	pub intended_rank: u16, // Same Rank Means Nothing to vote
	pub stake: Balance, // https://www.youtube.com/watch?time_continue=3&v=B6f8uGNUSQQ 
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vote<Account, Balance>{
	pub account: Account,
	pub stake: Balance, 
}

decl_storage! {
	trait Store for Module<T: Trait> as FederationModule {
		/// Query by candidate
        CandidateStore get(candidate_by_account): map T::AccountId => Candidate<T::Balance>;

		/// Array of personal votes
        VoteArray get(votes_of_owner_by_index): map (T::AccountId, u64) => Vote<T::AccountId, T::Balance>;

        /// Total count of votes of a user
        VoteCount get(vote_count): map T::AccountId => u64;

        /// Index of specific (user, voted account) combination
        VoteIndex get(vote_index): map (T::AccountId, T::AccountId) => u64;

		// TotalCandidateStake get(totalStake): T::Balance;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// Change own rank
		/// Account must have enough transferrable funds in it to pay the stake.
        fn change_rank(origin, intended_rank: u16) -> Result {
			// TODO: automatic update rank if specific stake
			let sender = ensure_signed(origin)?;
			ensure!(intended_rank <= ADMIRAL_RANK, ERR_RANK_LOWER);
			let mut candidate = Self::candidate_by_account(&sender);
			candidate.intended_rank = intended_rank;
			<CandidateStore<T>>::insert(sender.clone(), &candidate);
			Self::deposit_event(RawEvent::CandidateStored(sender, intended_rank));
			Ok(())
		}

		/// Vote for a candidate
		fn vote(origin, candidate_vote: T::AccountId, stake: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;
			let mut candidate = Self::candidate_by_account(&candidate_vote);
			
			let vote = Vote {
                account: candidate_vote.clone(),
				stake,
            };
			// candidate exist
			ensure!(candidate.intended_rank > GUEST_RANK, ERR_VOTE_RANK);
			
			let count = Self::vote_count(&sender);
			let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

			Self::_stake(sender.clone(), stake.clone())?;
			
			//store vote
			let vote_index = Self::vote_index((sender.clone(), candidate_vote.clone()));
			//if voted before, else new vote
			if vote_index > 0{
				//update vote
				let mut old_vote = Self::votes_of_owner_by_index((sender.clone(), vote_index.clone()));
				old_vote.stake += vote.stake;
				<VoteArray<T>>::insert((sender.clone(), vote_index), &old_vote);
			} else {
				//store new vote
				<VoteArray<T>>::insert((sender.clone(), count), &vote);
				<VoteCount<T>>::insert(&sender, updated_count);
				<VoteIndex<T>>::insert((sender,candidate_vote.clone()), updated_count);
			}
			//update candidate
			candidate.stake = stake;
			<CandidateStore<T>>::insert(candidate_vote.clone(), &candidate);
			Self::deposit_event(RawEvent::Voted(candidate_vote, stake));
			Ok(())
		}

		/// Cancel vote for specific account and collect funds
		fn cancel_vote(origin, candidate_vote: T::AccountId) -> Result {
			//TODO: remove stake + own vote
			let sender = ensure_signed(origin)?;
			let vote_index = Self::vote_index((sender.clone(), candidate_vote.clone()));
			let mut old_vote = Self::votes_of_owner_by_index((sender.clone(), vote_index));

			ensure!(old_vote.stake >  T::Balance::sa(0), ERR_VOTE_EXIST);

			// TODO: return own money_earned and 
			let money_earned = old_vote.stake;
			Self::deposit_event(RawEvent::Voted(candidate_vote, money_earned));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where 
	<T as system::Trait>::AccountId, 
	<T as balances::Trait>::Balance 
	{
		CandidateStored(AccountId, u16),
		Voted(AccountId, Balance),
		CancelVote(AccountId, Balance),
	}
);

impl<T: Trait> Module<T> {
	//TODO: needs to depend on the blocknumber (put into storage)
	fn _stake(sender: T::AccountId, stake: T::Balance) -> Result{
		let _ = <balances::Module<T> as Currency<_>>::withdraw(
            &sender,
            stake,
            WithdrawReason::Reserve,
            ExistenceRequirement::KeepAlive,
        )?;
        Ok(())
	}

	// if rank specific requirement fulfilled -> update rank
	fn _check_rank_requirements(rank: u16, stake: T::Balance) -> Result{

		Ok(())
	}
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_noop, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

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
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}

	impl balances::Trait for Test {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransactionPayment = ();
        type DustRemoval = ();
        type TransferPayment = ();
    }

	impl Trait for Test {
		type Event = ();
	}

	type Balances = balances::Module<Test>;
	type FederationModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}


	#[test]
	fn change_rank_works() {
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(FederationModule::change_rank(Origin::signed(0), 7), ERR_RANK_LOWER);
			assert_ok!(FederationModule::change_rank(Origin::signed(0), 2));
			let candidate = FederationModule::candidate_by_account(&0);
			assert_eq!(candidate.intended_rank, 2);
		});
	}

	#[test]
	fn vote_works() {
		with_externalities(&mut new_test_ext(), || {
			let candidate_to_vote: u64 = 2;
			let voter: u64 = 0;
			assert_noop!(
                FederationModule::vote(Origin::signed(1), 1, 10),
                ERR_VOTE_RANK
            );
			FederationModule::change_rank(Origin::signed(candidate_to_vote.clone()), 1);
			assert_noop!(
                FederationModule::vote(Origin::signed(1), candidate_to_vote.clone(), 10),
                ERR_BALANCE_LOW
            );
			let _ = Balances::make_free_balance_be(&voter, 500000);
			assert_ok!(FederationModule::vote(Origin::signed(voter.clone()), candidate_to_vote.clone(), 10));
			let candidate = FederationModule::candidate_by_account(candidate_to_vote.clone());
			assert_eq!(candidate.stake, 10);
			let vote = FederationModule::votes_of_owner_by_index((voter.clone(), 0));
			assert_eq!(vote.stake, 10);
		});
	}

	#[test]
	fn cancel_vote_works() {
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(FederationModule::cancel_vote(Origin::signed(0), 1), ERR_VOTE_EXIST);
		});
	}
}