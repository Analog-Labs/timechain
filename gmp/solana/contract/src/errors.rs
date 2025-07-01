use anchor_lang::prelude::*;

#[error_code]
pub enum GatewayError {
	#[msg("The gateway is not initialized.")]
	NotInitialized,
	#[msg("The gateway is already initialized")]
	AlreadyInitialized,
	#[msg("Unauthorized: The provided authority is not the gateway admin.")]
	Unauthorized,
	#[msg("Too much shards to register")]
	ShardsLengthExceedLimit,
	#[msg("Msg size too large")]
	MsgTooLarge,
	#[msg("Invalid y parity")]
	InvalidYParity,
	#[msg("y parity mismatch")]
	YParityMismatch,
	#[msg("invalid gas limit")]
	InvalidGasLimit,
	#[msg("invalid gas price")]
	InvalidGasPrice,
	#[msg("routes length exeeds limit")]
	RoutesLengthExceedLimit,
}
