#!/bin/bash

network_type=$1  # The first argument to the script
mode=$2         # The second argument to the script

run_ethereum() {
    if [ "$1" == "single" ]; then
        echo "Running single node Ethereum configuration."
        docker compose --profile ethereum down -v && ./scripts/build_docker.sh && docker compose --profile ethereum up
    elif [ "$1" == "multi" ]; then
        echo "Running multi node Ethereum configuration."
        docker compose -f docker-compose-multinode.yml --profile ethereum down -v && ./scripts/build_docker.sh && docker compose -f docker-compose-multinode.yml --profile ethereum up -d
    fi
}

run_astar() {
    if [ "$1" == "single" ]; then
        echo "Running single node Astar configuration."
        docker compose --profile astar down -v && ./scripts/build_docker.sh && docker compose --profile astar up
    elif [ "$1" == "multi" ]; then
        echo "Running multi node Astar configuration."
        docker compose -f docker-compose-multinode.yml --profile astar down -v && ./scripts/build_docker.sh && docker compose -f docker-compose-multinode.yml --profile astar up
    fi
}

run_gmp() {
    if [ "$1" == "single"]; then
        echo "Running single astar and eth node for gmp"
        docker compose --profile ethereum --profile astar down -v && ./scripts/build_docker.sh && docker compose --profile ethereum --profile astar up
    elif [ "$1" == "multi" ]; then
        echo "Running multi astar and eth node for gmp"
        docker compose -f docker-compose-multinode.yml --profile ethereum --profile astar down -v && ./scripts/build_docker.sh && docker compose -f docker-compose-multinode.yml --profile ethereum --profile astar up
    fi
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