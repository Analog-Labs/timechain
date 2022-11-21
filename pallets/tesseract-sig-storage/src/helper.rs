// use std::collections::hash_map::DefaultHasher;
use core::hash::{Hash, Hasher};
use core::hash::BuildHasher;
// use std::time::SystemTime;
// use chrono::prelude::*;
use pallet_randomness_collective_flip;


// use frame_support::traits::Randomness;
// #[frame_support::pallet]
// pub mod pallet {
//     use super::*;
//     use frame_support::pallet_prelude::*;
//     use frame_system::pallet_prelude::*;
//     #[pallet::pallet]
//     #[pallet::generate_store(pub(super) trait Store)]
//     pub struct Pallet<T>(_);
//     #[pallet::config]
//     pub trait Config: frame_system::Config + pallet_randomness_collective_flip::Config {}
//     #[pallet::call]
//     impl<T: Config> Pallet<T> {
//         #[pallet::weight(0)]
//         pub fn random_module_example(origin: OriginFor<T>) -> DispatchResult {
//             let _random_value = <pallet_randomness_collective_flip::Pallet<T>>::random(&b"my context"[..]);
//             Ok(())
//         }
//     }
// }


pub struct Lib_Fn {
}

impl Lib_Fn {
	pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
		// let mut s = DefaultHasher::new();
		// t.hash(&mut s);
		// s.finish()
        1234
	}
    pub fn calculate_timeStamp() -> u64 {
        // let utc: DateTime<Utc> = Utc::now(); 
        // let val = utc.timestamp_millis() as u64;
		// // let val = SystemTime::now().elapsed().unwrap().as_secs() as u64;

        // val
        // RandomMaterial::
        // let data = pallet_randomness_collective_flip::Pallet::random_material().;
        1234
	}
}