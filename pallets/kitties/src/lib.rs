//! # Kitties Pallet

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Currency, Randomness, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded};

	#[derive(Encode, Decode)]
	pub struct Kitty(pub [u8; 16]);

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// 事件
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// 随机数模块
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		/// Kitty 编号
		type KittyIndex: Parameter + AtLeast32BitUnsigned + Default + Copy + Bounded;
		/// 创建 Kitty 时需要质押的金额
		type ReserveOfNewCreate: Get<BalanceOf<Self>>;
		/// 余额模块
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Kitties 总数
	#[pallet::storage]
	#[pallet::getter(fn kitties_count)]
	pub type KittiesCount<T: Config> = StorageValue<_, T::KittyIndex>;

	/// Kitties 价格表
	#[pallet::storage]
	#[pallet::getter(fn kitties_price)]
	pub type KittiesPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<BalanceOf<T>>, ValueQuery>;

	/// Kitties
	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T: Config> =
		StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<Kitty>, ValueQuery>;

	/// Kitties 的主人
	#[pallet::storage]
	#[pallet::getter(fn owner)]
	pub type Owner<T: Config> =
		StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<T::AccountId>, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// 创建成功
		KittyCreated(T::AccountId, T::KittyIndex),
		/// 转让成功
		KittyTransfered(T::AccountId, T::AccountId, T::KittyIndex),
		/// 发起出售
		KittyForSale(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Kitties 数量达到上限
		KittiesCountOverflow,
		/// Kitty 编号不存在
		InvalidKittyIndex,
		/// 当前用户不是 Kitty 的主人
		NotOwnerOfKitty,
		/// 父母的编号不能相同
		SameParentIndex,
		/// Kitty 暂未出售
		NotForSale,
		/// 余额不足
		NotEnoughBalance,
		/// 已经拥有 Kitty
		KittyAlreadyOwned,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 创建 Kitty
		#[pallet::weight(0)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let kitty_id = match Self::kitties_count() {
				Some(id) => {
					ensure!(id != T::KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
					id
				}
				None => 1u32.into(),
			};

			// 扣除质押金额
			T::Currency::reserve(&who, T::ReserveOfNewCreate::get()).map_err(|_| Error::<T>::NotEnoughBalance)?;

			let dna = Self::random_value(&who);

			Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
			Owner::<T>::insert(kitty_id, Some(&who));
			KittiesCount::<T>::put(kitty_id + 1u32.into());

			Self::deposit_event(Event::KittyCreated(who, kitty_id));

			Ok(())
		}

		/// 转让 Kitty
		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			kitty_id: T::KittyIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let owner = Owner::<T>::get(&kitty_id).unwrap();
			ensure!(owner == sender, Error::<T>::NotOwnerOfKitty);

			Self::transfer_kitty(sender, to, kitty_id);
			Ok(())
		}

		/// 生产 Kitty
		/// 父母的编号不能相同
		#[pallet::weight(0)]
		pub fn breed(
			origin: OriginFor<T>,
			kitty_id_1: T::KittyIndex,
			kitty_id_2: T::KittyIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameParentIndex);

			let owner1 = Self::owner(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
			let owner2 = Self::owner(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;

			ensure!(owner1 == who, Error::<T>::NotOwnerOfKitty);
			ensure!(owner2 == who, Error::<T>::NotOwnerOfKitty);

			let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
			let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;

			let kitty_id = match Self::kitties_count() {
				Some(id) => {
					ensure!(id != T::KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
					id
				}
				None => 1u32.into(),
			};

			let dna_1 = kitty1.0;
			let dna_2 = kitty2.0;

			let selector = Self::random_value(&who);
			let mut new_dna = [0u8; 16];

			for i in 0..dna_1.len() {
				new_dna[i] = (selector[i] & dna_1[i]) | (!selector[i] & dna_2[i])
			}

			Kitties::<T>::insert(kitty_id, Some(Kitty(new_dna)));
			Owner::<T>::insert(kitty_id, Some(&who));
			KittiesCount::<T>::put(kitty_id + 1u32.into());

			Self::deposit_event(Event::KittyCreated(who, kitty_id));

			Ok(())
		}

		/// 出售 Kitty
		/// price 为 None 时, 表示取消出售
		#[pallet::weight(0)]
		pub fn sell(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Some(who.clone()) == Self::owner(kitty_id), Error::<T>::NotOwnerOfKitty);

			KittiesPrice::<T>::mutate_exists(kitty_id, |p| *p = Some(price));

			Self::deposit_event(Event::KittyForSale(who, kitty_id, price));

			Ok(())
		}

		/// 购买 Kitty
		#[pallet::weight(0)]
		pub fn buy(origin: OriginFor<T>, kitty_id: T::KittyIndex) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			let owner = Self::owner(kitty_id).unwrap();
			ensure!(owner != buyer.clone(), Error::<T>::KittyAlreadyOwned);

			let price = Self::kitties_price(kitty_id).ok_or(Error::<T>::NotForSale)?;

			let reserve = T::ReserveOfNewCreate::get();

			// 扣除质押金额
			T::Currency::reserve(&buyer, reserve).map_err(|_| Error::<T>::NotEnoughBalance)?;

			// 出售方解除质押
			T::Currency::unreserve(&owner, reserve);

			// 转账
			T::Currency::transfer(
				&buyer,
				&owner,
				price,
				frame_support::traits::ExistenceRequirement::KeepAlive,
			)?;

			// 出售下架
			KittiesPrice::<T>::remove(kitty_id);

			Self::transfer_kitty(owner, buyer, kitty_id);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// 随机数生成
		fn random_value(who: &T::AccountId) -> [u8; 16] {
			let payload =
				(T::Randomness::random_seed(), &who, <frame_system::Pallet<T>>::extrinsic_index());
			payload.using_encoded(blake2_128)
		}

		/// 转移 Kitty
		fn transfer_kitty(from: T::AccountId, to: T::AccountId, kitty_id: T::KittyIndex) {
			Owner::<T>::insert(kitty_id, Some(to.clone()));
			Self::deposit_event(Event::KittyTransfered(from, to, kitty_id));
		}
	}
}
