use scale_info::prelude::vec::Vec;
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
