import csv
import os
import requests
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime
from matplotlib.ticker import MultipleLocator
from dotenv import load_dotenv
load_dotenv()


ETH_RPC = os.getenv('ETHEREUM_RPC')
ARB_RPC = os.getenv('ARBITRUM_RPC')

rpc_urls = {
    1: ETH_RPC,      
    42161: ARB_RPC,         
}
    
chain_names = {
    1: "Ethereum",
    42161: "Arbitrum", 
}

def read_csv(path):
    data = {}
    with open(path, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            if row[0].isdigit():
                chain_id = int(row[0])
                l2block = int(row[1])
                l1block = int(row[2])
                timestamp = int(row[3])
                gas_price = int(row[4])
                
                if chain_id not in data:
                    data[chain_id] = []
                
                data[chain_id].append({
                    'l2block': l2block,
                    'l1block': l1block,
                    'timestamp': timestamp,
                    'csv_gas_price': gas_price
                })
    
    return data

def make_rpc_call(url, method, params):
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

def get_block_fee_history(chain_id, block_number):
    if chain_id not in rpc_urls:
        return None
        
    url = rpc_urls[chain_id]
    
    block_hex = hex(block_number)
    
    params = [1, block_hex, [25, 50, 75]]
    
    result = make_rpc_call(url, "eth_feeHistory", params)
    print(result);
    
    if result and 'baseFeePerGas' in result:
        base_fee = int(result['baseFeePerGas'][0], 16)  
        
        if 'reward' in result and result['reward'] and len(result['reward']) > 0:
            priority_fee = int(result['reward'][0][1], 16)  
            effective_gas_price = base_fee + priority_fee
        else:
            effective_gas_price = base_fee
        
        return {
            'base_fee': base_fee,
            'effective_gas_price': effective_gas_price,
            'gas_used_ratio': result['gasUsedRatio'][0] if result['gasUsedRatio'] else 0
        }
    
    return None

def wei_to_gwei(wei_value):
    return wei_value / 1e9


def plot_data(chain_results, chain_name='chain'):
    if not chain_results:
        print("No data to plot.")
        return

    chain_results.sort(key=lambda x: x['block'])
    blocks = [r['block'] for r in chain_results]
    csv_prices = [r['csv_gas_price'] for r in chain_results]
    rpc_prices = [r['rpc_gas_price'] for r in chain_results]
    timestamps = [datetime.fromtimestamp(r['timestamp']) for r in chain_results]

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 10))

    ax1.plot(blocks, csv_prices, label='GasNetwork Gas Price', marker='o', color='blue')
    ax1.plot(blocks, rpc_prices, label='RPC Gas Price', marker='x', color='red')
    for b, c, r in zip(blocks, csv_prices, rpc_prices):
        ax1.plot([b, b], [c, r], color='gray', linestyle='--', alpha=0.5)

    ax1.set_xticks(blocks)
    ax1.set_xticklabels([str(b) for b in blocks], rotation=45)

    minor_ticks = [(blocks[i] + blocks[i+1]) / 2 for i in range(len(blocks)-1)]
    ax1.set_xticks(minor_ticks, minor=True)

    ax1.grid(which='major', color='black', linestyle='-', linewidth=0.6)

    ax1.set_xlabel('Block Number')
    ax1.set_ylabel('Gas Price (Gwei)')
    ax1.legend()
    ax1.set_title(f'{chain_name} Gas Price Comparison by Block (With Extra Grid Lines)')

    differences = [abs(c - r) for c, r in zip(csv_prices, rpc_prices)]
    ax2.plot(timestamps, differences, marker='o', color='purple', label='Absolute Difference')
    ax2.set_title('Gas Price Differences Over Time')
    ax2.set_xlabel('Date')
    ax2.set_ylabel('Absolute Difference (Gwei)')
    ax2.legend()
    ax2.grid(True, alpha=0.3)
    ax2.tick_params(axis='x', rotation=45)

    plt.tight_layout()

def main():
    csv_data = read_csv('gas_price.csv')

    all_results = {}    
    for chain_id, entries in csv_data.items():
        if chain_id == 1:
            continue;
        chain_name = chain_names.get(chain_id, f"Chain {chain_id}")
        print(f"\nProcessing {chain_name} ({len(entries)} entries):")

        results = []
        
        for i, entry in enumerate(entries):
            block_number = entry['l1block'] if chain_id == 1 else entry['l2block']
            
            print(f"  Processing block {block_number} ({i+1}/{len(entries)})")
            
            rpc_data = get_block_fee_history(chain_id, block_number)
            
            if rpc_data:
                rpc_gas_price_gwei = wei_to_gwei(rpc_data['effective_gas_price'])
                
                results.append({
                    'block': block_number,
                    'csv_gas_price': wei_to_gwei(entry['csv_gas_price']),
                    'rpc_gas_price': rpc_gas_price_gwei,
                    'timestamp': entry['timestamp'],
                    'base_fee_gwei': wei_to_gwei(rpc_data['base_fee']),
                })
                
                print(f"    CSV: {entry['csv_gas_price']:.2f} Gwei, RPC: {rpc_gas_price_gwei:.2f} Gwei")
            else:
                print(f"    Failed to fetch RPC data for block {block_number}")
        
        all_results[chain_id] = results
        print(f"  Successfully processed {len(results)} blocks")
    
    if not all_results:
        print("No data available for plotting")
        return

    print(all_results);

    plot_data(all_results[1], "Ethereum");
    plot_data(all_results[42161], "Arbitrum");
    plt.show();

if __name__ == "__main__":
    main()
