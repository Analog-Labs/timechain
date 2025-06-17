import csv
import time
import os
import requests
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime
from matplotlib.ticker import MultipleLocator
import argparse 
from dotenv import load_dotenv
import numpy as np
load_dotenv()

def parse_arguments():
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    
    parser.add_argument(
        'url', 
        type=str,
        help='RPC endpoint URL for the blockchain network'
    )
    
    parser.add_argument(
        'block_time', 
        type=float,
        help='Average block production time in seconds'
    )
    
    return parser.parse_args()


def get_chain_id(url):
    """
    Get the chain ID from the blockchain node
    """
    result = make_rpc_call(url, "eth_chainId", [])
    if result:
        return int(result, 16)
    
    from urllib.parse import urlparse
    domain = urlparse(url).netloc
    return domain.replace('.', '_')

def make_rpc_call(url, method, params):
    """
    Make RPC call to blockchain node
    """
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }
    
    try:
        response = requests.post(url, json=payload, timeout=30)
        response.raise_for_status()
        result = response.json()
        
        if 'error' in result:
            print(f"RPC Error: {result['error']}")
            return None
            
        return result.get('result')
    except Exception as e:
        print(f"Request failed: {e}")
        return None

def fetch_historical_gas_data(url, block_count, newest_block):
    """
    Fetch historical gas price data using eth_feeHistory RPC method
    Fixed to 1024 blocks for simplicity
    """
    percentiles = [10, 50, 90]
    print("fetching history data for block", newest_block)
    if str(newest_block).isdigit():
        newest_block = hex(newest_block)
    params = [block_count, newest_block, percentiles]
    

    retries = 0
    init_wait = 2
    while retries < 3:
        result = make_rpc_call(url, "eth_feeHistory", params)
        if result: break
        retries += 1
        time.sleep(init_wait + retries)
    
    if not result:
        return None
    
    gas_data = []
    
    oldest_block = int(result['oldestBlock'], 16)
    base_fees = result['baseFeePerGas']
    gas_used_ratios = result['gasUsedRatio']
    rewards = result.get('reward', [])
    
    for i in range(len(base_fees) - 1):
        block_number = oldest_block + i
        base_fee_wei = int(base_fees[i], 16)
        base_fee_gwei = wei_to_gwei(base_fee_wei)
        
        priority_fee_50p = 0
        if i < len(rewards) and rewards[i] and len(rewards[i]) >= 2:
            priority_fee_50p = wei_to_gwei(int(rewards[i][1], 16))
        
        effective_gas_price = base_fee_gwei + priority_fee_50p
        
        gas_data.append({
            'block': block_number,
            'base_fee': base_fee_gwei,
            'effective_gas_price': effective_gas_price
        })
    
    return gas_data

def wei_to_gwei(wei_value):
    return wei_value / 1e9


def analyze_gas_price_statistics(gas_data):
    """
    Calculate statistical analysis of gas price data
    """
    if not gas_data:
        return None
    
    effective_prices = [data['effective_gas_price'] for data in gas_data]
    base_fees = [data['base_fee'] for data in gas_data]
    
    stats = {
        'effective_gas_price': {
            'min': np.min(effective_prices),
            'max': np.max(effective_prices),
            'mean': np.mean(effective_prices),
            'median': np.median(effective_prices),
            '90th_percentile': np.percentile(effective_prices, 90),
            '95th_percentile': np.percentile(effective_prices, 95),
            '98th_percentile': np.percentile(effective_prices, 98),
            '99th_percentile': np.percentile(effective_prices, 99)
        },
        'base_fee': {
            'min': np.min(base_fees),
            'max': np.max(base_fees),
            'mean': np.mean(base_fees),
            'median': np.median(base_fees),
            '90th_percentile': np.percentile(base_fees, 90),
            '95th_percentile': np.percentile(base_fees, 95),
            '98th_percentile': np.percentile(base_fees, 98),
            '99th_percentile': np.percentile(base_fees, 99)
        },
        'sample_count': len(gas_data)
    }
    
    return stats

def main():
    args = parse_arguments()
    fetch_duration = None
    
    if args.block_time <= 0:
        print("Error: Block time must be greater than 0")
        return


    current_file_path = os.path.dirname(os.path.abspath(__file__))
    data_dir = os.path.join(current_file_path, 'data')
    os.makedirs(data_dir, exist_ok=True)

    chain_id = get_chain_id(args.url)   
    csv_name = f"{chain_id}.csv"
    csv_path = os.path.join(data_dir, csv_name)


    print("="*60)
    print("GAS PRICE ANALYZER")
    print("="*60)
    print(f"RPC URL: {args.url}")
    print(f"Chain ID: {chain_id}")
    print(f"CSV File: {csv_path}")
    print(f"Block Production Time: {args.block_time} seconds")

    if os.path.exists(csv_path):
        print(f"\nReading gas data from existing CSV file: {csv_path}")
        all_gas_data = []
        with open(csv_path, 'r', newline='') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                all_gas_data.append({
                    'block': int(row['block']),
                    'base_fee': float(row['base_fee']),
                    'effective_gas_price': float(row['effective_gas_price'])
                })
        print(f"Successfully read {len(all_gas_data)} records from CSV")
    else:
        print("\nCSV file does not exist. Fetching data from blockchain...")
        
        seconds_per_month = 30 * 24 * 60 * 60 
        total_blocks_needed = int(seconds_per_month / args.block_time)
        max_blocks_per_call = 1024
        calls_needed = (total_blocks_needed + max_blocks_per_call - 1) // max_blocks_per_call

        print("Fetching historical gas price data...")

        all_gas_data = []
        current_block = "latest"
        
        print("Fetching monthly historical gas price data...")
        start_time = time.time()

        for call_num in range(calls_needed):
            remaining_blocks = total_blocks_needed - len(all_gas_data)
            blocks_this_call = min(max_blocks_per_call, remaining_blocks)
            
            if blocks_this_call <= 0:
                break
            
            print(f"Call {call_num + 1}/{calls_needed}: Fetching {blocks_this_call} blocks from block {current_block}")
            
            gas_data_batch = fetch_historical_gas_data(args.url, blocks_this_call, current_block)
            
            if not gas_data_batch:
                print(f"Failed to fetch data on call {call_num + 1}")
                break
            
            all_gas_data.extend(gas_data_batch)
            
            if gas_data_batch:
                oldest_block_this_call = min(data['block'] for data in gas_data_batch)
                current_block = oldest_block_this_call
            
            progress = (len(all_gas_data) / total_blocks_needed) * 100
            print(f"Progress: {len(all_gas_data):,}/{total_blocks_needed:,} blocks ({progress:.1f}%)")
            
            time.sleep(0.5)

        end_time = time.time()
        fetch_duration = end_time - start_time
        
        if not all_gas_data:
            print("Failed to fetch any gas data")
            return
        
        print(f"\nSuccessfully fetched data for {len(all_gas_data):,} blocks in {fetch_duration:.1f} seconds")
        
        print(f"\nSaving gas data to CSV file: {csv_path}")
        with open(csv_path, 'w', newline='') as csvfile:
            fieldnames = ['block', 'base_fee', 'effective_gas_price']
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            for data in all_gas_data:
                writer.writerow(data)
        print(f"Successfully saved {len(all_gas_data)} records to CSV")

    actual_hours = (len(all_gas_data) * args.block_time) / 3600
    actual_days = actual_hours / 24
    if fetch_duration is None:
        fetch_duration = 0.0
    
    stats = analyze_gas_price_statistics(all_gas_data)
    
    if not stats:
        print("Failed to analyze gas data")
        return
    
    print("\n" + "="*60)
    print("MONTHLY ANALYSIS RESULTS")
    print("="*60)
    print(f"Sample Size: {stats['sample_count']:,} blocks")
    print(f"Actual Time Range: {actual_days:.1f} days ({actual_hours:.1f} hours)")
    print(f"Data Collection Duration: {fetch_duration:.1f} seconds")
    
    print("\nEffective Gas Price (Gwei):")
    print(f"  Minimum: {stats['effective_gas_price']['min']:.4f}")
    print(f"  Maximum: {stats['effective_gas_price']['max']:.4f}")
    print(f"  Mean: {stats['effective_gas_price']['mean']:.4f}")
    print(f"  Median: {stats['effective_gas_price']['median']:.4f}")
    print(f"  90th Percentile: {stats['effective_gas_price']['90th_percentile']:.4f}")
    print(f"  95th Percentile: {stats['effective_gas_price']['95th_percentile']:.4f}")
    print(f"  98th Percentile: {stats['effective_gas_price']['98th_percentile']:.4f}")
    print(f"  99th Percentile: {stats['effective_gas_price']['99th_percentile']:.4f}")
    
    print("\nBase Fee (Gwei):")
    print(f"  Minimum: {stats['base_fee']['min']:.4f}")
    print(f"  Maximum: {stats['base_fee']['max']:.4f}")
    print(f"  Mean: {stats['base_fee']['mean']:.4f}")
    print(f"  Median: {stats['base_fee']['median']:.4f}")
    print(f"  90th Percentile: {stats['base_fee']['90th_percentile']:.4f}")
    print(f"  95th Percentile: {stats['base_fee']['95th_percentile']:.4f}")
    print(f"  98th Percentile: {stats['base_fee']['98th_percentile']:.4f}")
    print(f"  99th Percentile: {stats['base_fee']['99th_percentile']:.4f}")
    
if __name__ == "__main__":
    main()
