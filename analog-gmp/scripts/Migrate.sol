// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (scripts/Upgrade.sol)

pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {ERC1967} from "../src/utils/ERC1967.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {Gateway} from "../src/Gateway.sol";
import {
    TssKey,
    GmpMessage,
    UpdateKeysMessage,
    UpdateNetworkInfo,
    Signature,
    Network,
    GmpStatus,
    GmpSender,
    PrimitiveUtils
} from "../src/Primitives.sol";

contract MigrateGateway is Script {
    bytes32 internal constant PROXY_CODEHASH = 0x54afeb06256bce71659256132ac18f1515de3011aaec4fbd6fc7b0c00c7263d8;

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

    /**
     * @dev Retrieve network info and verify if the deployer is the admin of the proxy contract.
     */
    function _setupNetwork(string memory name, address proxyAddress, address deployer)
        private
        returns (UpdateNetworkInfo memory info)
    {
        console.log(string(bytes.concat(" -- CHECKING ", bytes(name))));

        // Retrieve the RPC URL
        string memory rpcUrl = vm.envOr(name, string(""));
        require(bytes(rpcUrl).length > 0, "rpc url not found");
        console.log("               RPC", rpcUrl);

        // Create a new fork
        uint256 forkId = vm.createSelectFork(rpcUrl);

        // Verify if the provided proxy address is valid
        {
            require(proxyAddress.code.length > 0, "UpgradeGateway: proxy doesn't exists");
            bytes32 codehash;
            assembly {
                codehash := extcodehash(proxyAddress)
            }
            require(codehash == PROXY_CODEHASH, "UpgradeGateway: invalid proxy codehash");
        }

        // Allocate the network information
        info = UpdateNetworkInfo({
            networkId: 0,
            domainSeparator: bytes32(0),
            gasLimit: 0,
            relativeGasPrice: UFloatMath.ONE,
            baseFee: 0,
            mortality: 0
        });

        // Retrieve the network id
        info.networkId = IGateway(proxyAddress).networkId();
        console.log("        NETWORK ID", info.networkId);

        // Retrieve the proxy admin
        address admin = address(uint160(uint256(vm.load(proxyAddress, ERC1967.ADMIN_SLOT))));
        console.log("       PROXY_ADMIN", admin);

        // Retrieve the current implementation
        address implementation = address(uint160(uint256(vm.load(proxyAddress, ERC1967.IMPLEMENTATION_SLOT))));
        console.log("    IMPLEMENATTION", implementation);

        // Print information about the current network
        console.log("   GATEWAY BALANCE", proxyAddress.balance);
        console.log("  DEPLOYER BALANCE", deployer.balance);
        console.log("      LATEST BLOCK", block.number);
        console.log("   BLOCK GAS LIMIT", block.gaslimit);
        console.log("          CHAIN ID", block.chainid);
        console.log("         GAS PRICE", tx.gasprice);
        console.log("          BASE FEE", block.basefee, "\n");

        require(admin == deployer, "deployer is not the admin if this contract");
        require(block.gaslimit < uint64(type(int64).max), "block gas limit exceeds the limit of int64");
        require(block.gaslimit > 1_000_000, "block gas limit is too low");
        require(block.number < uint64(type(int64).max), "block number limit exceeds the limit of int64");
        require(block.number > 1_000_000, "block number is low, is this a local testnet?");

        // Update network information
        info.domainSeparator = _computeDomainSeparator(info.networkId, proxyAddress);
        info.gasLimit = uint64(block.gaslimit >> 1);
        info.relativeGasPrice = UFloatMath.ONE;
        info.baseFee = 0;
        info.mortality = uint64(block.number + 128);

        // Save migration state information
        states[info.networkId] = State({forkId: forkId, mortality: info.mortality, proxyAddress: proxyAddress});
    }

    /**
     * @dev Verify the networks and check if the deployer is the admin of the proxy contract
     */
    function _setupNetworks(address proxyAddress, address deployer)
        private
        returns (UpdateNetworkInfo[] memory networks)
    {
        networks = new UpdateNetworkInfo[](3);
        networks[0] = _setupNetwork("SEPOLIA_RPC_URL", proxyAddress, deployer);

        networks[1] = _setupNetwork("SHIBUYA_RPC_URL", proxyAddress, deployer);
        require(networks[0].networkId != networks[1].networkId, "SEPOLIA and SHIBUYA have the same network id");

        networks[2] = _setupNetwork("POLYGON_AMOY_RPC_URL", proxyAddress, deployer);
        require(networks[2].networkId != networks[0].networkId, "AMOY and SEPOLIA have the same network id");
        require(networks[2].networkId != networks[0].networkId, "AMOY and SHIBUYA have the same network id");
    }

    /**
     * @dev Deploy the new Gateway implementation and upgrade the proxy contract
     */
    function _upgradeNetwork(uint16 networkId, uint256 deployerPrivateKey, UpdateNetworkInfo[] memory networks)
        private
    {
        State memory state = states[networkId];

        // Switch to the fork
        vm.selectFork(state.forkId);

        // Check if the implementation expected address
        {
            // Retrieve the deployer nonce
            address deployer = vm.addr(deployerPrivateKey);
            uint256 nonce = vm.getNonce(deployer);
            address implementation = vm.computeCreateAddress(deployer, nonce);
            console.log(" NEW IMPLEMENATTION", implementation);
        }

        // Update message mortality
        for (uint256 i = 0; i < networks.length; i++) {
            networks[i].mortality = state.mortality;
        }

        // Deploy the new implementation contract
        vm.startBroadcast(deployerPrivateKey);
        Gateway newImplementation = new Gateway(networkId, state.proxyAddress);
        console.log("             DEPLOYED", address(newImplementation));

        bytes memory initializer = abi.encodeCall(Gateway.updateNetworks, (networks));
        console.log("          INITIALIZER:");
        console.logBytes(initializer);
        Gateway(state.proxyAddress).upgradeAndCall(address(newImplementation), initializer);
        console.log("     GATEWAY UPGRADED");
        vm.stopBroadcast();
    }

    /**
     * @dev Script entry point, the following core will upgrade the gateway contract of all networks.
     */
    function run() external {
        // Retrieve the gateway proxy address
        address proxyAddress = vm.envAddress("PROXY_ADDRESS");
        console.log("     PROXY_ADDRESS", proxyAddress);

        // Retrieve deployer private key
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        console.log("          DEPLOYER", deployer, "\n");

        // Setup the networks
        UpdateNetworkInfo[] memory networkInfos = _setupNetworks(proxyAddress, deployer);

        // Extract the network ids
        uint16[] memory networkByID = new uint16[](networkInfos.length);
        for (uint256 i = 0; i < networkInfos.length; i++) {
            networkByID[i] = networkInfos[i].networkId;
        }

        // Upgrade the networks
        for (uint256 i = 0; i < networkByID.length; i++) {
            uint16 networkID = networkByID[i];
            console.log(" -- UPGRADING NETWORK", networkID);
            _upgradeNetwork(networkID, deployerPrivateKey, networkInfos);
        }
    }
}
