#!/bin/bash
set -e

# By default, doesn't broadcast any transactions.
DRY_RUN=1

# Setup console colors
if test -t 1 && command -v tput >/dev/null 2>&1; then
    ncolors=$(tput colors)
    if test -n "${ncolors}" && test "${ncolors}" -ge 8; then
        bold_color=$(tput bold)
        green_color=$(tput setaf 2)
        warn_color=$(tput setaf 3)
        error_color=$(tput setaf 1)
        reset_color=$(tput sgr0)
    fi
    # 72 used instead of 80 since that's the default of pr
    ncols=$(tput cols)
fi
: "${ncols:=72}"

# process arguments
while [[ $# -gt 0 ]]
do
    case "$1" in
        --migrate)
          unset DRY_RUN
          shift 1
        ;;
        --proxy=*)
          PROXY_ADDRESS="${i#*=}"
          shift 1
        ;;
        -pk=*|--private-key=*)
          PRIVATE_KEY="${i#*=}"
          shift 1
        ;;
        --sepolia-rpc=*)
          SEPOLIA_RPC_URL="${i#*=}"
          shift 1
        ;;
        --shibuya-rpc=*)
          SHIBUYA_RPC_URL="${i#*=}"
          shift 1
        ;;
        --amoy-rpc=*)
          POLYGON_AMOY_RPC_URL="${i#*=}"
          shift 1
        ;;
        *)
        warn "Unknown argument: $1"
        echo "Usage: $0 --pk=<PRIVATE_KEY> --proxy=PROXY_ADDRESS [--migrate] [--sepolia-rpc=] [--shibuya-rpc=] [--amoy-rpc=]"
        ;;
    esac
done

# Load .env file
if [ -f .env ]; then
    echo "Load .env file"
    source .env
else
    echo ".env file not found, run 'cp .env.example .env' and fill the values"
fi

# Check if PRIVATE_KEY is set
if [ -z "${PRIVATE_KEY}" ]; then
  echo "PRIVATE_KEY is not set"
  exit 1
fi

# Check if PROXY_ADDRESS is set
if [ -z "${PROXY_ADDRESS}" ]; then
  echo "PROXY_ADDRESS is not set"
  exit 1
fi

# Set fork-url
PARAMS=(-vvvv)

# Verify if the migration is going to be broadcasted
if [ -z "${DRY_RUN}" ]; then
  read -r -p "running in broadcast mode, the transaction will be broadcasted, are you sure you want to continue? [y/n] " response
  case "$response" in
      [yY][eE][sS]|[yY]) 
          PARAMS+=(--broadcast)
          ;;
      *)
        echo "running in dry-mode..."
        ;;
  esac
fi

forge script ./scripts/Migrate.sol "${PARAMS[@]}"
