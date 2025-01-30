#!/bin/bash

network_type=$1  # The first argument to the script
mode=$2         # The second argument to the script

run_ethereum() {
        echo "Running single node Ethereum configuration."
        docker compose --profile evm down -v && ./scripts/build_docker.sh && docker compose --profile evm up  
}

# Check the network type and mode and call the appropriate function
case $network_type in
    eth)
        run_ethereum $mode
        ;;
    astar)
        run_astar $mode
        ;;
    gmp)
        run_gmp $mode
        ;;
    *)
        echo "Unknown network type. Please specify 'eth' or 'astar'."
        ;;
esac
