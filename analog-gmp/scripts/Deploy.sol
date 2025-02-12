// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (scripts/Deploy.sol)

pragma solidity ^0.8.0;

import {FactoryUtils} from "@universal-factory/FactoryUtils.sol";
import {IUniversalFactory} from "@universal-factory/IUniversalFactory.sol";
import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {NetworkID, NetworkIDHelpers} from "../src/NetworkID.sol";
import {ERC1967} from "../src/utils/ERC1967.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {Gateway} from "../src/Gateway.sol";
import {
    TssKey,
    GmpMessage,
    UpdateKeysMessage,
    Signature,
    Network,
    GmpStatus,
    GmpSender,
    PrimitiveUtils
} from "../src/Primitives.sol";

/**
 * @dev Message payload used to update the network info.
 * @param networkId Domain EIP-712 - Replay Protection Mechanism.
 * @param domainSeparator Domain EIP-712 - Replay Protection Mechanism.
 * @param gasLimit The maximum amount of gas we allow on this particular network.
 * @param relativeGasPrice Gas price of destination chain, in terms of the source chain token.
 * @param baseFee Base fee for cross-chain message approval on destination, in terms of source native gas token.
 * @param mortality maximum block in which this message is valid.
 */
struct UpdateNetworkInfo {
    uint16 networkId;
    bytes32 domainSeparator;
    uint64 gasLimit;
    UFloat9x56 relativeGasPrice;
    uint128 baseFee;
    uint64 mortality;
}

contract MigrateGateway is Script {
    using NetworkIDHelpers for NetworkID;
    using FactoryUtils for IUniversalFactory;

    /**
     * @dev The codehash of the proxy contract
     */
    bytes32 internal constant PROXY_CODEHASH = 0x54afeb06256bce71659256132ac18f1515de3011aaec4fbd6fc7b0c00c7263d8;
    
    /**
     * @dev The minimal balance required to deploy the proxy contract
     */
    uint256 internal constant MINIMAL_DEPLOYER_BALANCE = 0.5 ether;

    /**
     * @dev Universal Factory used to deploy the implementation contract
     * see https://github.com/Analog-Labs/universal-factory/tree/main for mode details.
     */
    IUniversalFactory internal constant FACTORY = IUniversalFactory(0x0000000000001C4Bf962dF86e38F0c10c7972C6E);

    /**
     * @dev Bytecode hash of the Universal Factory, used to verify if the contract is deployed.
     */
    bytes32 internal constant FACTORY_CODEHASH = 0x0dac89b851eaa2369ef725788f1aa9e2094bc7819f5951e3eeaa28420f202b50;

    /**
     * @dev Hash of the implementation contract creation code.
     */
    bytes32 internal constant IMPLEMENTATION_CODEHASH = keccak256(type(Gateway).creationCode);

    /**
     * @dev Salt of the implementation.
     */
    bytes32 internal constant IMPLEMENTATION_SALT = bytes32(uint256(0x010000000000));

    /**
     * @dev Default Proxy Admin, if none is provided, use this one.
     */
    address internal constant DEFAULT_ADMIN_ACCOUNT = 0xB41440FF80e1083350c91B21DE1061e0920A75AD;
    
    /**
     * Information about the current state of the migration
     * @param forkId The network fork id, see: https://book.getfoundry.sh/forge/fork-testing#forking-cheatcodes
     * @param mortality The maximum block number where the migration can be executed.
     * @param proxyAddress The address of the proxy contract
     */
    struct State {
        uint256 forkId;
        uint64 mortality;
        address proxyAddress;
    }

    struct NetworkConfiguration {
        string name;
        uint256 forkID;
        uint256 chainID;
        bool hasProxy;
        address adminAddress;
        address implementationContract;
        UpdateNetworkInfo info;
    }

    /**
     * Information about the current state of the migration
     * @param proxy The address of the proxy contract
     * @param proxyAdmin The address of the proxy admin
     * @param proxyDeployer The address of the proxy deployer
     * @param proxyDeployerNonce The nonce that must be used by the deployer to deploy the proxy contract
     * @param implementationDeployer Account used to deploy the implementation contract.
     */
    struct Configuration {
        address proxy;
        address proxyAdmin;
        address proxyDeployer;
        uint256 proxyDeployerNonce;
        address implementationDeployer;
        NetworkConfiguration[] networks;
    }

    /**
     * @dev Maps the network id to its migration state
     */
    mapping(uint16 => State) public states;

    // Computes the EIP-712 domain separador
    function _computeDomainSeparator(uint256 networkId, address addr) private pure returns (bytes32) {
        return keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256("Analog Gateway Contract"),
                keccak256("0.1.0"),
                uint256(networkId),
                address(addr)
            )
        );
    }

    function _toString(string memory a, uint256 b, string memory c, uint256 d) private pure returns (string memory) {
        return string(bytes.concat(
            bytes(a),
            bytes(vm.toString(b)),
            bytes(c),
            bytes(vm.toString(d))
        ));
    }

    function _toString(string memory a, address b, string memory c, address d) private pure returns (string memory) {
        return string(bytes.concat(
            bytes(a),
            bytes(vm.toString(b)),
            bytes(c),
            bytes(vm.toString(d))
        ));
    }

    /**
     * @dev Convert an address to string.
     */
    function _toString(string memory a, address b, string memory c) private pure returns (string memory) {
        return string(bytes.concat(bytes(a), bytes(vm.toString(b)), bytes(c)));
    }

    /**
     * @dev Find what nonce the deployer must use to deploy the contract at the specified address.
     */
    function _findDeployerNonce(address deployer, address contractAddress) private pure returns (uint64) {
        for (uint256 nonce = 0; nonce < 1000; nonce++) {
            // Obs: using this instead of `vm.computeCreateAddress(deployer, nonce);` to avoid `OutOfMemory` error.
            address addr = FACTORY.computeCreateAddress(deployer, nonce);
            if (contractAddress == addr) {
                return uint64(nonce);
            }
        }
        return type(uint64).max;
    }

    /**
     * @dev Retrieve network info and verify if the deployer is the admin of the proxy contract.
     */
    function _setupNetwork(NetworkConfiguration memory network, address proxyAddress, address proxyDeployer, address proxyAdmin)
        private
        view
    {
        network.chainID = block.chainid;

        // Check if the chain has a network id
        NetworkID networkId;
        {
            bool exists;
            (exists,networkId) = NetworkIDHelpers.tryFromChainID(block.chainid);
            require(exists, string(bytes.concat(bytes("network id not found for chain "), bytes(vm.toString(block.chainid)))));
        }

        //////////////////////////////////////////////////
        // Verify if the proxy is deployed and is valid //
        //////////////////////////////////////////////////
        network.hasProxy = proxyAddress.code.length > 0;
        if (network.hasProxy) {
            require(proxyAddress.codehash == PROXY_CODEHASH, "invalid proxy codehash");
        } else {
            console.log("PROXY NOT DEPLOYED");
        }

        ////////////////////////////////////////////////
        // Verify if the UNIVERSAL FACORY is deployed //
        ////////////////////////////////////////////////
        require(address(FACTORY).code.length > 0, "universal factory not deployed");
        require(address(FACTORY).codehash == FACTORY_CODEHASH, "invalid universal factory codehash");

        /////////////////////////
        // Retrieve Chain Info //
        /////////////////////////

        // Allocate the network information
        UpdateNetworkInfo memory info = network.info;
        info.networkId = networkId.asUint();
        info.domainSeparator = bytes32(0);
        info.gasLimit = 0;
        info.relativeGasPrice = UFloatMath.ONE;
        info.baseFee = 0;
        info.mortality = 0;
        console.log("        NETWORK ID", info.networkId);

        // Print information about the proxy contract
        if (network.hasProxy) {
            // Check if the gateway.networkId == info.networkId
            uint16 gatewayNetworkId = IGateway(proxyAddress).networkId();
            vm.assertEq(gatewayNetworkId, info.networkId, "network id mismatch");

            // Retrieve the proxy admin
            network.adminAddress = address(uint160(uint256(vm.load(proxyAddress, ERC1967.ADMIN_SLOT))));
            console.log("       PROXY_ADMIN", network.adminAddress);

            // Retrieve the current implementation
            network.implementationContract = address(uint160(uint256(vm.load(proxyAddress, ERC1967.IMPLEMENTATION_SLOT))));
            console.log("    IMPLEMENATTION", network.implementationContract);
        } else {
            console.log("       PROXY_ADMIN", "N/A");
            console.log("    IMPLEMENATTION", "N/A");
            network.adminAddress = address(0);
            network.implementationContract = address(0);
        }

        // Print information about the current network
        uint256 nonce = vm.getNonce(proxyDeployer);
        console.log("   GATEWAY BALANCE", proxyAddress.balance);
        console.log("  DEPLOYER BALANCE", proxyDeployer.balance);
        console.log("    DEPLOYER NONCE", nonce);
        console.log("     ADMIN BALANCE", proxyAdmin.balance);
        console.log("       ADMIN NONCE", vm.getNonce(proxyAdmin));
        console.log("      LATEST BLOCK", block.number);
        console.log("   BLOCK GAS LIMIT", block.gaslimit);
        console.log("          CHAIN ID", block.chainid);
        console.log("         GAS PRICE", tx.gasprice);
        console.log("          BASE FEE", block.basefee, "\n");

        if (network.hasProxy && network.adminAddress != proxyAdmin) {
            revert(_toString("proxy admin mismatch, got ", network.adminAddress, " but expected ", proxyAdmin));
        }
        require(block.gaslimit < uint64(type(int64).max), "block gas limit exceeds the limit of int64");
        require(block.gaslimit > 1_000_000, "block gas limit is too low");
        require(block.number < uint64(type(int64).max), "block number limit exceeds the limit of int64");
        require(block.number > 1_000_000, "block number is low, is this a local testnet?");

        // If this chain needs proxy, the deployer nonce must match the expected nonce.
        if (network.hasProxy == false) {
            address addr = vm.computeCreateAddress(proxyDeployer, nonce);
            if (addr != proxyAddress) {
                uint256 expected = _findDeployerNonce(proxyDeployer, proxyAddress);
                revert(_toString("Deployer nonce mismatch, got ", nonce, " but expected ", expected));
            }
        }

        // Update network information
        info.domainSeparator = _computeDomainSeparator(info.networkId, proxyAddress);
        info.gasLimit = uint64(block.gaslimit >> 1);
        info.relativeGasPrice = UFloatMath.ONE;
        info.baseFee = 0;
        info.mortality = uint64(block.number + 128);
    }

    function _setupNetworks(address proxyAddress, address proxyDeployer, address proxyAdmin) private returns (NetworkConfiguration[] memory networks) {
        string[2][] memory urls = vm.rpcUrls();
        require(urls.length > 0, "no rpc urls found, check the `foundry.toml` file");
        networks = new NetworkConfiguration[](urls.length);

        // Initialize and check the network information
        for (uint256 i = 0; i < urls.length; i++) {
            networks[i] = NetworkConfiguration({
                name: urls[i][0],
                forkID: 0,
                chainID: 0,
                hasProxy: false,
                adminAddress: address(0),
                implementationContract: address(0),
                info: UpdateNetworkInfo({
                    networkId: 0,
                    domainSeparator: bytes32(0),
                    gasLimit: 0,
                    relativeGasPrice: UFloatMath.ONE,
                    baseFee: 0,
                    mortality: 0
                })
            });
            NetworkConfiguration memory network = networks[i];
            string[2] memory entry = urls[i];
            console.log("        BLOCKCHAIN", entry[0]);
            console.log("           RPC URL", entry[1]);
            network.forkID = vm.createSelectFork(entry[1]);
            _setupNetwork(network, proxyAddress, proxyDeployer, proxyAdmin);
            console.log("");
        }
    }

    /**
     * @dev Script entry point, the following core will upgrade the gateway contract of all networks.
     */
    function run() external {
        Configuration memory config;
        {
            // Retrieve the `GatewayProxy` address
            address proxy = vm.envOr("PROXY_ADDRESS", address(0));
            if (proxy == address(0)) {
                proxy = vm.promptAddress("Enter the address of the proxy contract");
            }

            // Retrieve the account that must be used to deploy the proxy contract.
            address proxyDeployer = vm.envOr("PROXY_DEPLOYER", address(0));
            if (proxyDeployer == address(0)) {
                proxyDeployer = vm.promptAddress("Enter the address of the account that must be used to deploy the proxy contract");
            }
            require(msg.sender != proxyDeployer, "The account used to deploy the implementation and the proxy cannot be the same");

            // Find the nonce that must be used by the deployer to deploy the proxy contract
            uint64 proxyDeployerNonce = _findDeployerNonce(proxyDeployer, proxy);
            if (proxyDeployerNonce == type(uint64).max) {
                revert(_toString("The provided deployer ", proxyDeployer, " cannot deploy the proxy contract"));
            }
            
            // Retrieve the proxy admin account
            address proxyAdmin = vm.envOr("PROXY_ADMIN", DEFAULT_ADMIN_ACCOUNT);

            // Initialize Networks
            config = Configuration({
                proxy: proxy,
                proxyAdmin: proxyAdmin,
                proxyDeployer: proxyDeployer,
                proxyDeployerNonce: proxyDeployerNonce,
                implementationDeployer: msg.sender,
                networks: _setupNetworks(proxy, proxyDeployer, proxyAdmin)
            });
        }

        console.log("   FUNDING ACCOUNT", config.implementationDeployer);
        console.log("     PROXY_ADDRESS", config.proxy);
        console.log("    PROXY DEPLOYER", config.proxyDeployer);
        console.log("     ADMIN ACCOUNT", config.proxyAdmin, "\n");

        // Iterate over all the RPC URLs, defined in the `foundry.toml` file
        NetworkConfiguration[] memory allNetworks = config.networks;
        
        // Filter the networks that need a proxy
        NetworkConfiguration[] memory needsProxy = new NetworkConfiguration[](allNetworks.length);
        {
            uint256 count = 0;
            for (uint256 i=0; i<allNetworks.length; i++) {
                NetworkConfiguration memory network = allNetworks[i];
                if (network.hasProxy == false) {
                    needsProxy[count++] = network;
                }
            }
            assembly {
                mstore(needsProxy, count)
            }
            
            if (count == 0) {
                console.log("\nTHE PROXY IS ALREADY DEPLOYED IN ALL PROVIDED NETWORKS\n");
                return;
            }
        }

        console.log(" ------------ SENDS FUNDS TO PROXY DEPLOYER ------------- ");
        // Validate the proxy deployment account balance and nonce
        // Send funds to the deployer account if necessary.
        for (uint256 i = 0; i < needsProxy.length; i++) {
            NetworkConfiguration memory network = needsProxy[i];
            console.log("     -- BLOCKCHAIN", network.name);
            
            // Switch to the selected network
            vm.selectFork(network.forkID);

            // Check if the deployer nonce matches the expected nonce.
            address deployer = config.proxyDeployer;
            uint256 nonce = vm.getNonce(config.proxyDeployer);
            uint256 balance = config.proxyDeployer.balance;
            console.log("    PROXY DEPLOYER", deployer);
            console.log("  DEPLOYER BALANCE", balance);
            console.log("    DEPLOYER NONCE", nonce);
            if (nonce != config.proxyDeployerNonce) {
                revert(_toString("Deployer nonce mismatch, got ", nonce, " but expected ", config.proxyDeployerNonce));
            }
            
            // Send funds to the deployer account
            if (balance < MINIMAL_DEPLOYER_BALANCE) {
                require(deployer != msg.sender, "deployer account and funding account cannot be the same");
                console.log("   FUNDING ACCOUNT", msg.sender);
                console.log("   FUNDING BALANCE", msg.sender.balance);
                console.log("     FUNDING NONCE", vm.getNonce(msg.sender));
                require(msg.sender.balance > MINIMAL_DEPLOYER_BALANCE, "insufficient funds");

                vm.startBroadcast(msg.sender);
                payable(deployer).transfer(MINIMAL_DEPLOYER_BALANCE - balance);
                vm.stopBroadcast();
            }
            console.log("");
        }

        console.log(" --------------- DEPLOYING IMPLEMENTATION --------------- ");
        {
            bytes memory bytecode = type(Gateway).creationCode;
            console.log("         IMPL HASH", vm.toString(bytes32(keccak256(bytecode))), "\n");
            for (uint256 i = 0; i < needsProxy.length; i++) {
                NetworkConfiguration memory network = needsProxy[i];

                // Switch to the selected network
                vm.selectFork(network.forkID);
                require(network.chainID == block.chainid, "chain id mismatch");

                // Print information about the current network
                console.log("     -- BLOCKCHAIN", network.name);
                console.log("          CHAIN ID", block.chainid);
                console.log("        NETWORK ID", network.info.networkId);
                console.log("      BLOCK NUMBER", block.number);

                // Check if the the implementation is already deployed.
                bytes memory initCode = bytes.concat(
                    bytecode,
                    abi.encode(uint16(network.info.networkId), address(config.proxy))
                );
                address deployer = config.implementationDeployer;
                network.implementationContract = FACTORY.computeCreate2Address(IMPLEMENTATION_SALT, initCode);
                console.log("          DEPLOYER", deployer);
                console.log("  DEPLOYER BALANCE", deployer.balance);
                console.log("  CONTRACT ADDRESS", network.implementationContract);

                if (network.implementationContract.code.length == 0) {
                    // Deploy the implementation contract
                    vm.startBroadcast(deployer);
                    address implementation = FACTORY.create2(IMPLEMENTATION_SALT, initCode);
                    vm.stopBroadcast();
                    vm.assertEq(network.implementationContract, implementation, "implementation address mismatch");
                    console.log(" DEPLOYMENT STATUS", "Deployed");
                } else {
                    console.log(" DEPLOYMENT STATUS", "Skipped, already deployed");
                }
                console.log();
            }
        }

        console.log(" -------------------- DEPLOYING PROXY ------------------- ");
        for (uint256 i = 0; i < needsProxy.length; i++) {
            NetworkConfiguration memory network = needsProxy[i];

            // Switch to the selected network
            vm.selectFork(network.forkID);
            require(network.chainID == block.chainid, "chain id mismatch");

            console.log("     -- BLOCKCHAIN", network.name);
            console.log("          CHAIN ID", block.chainid);
            console.log("        NETWORK ID", network.info.networkId);
            console.log("      BLOCK NUMBER", block.number);
            
            address deployer = config.proxyDeployer;
            address implementation = network.implementationContract;
            uint256 nonce = vm.getNonce(deployer);
            console.log("  DEPLOYER ACCOUNT", deployer);
            console.log("  DEPLOYER BALANCE", deployer.balance);
            console.log("    DEPLOYER NONCE", nonce);
            console.log("    IMPLEMENTATION", implementation);
            console.log("     PROXY ADDRESS", config.proxy);
            require(config.proxy.code.length == 0, "proxy already deployed");
            require(deployer.balance >= MINIMAL_DEPLOYER_BALANCE, "deployer insufficient funds");
            require(nonce == config.proxyDeployerNonce, "wrong deployer nonce");
            require(implementation.code.length > 0, "implementation not found");

            // TODO: Load the shards from the network, currently only the admin can add the shards.
            TssKey[] memory emptyShards = new TssKey[](0);
            Network[] memory emptyNetworks = new Network[](0);
            bytes memory initializer = abi.encodeCall(Gateway.initialize, (config.proxyAdmin, emptyShards, emptyNetworks));

            vm.startBroadcast(deployer);
            // address deployed = address(new GatewayProxy(implementation, initializer));
            address deployed = address(new GatewayProxy(config.proxyAdmin));  // TODO: fix me
            vm.stopBroadcast();
            console.log("     PROXY ADDRESS", deployed);
            console.log(" DEPLOYMENT STATUS", deployed == config.proxy ? "Success" : "Address Mismatch");
            console.log("");
        }
    }
}
