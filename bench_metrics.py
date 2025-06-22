import pandas as pd

df = pd.read_csv('benchmark.csv')

df['latency'] = df['msg_sent_to_dest_chain'] - df['msg_received_on_timechain']
print(f"Average latency: {df['latency'].mean()} blocks")
print(f"Max latency: {df['latency'].max()} blocks")

throughput = df.groupby('msg_sent_to_dest_chain').size()
print(throughput)
print(f"Average throughput: {throughput.mean()} messages/block")
