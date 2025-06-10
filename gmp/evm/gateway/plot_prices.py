import csv
import os
import requests
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime
from matplotlib.ticker import MultipleLocator

def read_csv(path):
    data = {}
    with open(path, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            if row[0].isdigit():
                timestamp = int(row[1])
                price = float(row[2])
                data[timestamp] = price
    return data

def fetch_prices(timestamp):
    url = "https://min-api.cryptocompare.com/data/v2/histominute"
    params = {
        'fsym': 'ETH',
        'tsym': 'USDT',
        'toTs': timestamp,
        'limit': 1,
    }
    
    response = requests.get(url, params=params)
    if response.status_code == 200:
        data = response.json()
        api_data = data['Data']['Data']
        data_len = len(api_data)
        last_item = api_data[data_len - 1]
        if data['Response'] == 'Success' and data_len > 0:
            if timestamp - last_item['time'] <= 60:
                return last_item['time'], last_item['close']
            else:
                return last_item['time'], 0
def main():
    csv_data = read_csv('uni_prices.csv')
    uni_timestamps = []
    
    uni_prices = []
    api_timestamps = []
    api_prices = []
    
    for ts, price in csv_data.items():
        (api_ts, api_price)= fetch_prices(ts)
        if api_price:
            uni_timestamps.append(ts)
            uni_prices.append(price)
            api_timestamps.append(api_ts)
            api_prices.append(api_price)
    
    print(uni_timestamps);
    print(uni_prices);
    print(api_timestamps);
    print(api_prices);

    uni_dates = [datetime.fromtimestamp(ts) for ts in uni_timestamps]
    api_dates = [datetime.fromtimestamp(ts) for ts in api_timestamps]

    plt.figure(figsize=(14, 7))

    plt.plot(uni_dates, uni_prices, label='UNI Price', marker='o', color='blue', markersize=5)
    plt.plot(api_dates, api_prices, label='API Price', marker='x', color='green', markersize=5)

    plt.title('ETH Price Comparison: UNI vs API', fontsize=14)
    plt.xlabel('Time', fontsize=12)
    plt.ylabel('Price (USD)', fontsize=12)
    plt.legend(fontsize=12)
    
    all_dates = uni_dates + api_dates
    x_min = min(all_dates)
    x_max = max(all_dates)
    plt.xlim(x_min, x_max)
    
    ax = plt.gca()
    
    ax.xaxis.set_major_locator(mdates.HourLocator(interval=2))
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%m-%d %H:%M'))
    
    ax.yaxis.set_major_locator(MultipleLocator(2))
    ax.yaxis.set_minor_locator(MultipleLocator(1))
    
    ax.grid(which='major', axis='both', linestyle='-', alpha=0.7)
    ax.grid(which='minor', axis='both', linestyle=':', alpha=0.4)


    differences = [abs(u - a) for u, a in zip(uni_prices, api_prices)]
    max_diff = max(differences)
    max_diff_index = differences.index(max_diff)

    max_uni_ts = uni_timestamps[max_diff_index]
    max_api_ts = api_timestamps[max_diff_index]
    max_time_str = datetime.fromtimestamp(max_uni_ts).strftime('%Y-%m-%d %H:%M:%S')

    text_str = f"Max Difference: {max_diff:.4f}\nAt UNI timestamp: {max_time_str}"
    plt.text(0.02, 0.02, text_str, 
             transform=ax.transAxes, 
             fontsize=10,
             bbox=dict(facecolor='white', alpha=0.8))

    plt.gcf().autofmt_xdate()
    plt.show()
    
if __name__ == "__main__":
    main()
