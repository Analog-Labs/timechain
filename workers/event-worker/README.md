## **Event Worker**

This worker is responsible for handling events from foreign chains and executing tss process for `payable task executor`.

`Payable Task Executor`: <br/>
    For Payable Task executor this service takes the transaction hash of transaction which is executed and look for transaction generated events and verifies if the contract address executed is same as the contract address in transaction receipt. If true passes the transaction hash to `tss process` for further processing.


