## **Payable Task Executor**

*Functionality*:<br/>
    This service is responsible for executing the payable task. When a payable task is submitted to chain provided destination chain, destination contract address and function signature. This service will fetch the task and use validator's account for that foreign chain and will execute the task. After executing the task it will take the transaction hash and broadcast to network to verify the transaction and also sends it to event-worker for local verification. <br/>
    All other nodes of shard then validates the transaction and if its valid then it will be sent to tss process for tss signature generation. <br/>
    The payable task is run by single node of shard and currently only collector node is reponsible for executing payable task <br/>

Currently support functions are:

```
Function::EthereumTxWithoutAbi {
    address: (String, address of contract),
    function_signature: (String, function signature of function to be executed),
    input: (Vec<String>, input parameters in form of string for function.)
    output: (This field can be left empty),
}
```






