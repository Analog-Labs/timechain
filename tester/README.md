## Running GMP Benchmark

### Command
` --network "src_network_id;src_network_rpc_url" --network "dest_network_id;dest_network_rpc_url" gmp-benchmark [total_number_of_tasks] [src_contract] [dest_contract]` </br>

gmp-benchmark total_number_of_tasks is important. </br>

gmp-benchmark total_number_of_tasks (src_contract, dest_contract) these contracts are optional, if not provided then it deploys the contract. </br>
These parameters are important when we want to reuse the sample deployed contract for testing purpose since we deposit funds inside it. </br>
