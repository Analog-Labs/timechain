use scale_codec::{Decode, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
pub const DECODE_BLOCK_SIZE: usize = 32;

#[derive(Debug)]
pub enum DecodeError {
	InsufficientData,
	InvalidBlockSize,
}

pub trait AbiFixedDecode: Sized {
	fn decode_from_block(block: &[u8]) -> Result<Self, DecodeError>;
}

macro_rules! impl_abi_fixed_decode_int {
	($type:ty, $size:expr) => {
		impl AbiFixedDecode for $type {
			fn decode_from_block(block: &[u8]) -> Result<Self, DecodeError> {
				if block.len() != DECODE_BLOCK_SIZE {
					return Err(DecodeError::InvalidBlockSize);
				}
				let bytes: [u8; $size] =
					block[DECODE_BLOCK_SIZE - $size..DECODE_BLOCK_SIZE].try_into().unwrap();
				Ok(<$type>::from_be_bytes(bytes))
			}
		}
	};
}

impl_abi_fixed_decode_int!(u8, 1);
impl_abi_fixed_decode_int!(u16, 2);
impl_abi_fixed_decode_int!(u32, 4);
impl_abi_fixed_decode_int!(u64, 8);

impl AbiFixedDecode for [u8; DECODE_BLOCK_SIZE] {
	fn decode_from_block(block: &[u8]) -> Result<Self, DecodeError> {
		if block.len() != DECODE_BLOCK_SIZE {
			return Err(DecodeError::InvalidBlockSize);
		}
		Ok(block.try_into().unwrap())
	}
}

pub trait AbiDynamicDecode: Sized {
	fn decode_dynamic(input: &[u8]) -> Result<(Self, usize), DecodeError>;
}

impl AbiDynamicDecode for Vec<u8> {
	fn decode_dynamic(input: &[u8]) -> Result<(Self, usize), DecodeError> {
		if input.len() < DECODE_BLOCK_SIZE {
			return Err(DecodeError::InsufficientData);
		}
		let len_bytes = &input[0..DECODE_BLOCK_SIZE];
		let length = u64::from_be_bytes(
			len_bytes[DECODE_BLOCK_SIZE - 8..DECODE_BLOCK_SIZE].try_into().unwrap(),
		) as usize;
		let padded_length = if length % DECODE_BLOCK_SIZE == 0 {
			length
		} else {
			length + (DECODE_BLOCK_SIZE - (length % DECODE_BLOCK_SIZE))
		};
		let total_size = DECODE_BLOCK_SIZE + padded_length;
		if input.len() < total_size {
			return Err(DecodeError::InsufficientData);
		}
		let data = input[DECODE_BLOCK_SIZE..DECODE_BLOCK_SIZE + length].to_vec();
		Ok((data, total_size))
	}
}

use scale_info::prelude::vec;

use crate::Address32;

pub trait FixedSizeEncodable {
	fn left_pad_32(&self) -> [u8; 32];
}

macro_rules! impl_fixed_size_encodable {
    ($($n:expr),*) => {
        $(
            impl FixedSizeEncodable for [u8; $n] {
                fn left_pad_32(&self) -> [u8; 32] {
                    let mut out = [0u8; 32];
                    out[32-$n..].copy_from_slice(self);
                    out
                }
            }
        )*
    }
}

impl_fixed_size_encodable!(
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
	26, 27, 28, 29, 30, 31, 32
);

// encodes dynamic length data, Stores the length of data in first 32 bytes and then store the data in multiple of 32 bytes
pub fn encode_dynamic(bytes: &[u8]) -> vec::Vec<u8> {
	let mut encoded = vec::Vec::new();
	// encode 32 with length of bytes
	encoded.extend_from_slice(&bytes.len().to_be_bytes().left_pad_32());
	// store actual data
	encoded.extend_from_slice(bytes);
	// pad remaining data with 0 until we have a multiplier of 32 bytes
	let remainder = bytes.len() % 32;
	let padding = if remainder == 0 { 0 } else { 32 - remainder };
	encoded.extend_from_slice(&vec![0u8; padding]);
	encoded
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, Decode, Encode, TypeInfo, Eq, PartialEq, Ord, PartialOrd)]
pub struct CCTPMessage {
	pub attestation: Vec<u8>,
	pub message: Vec<u8>,
	pub extra_data: Vec<u8>,
}

impl CCTPMessage {
	pub fn encode(&self) -> Vec<u8> {
		let mut tail = Vec::new();
		let attestation = encode_dynamic(&self.attestation);
		let message = encode_dynamic(&self.message);
		let extra_data = encode_dynamic(&self.extra_data);

		// add 32 * 2 bytes for 2 offsets that we have
		let attestation_offset = 3 * DECODE_BLOCK_SIZE;
		let message_offset = attestation_offset + attestation.len();
		let extra_offset = message_offset + message.len();

		tail.extend_from_slice(&attestation_offset.to_be_bytes().left_pad_32());
		tail.extend_from_slice(&message_offset.to_be_bytes().left_pad_32());
		tail.extend_from_slice(&extra_offset.to_be_bytes().left_pad_32());

		tail.extend_from_slice(&attestation);
		tail.extend_from_slice(&message);
		tail.extend_from_slice(&extra_data);

		// prepends 0x20 in the list to tell where the data starts
		let mut encoded = Vec::new();
		encoded.extend_from_slice(&32u64.to_be_bytes().left_pad_32());
		encoded.extend_from_slice(&tail);

		encoded
	}

	pub fn from_bytes(input: &[u8]) -> Result<Self, DecodeError> {
		if input.len() < DECODE_BLOCK_SIZE {
			return Err(DecodeError::InsufficientData);
		}

		// removing the head offset from the bytes
		let _ = u64::decode_from_block(&input[0..DECODE_BLOCK_SIZE])?;
		let input = &input[DECODE_BLOCK_SIZE..];

		// 2 fields
		if input.len() < 3 * DECODE_BLOCK_SIZE {
			return Err(DecodeError::InsufficientData);
		}

		let mut offset = 0;
		let attestation_offset =
			u64::decode_from_block(&input[offset..offset + DECODE_BLOCK_SIZE])?;
		offset += DECODE_BLOCK_SIZE;

		let message_offset = u64::decode_from_block(&input[offset..offset + DECODE_BLOCK_SIZE])?;
		offset += DECODE_BLOCK_SIZE;
		let extra_offset = u64::decode_from_block(&input[offset..offset + DECODE_BLOCK_SIZE])?;

		let (attestation, _) = <Vec<u8>>::decode_dynamic(&input[attestation_offset as usize..])?;
		let (message, _) = <Vec<u8>>::decode_dynamic(&input[message_offset as usize..])?;
		let (extra_data, _) = <Vec<u8>>::decode_dynamic(&input[extra_offset as usize..])?;

		Ok(Self {
			attestation,
			message,
			extra_data,
		})
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct SwapPrerequisites {
	pub universal_router: Address32,
	pub permit2: Address32,
	pub token_messenger: Address32,
	pub msg_transmitter: Address32,
	pub usdc: Address32,
	pub weth: Address32,
}
