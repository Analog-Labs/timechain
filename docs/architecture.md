# Architecture of Timechain Node
A Timechain node is a software that allows users to join a blockchain network, establish peer-to-peer connections, and participate in the blockchain consensus. The Timechain node is built on the Substrate framework and includes various components such as the p2p network, runtime, client, wasm execution, rpc, and pallets.

In addition to the core functionalities of a blockchain node, the Timechain node can also run other applications such as TSS (Threshold Signature Scheme) and task execution. These additional functions are executed within the chronicle worker, which operates in the same process as the Substrate client. The chronicle worker starts with handlers from Substrate components such as the p2p network, grandpa message notifier, Substrate backend, rpc service, and runtime API, allowing easy access to all services provided by the Substrate node.

 ## Summary
 The summary is a brief overview of the blockchain client architecture. It describes the main features and functionalities of the client, as well as its overall design. The summary should be concise and provide a clear understanding of what the blockchain client is and what it does.
 ## Modules
 The modules of a blockchain client are the different components that make up the client. These modules can be divided into three categories: the core module, the network module, and the user interface module.
 ### Core Module
 The core module is the heart of the blockchain client. It provides the core functionality of the client, such as creating and managing blockchain assets, validating transactions, and maintaining the blockchain ledger. The core module also includes the consensus algorithm, which is responsible for ensuring that all nodes on the network agree on the state of the blockchain ledger.
 ### Network Module
 The network module is responsible for connecting the blockchain client to the blockchain network. It handles the communication between the client and other nodes on the network, as well as the synchronization of the blockchain ledger. The network module also includes the peer-to-peer networking protocol, which is used to establish connections between nodes on the network.
 ### User Interface Module
 The user interface module provides the graphical user interface (GUI) for the blockchain client. It allows users to interact with the blockchain client and perform various tasks, such as creating and managing blockchain assets, viewing transaction history, and monitoring the status of the blockchain network.
 ## Interface
 The interface of a blockchain client is the set of APIs and protocols that enable users to interact with the client. The interface can be divided into two categories: the user interface and the application programming interface (API).
 ### User Interface
 The user interface is the graphical interface that users interact with when using the blockchain client. It provides an easy-to-use interface for users to create and manage their blockchain assets, view transaction history, and monitor the status of the blockchain network.
 ### Application Programming Interface (API)
 The API is the set of protocols and tools that developers can use to interact with the blockchain client. It provides a way for developers to create custom applications that interact with the blockchain network, such as decentralized applications (dApps).
 ## Reference
 The reference section of a blockchain client documentation provides detailed information about the client's architecture, modules, and interface. It includes technical specifications, code samples, and other resources that developers can use to build applications that interact with the blockchain client.
 In conclusion, the architecture of a blockchain client is comprised of four main components: the summary, modules, interface, and reference. Each component plays a critical role in the overall design and functionality of the client. By understanding the architecture of a blockchain client, developers can build custom applications that interact with the blockchain network.