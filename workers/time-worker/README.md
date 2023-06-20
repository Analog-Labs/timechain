## **Time worker**

This worker is responsible for internal processes like:
- TSS dkg process
- TSS verification
- Receiving TSS gossiped data
- Broadcasting event-worker gossiped data etc

### **TSS DKG**:
When a shard is registered. Members of the shard initiates a TSS process which is Threshold Signature Scheme Distributed Key Generation, also known as TSS DKG.  

When a new shard is registered all members of shard shares they public key with other members of shard and other info to generate commitment shares. After commitments are received process makes a public group key that is same for every member in shard.

### **TSS verification**:
This process needs TSS DKG to be completed otherwise it might throw error of `Invalid State`. This workers takes a gossip receiver `sign_data_receiver` which is a `mpsc` receiver and multiple senders are sent to respective channels from where data is received. After data is received all nodes participating in tss process signs the data by their partial signature and later on all partial signatures/ threshold signatures are combined into one signature and other nodes verifies if the signature is valid if so. Then data is stored into db.

### **Receiving TSS gossiped data**:
When gossip data is received for the respective topic we check if data is decodable to `TimeMessage` then we send data to tss process for further processing.

### **Broadcasting event-worker gossiped data etc**:
When gossip data is received for the respective topic we check if data is decodable to `EthTxValidation` then we send data to event-worker via `tx_data_sender` for validation for transaction hash and later receive via `sign_data_receiver` for tss verification.




