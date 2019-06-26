//! # Federation Module
//!
//!	The Federation module implements the governance system in form of a Layered TCR.
//!
//! For more information see https://github.com/PACTCare/Stars-Network/blob/master/WHITEPAPER.md#governance

//TODO: 
// slashing
// overflow

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

const ERR_VOTE_MIN_STAKE: &str = "To vote you need to stake at least the minimum amount of tokens";
const ERR_VOTE_MIN_LOCK: &str = "To vote you need to lock at least for one week";
const ERR_VOTE_LOCK: &str = "The funds are still locked";
const ERR_VOTE_RANK: &str = "The intended rank of the candidate needs to be higher than the guest rank.";
const ERR_VOTE_EXIST: &str = "To cancel a vote, you need to have voted for the specific account";

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
pub struct Candidate {       
    pub current_rank: u16, 
	pub intended_rank: u16, // Same Rank Means Nothing to vote
	pub votes: u64, 
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Vote<Account, Balance, BlockNumber>{
	pub account: Account,
	pub stake: Balance, 
	pub block_number: BlockNumber,
	pub lock_time: BlockNumber,
}

decl_storage! {
	trait Store for Module<T: Trait> as FederationModule {
		/// Query by candidate
        CandidateStore get(candidate_by_account): map T::AccountId => Candidate;

		/// Array of personal votes
        VoteArray get(votes_of_owner_by_index): map (T::AccountId, u64) => Vote<T::AccountId, T::Balance, T::BlockNumber>;

        /// Total count of votes of a user
        VoteCount get(vote_count): map T::AccountId => u64;

        /// Index of specific (user, voted account) combination
        VoteIndex get(vote_index): map (T::AccountId, T::AccountId) => u64;

		// parameters 
		/// Minimum stake requirements for admirals
		pub AdmiralStake get(admiral_stake) config(): u64 = 5000;
		/// Minimum stake requirements for sections31	
		pub Section31Stake get(section31_stake) config(): u64 = 4000;
		/// Minimum stake requirements for captains		
		pub CaptainStake get(captain_stake) config(): u64 = 3000;
		/// Minimum stake requirements for engineers		
		pub EngineerStake get(engineer_stake) config(): u64 = 2000;
		/// Minimum stake requirements for crew members		
		pub CrewStake get(crew_stake) config(): u64 = 1000;

		/// Minimum stake
		pub MinStake get(min_stake) config(): u64 = 100;

		/// Minimum lock up time, one week with 6 seconds blocktime
		pub MinLockTime get(min_lock) config(): u64 = 100800;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// Change own rank
		/// Account must have enough transferrable funds in it to pay the stake.
        fn change_rank(origin, intended_rank: u16) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(intended_rank <= ADMIRAL_RANK, ERR_RANK_LOWER);
			let mut candidate = Self::candidate_by_account(&sender);
			candidate.intended_rank = intended_rank;

			let rank = Self::_return_updated_rank(&intended_rank, &candidate.votes);
			candidate.current_rank = rank;

			<CandidateStore<T>>::insert(sender.clone(), &candidate);
			Self::deposit_event(RawEvent::CandidateStored(sender, intended_rank));
			Ok(())
		}

		/// Vote for a candidate
		fn vote(origin, candidate_vote: T::AccountId, stake: T::Balance, lock_time: T::BlockNumber) -> Result {
			let sender = ensure_signed(origin)?;
			let mut candidate = Self::candidate_by_account(&candidate_vote);	
			// candidate exist
			ensure!(candidate.intended_rank > GUEST_RANK, ERR_VOTE_RANK);
			// vote needs to be above 0
			ensure!(stake >= T::Balance::sa(Self::min_stake()), ERR_VOTE_MIN_STAKE);
			ensure!(lock_time >= T::BlockNumber::sa(Self::min_lock()), ERR_VOTE_MIN_LOCK);

			let block_number = <system::Module<T>>::block_number();
			let vote = Vote {
                account: candidate_vote.clone(),
				stake,
				block_number,
				lock_time,
            };

			let count = Self::vote_count(&sender);
			let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

			Self::_stake(&sender, stake.clone())?;
			
			//store vote
			let vote_index = Self::vote_index((sender.clone(), candidate_vote.clone()));
			//if voted before, else new vote
			if vote_index > 0{
				//update vote
				let mut old_vote = Self::votes_of_owner_by_index((sender.clone(), vote_index.clone()));
				old_vote.stake += vote.stake;
				<VoteArray<T>>::insert((sender.clone(), vote_index), &old_vote); 
			} else {
				//store new vote, starts with one
				<VoteArray<T>>::insert((sender.clone(), updated_count), &vote);
				<VoteIndex<T>>::insert((sender.clone(),candidate_vote.clone()), updated_count);
				<VoteCount<T>>::insert(&sender, updated_count);
			}
			//update candidate
			candidate.votes += Self::_calculate_voting_power(stake, lock_time);
			let rank = Self::_return_updated_rank(&candidate.intended_rank, &candidate.votes);
			candidate.current_rank = rank;
			<CandidateStore<T>>::insert(candidate_vote.clone(), &candidate);
			Self::deposit_event(RawEvent::Voted(candidate_vote, stake));
			Ok(())
		}

		/// Cancel vote for specific account and collect funds
		fn cancel_vote(origin, candidate_vote: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			let vote_index = Self::vote_index((sender.clone(), candidate_vote.clone())); //0
			let old_vote = Self::votes_of_owner_by_index((sender.clone(), vote_index));
			ensure!(old_vote.stake >=  T::Balance::sa(Self::min_stake()), ERR_VOTE_EXIST);

			Self::_unstake(&sender, &old_vote)?;
			<VoteArray<T>>::remove((sender.clone(), vote_index)); 
			<VoteIndex<T>>::remove((sender.clone(), candidate_vote.clone()));
			
			let mut candidate = Self::candidate_by_account(&candidate_vote);
			candidate.votes -= Self::_calculate_voting_power(old_vote.stake, old_vote.lock_time);;
			let rank = Self::_return_updated_rank(&candidate.intended_rank, &candidate.votes);
			candidate.current_rank = rank;
			<CandidateStore<T>>::insert(&candidate_vote, &candidate);

			Self::deposit_event(RawEvent::Voted(candidate_vote, old_vote.stake));
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
	fn _stake(sender: &T::AccountId, stake: T::Balance) -> Result{
		let _ = <balances::Module<T> as Currency<_>>::withdraw(
            sender,
            stake,
            WithdrawReason::Reserve,
            ExistenceRequirement::KeepAlive,
        )?;
        Ok(())
	}

	fn _calculate_voting_power(stake: T::Balance, lock_time: T::BlockNumber) -> u64 {
		let voting_power = stake.as_()/&Self::min_stake() * lock_time.as_()/&Self::min_lock() * lock_time.as_()/&Self::min_lock();
		voting_power
	}

	fn _unstake(sender: &T::AccountId, old_vote: &Vote<T::AccountId, T::Balance, T::BlockNumber>) -> Result{
		let block_number = <system::Module<T>>::block_number();
		let block_dif = block_number - old_vote.block_number;
		ensure!(block_dif > old_vote.lock_time, ERR_VOTE_LOCK);
		// 10% income per year with 1 Block per 6 seconds  
		let earned_money = (T::Balance::sa(block_dif.as_() * old_vote.stake.as_() * 195069/10000000000000)) + old_vote.stake;
		let _ = <balances::Module<T> as Currency<_>>::deposit_into_existing(sender, earned_money)?;
		Ok(())
	}

	/// Returns the updated rank
	fn _return_updated_rank(intended_rank: &u16, total_votes: &u64) -> u16{
		let mut rank = GUEST_RANK;

		if total_votes > &Self::admiral_stake() && intended_rank == &ADMIRAL_RANK {
			rank = ADMIRAL_RANK;
		} else if total_votes > &Self::section31_stake() && intended_rank == &SECTION31_RANK {
			rank = SECTION31_RANK;
		} else if total_votes > &Self::captain_stake() && intended_rank == &CAPTAIN_RANK {
			rank = CAPTAIN_RANK;
		} else if total_votes > &Self::engineer_stake() && intended_rank == &ENGINEER_RANK {
			rank = ENGINEER_RANK;
		} else if total_votes > &Self::crew_stake() && intended_rank == &CREW_RANK {
			rank = CREW_RANK;
		} 

		rank
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

	const ERR_BALANCE_LOW: &str = "too few free funds in account";

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
	type System = system::Module<Test>;
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
			let intended_rank: u16 = 5;
			let stake: u64 = 7001;
			let lock: u64 = 1000000;

			assert_noop!(
                FederationModule::vote(Origin::signed(1), 1, stake.clone(), lock.clone()),
                ERR_VOTE_RANK
            );
			let _ = FederationModule::change_rank(Origin::signed(candidate_to_vote.clone()), intended_rank.clone());
			assert_noop!(
                FederationModule::vote(Origin::signed(1), candidate_to_vote.clone(), stake.clone(), lock.clone()),
                ERR_BALANCE_LOW
            );
			let _ = Balances::make_free_balance_be(&voter, 200000);
			assert_noop!(
                FederationModule::vote(Origin::signed(voter.clone()), candidate_to_vote.clone(), 5, lock.clone()),
                ERR_VOTE_MIN_STAKE
            );
			assert_noop!(
                FederationModule::vote(Origin::signed(voter.clone()), candidate_to_vote.clone(), stake.clone(), 5),
                ERR_VOTE_MIN_LOCK
            );
			assert_ok!(FederationModule::vote(Origin::signed(voter.clone()), candidate_to_vote.clone(), stake.clone(), lock.clone()));
			let candidate = FederationModule::candidate_by_account(candidate_to_vote.clone());
			assert_eq!(candidate.votes, 6884);
			assert_eq!(candidate.current_rank, intended_rank);
			let vote = FederationModule::votes_of_owner_by_index((voter.clone(), 1));
			assert_eq!(vote.stake, stake);
		});
	}

	#[test]
	fn cancel_vote_works() {
		with_externalities(&mut new_test_ext(), || {
			let candidate_to_vote: u64 = 2;
			let voter: u64 = 0;

			assert_noop!(FederationModule::cancel_vote(Origin::signed(voter.clone()), candidate_to_vote.clone()), ERR_VOTE_EXIST);
			let _ = Balances::make_free_balance_be(&voter, 2000);
			let _ = FederationModule::change_rank(Origin::signed(candidate_to_vote.clone()), 2);
			assert_ok!(FederationModule::vote(Origin::signed(voter.clone()), candidate_to_vote.clone(), 1000, 200000));
			// 5126400 Blocks per year -> 10 % income per year
			assert_noop!(FederationModule::cancel_vote(Origin::signed(voter.clone()), candidate_to_vote.clone()), ERR_VOTE_LOCK);			
			System::set_block_number(5126401);
			assert_ok!(FederationModule::cancel_vote(Origin::signed(voter.clone()), candidate_to_vote.clone()));
			let free_balance = Balances::free_balance(voter.clone());
			let candidate = FederationModule::candidate_by_account(&candidate_to_vote);
			assert_eq!(candidate.votes, 0);
			assert_eq!(free_balance, 2100);
		});
	}
}