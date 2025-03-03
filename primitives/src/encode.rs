use scale_info::prelude::vec;

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
