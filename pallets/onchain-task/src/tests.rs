use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn it_works_adding_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(OnChainTask::add_task( 1, TesseractTask::AddChain));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));
	});
}

#[test]
fn it_works_removing_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(OnChainTask::add_task( 1, TesseractTask::AddChain));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));

		// Call the tesseract member extrinsic
		assert_ok!(OnChainTask::remove_task(RawOrigin::Root.into(), 1,));
		// Checking that the member has been rmoved
		assert_eq!(OnChainTask::tesseract_tasks(1), None);
	});
}

#[test]
fn it_works_storing_and_get_chain_key() {
	let chain_key: ChainKey = "chain_key_1".as_bytes().to_owned();
	let chain: Chain = "this_is_the_chain".as_bytes().to_owned();
	let chain_endpoint: ChainEndpoint = "chain_endpoint".as_bytes().to_owned();
	let exchange: Exchange = "exchange".as_bytes().to_owned();
	let exchange_address: ExchangeAddress = "exchange_address".as_bytes().to_owned();
	let exchange_endpoint: ExchangeEndpoint = "exchange_endpoint".as_bytes().to_owned();
	let token: Token = "token".as_bytes().to_owned();
	let token_address: TokenAddress = "token_address".as_bytes().to_owned();
	let token_endpoint: TokenEndpoint = "token_endpoint".as_bytes().to_owned();
	let swap_token: SwapToken = "swap_token".as_bytes().to_owned();
	let swap_token_address: SwapTokenAddress = "swap_token_address".as_bytes().to_owned();
	let swap_token_endpoint: SwapTokenEndpoint = "swap_token_endpoint".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(OnChainTask::add_task(RawOrigin::Root.into(), 1, TesseractTask::AddChain));

		// Call the store signature extrinsic
		assert_ok!(OnChainTask::store_onchain_data(
			RawOrigin::Signed(1).into(),
			chain_key.clone(),
			chain.clone(),
			chain_endpoint.clone(),
			exchange.clone(),
			exchange_address.clone(),
			exchange_endpoint.clone(),
			token.clone(),
			token_address.clone(),
			token_endpoint.clone(),
			swap_token.clone(),
			swap_token_address.clone(),
			swap_token_endpoint.clone(),
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(OnChainTask::onchain_task_store(chain_key), Some(chain), "{} {} {} {} {} {} {} {} {} {}", Some(chain_endpoint), Some(exchange), Some(exchange_address), Some(exchange_endpoint), Some(token), Some(token_address), Some(token_endpoint), Some(swap_token), Some(swap_token_address), Some(swap_token_endpoint));
	});
}