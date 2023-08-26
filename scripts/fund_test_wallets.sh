eth_url="http://127.0.0.1:8080"
eth_blockchain="ethereum"
eth_network="dev"

astar_url="http://127.0.0.1:8081"
astar_blockchain="astar"
astar_network="dev"

rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile1 faucet 1000000000000
rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile2 faucet 1000000000000
rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile3 faucet 1000000000000
rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile4 faucet 1000000000000
rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile5 faucet 1000000000000
rosetta-wallet --url=$eth_url --blockchain=$eth_blockchain --network=$eth_network --keyfile=dummy_wallets/eth_keyfile6 faucet 1000000000000

rosetta-wallet --url=$astar_url --blockchain=$astar_blockchain --network=$astar_network --keyfile=dummy_wallets/astar_keyfile1 faucet 100000000
rosetta-wallet --url=$astar_url --blockchain=$astar_blockchain --network=$astar_network --keyfile=dummy_wallets/astar_keyfile2 faucet 100000000
rosetta-wallet --url=$astar_url --blockchain=$astar_blockchain --network=$astar_network --keyfile=dummy_wallets/astar_keyfile3 faucet 100000000
