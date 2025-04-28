# DEMO

1. Start node: RUST_LOG="error,evm=debug,sc_rpc_server=info,runtime::revive=debug" cargo run --release --features testnet --bin timechain-node -- --dev

./target/release/timechain-node -- --dev 
2. Start ETH RPC in separate window:  RUST_LOG="info,eth-rpc=debug" cargo run --release -p pallet-revive-eth-rpc -- --dev
3. Call map_account for Alice
4. Transfer funds from Alice to H160 address that we generated ourselves via: CALL REVIVE.Call from Alice
dest: 0xeFF80f68C3114A4F6d0151A35ddE6E1fAF092366 
value: 1000000000000000 
gasLimit.refTime: 10000000
gasLimit.proofSize: 100000000  
storageDepositLimit: 100000000000000  
data: 0x
5. Use H160 address that we generated ourselves to deploy and interact with a contract via remix plugin

Other Useful Commands:
For more logs:
RUST_LOG="error,evm=debug,sc_rpc_server=info,runtime::revive=trace,polkavm=trace" cargo run --release --features testnet --bin timechain-node -- --dev

Purge chain:
./target/release/timechain-node purge-chain --dev -y 


Errors/incompatibilities:- debug_traceTransaction not available (regardless of passing trace feature flags)




Reference Docs: https://contracts.polkadot.io/work-with-a-local-node







