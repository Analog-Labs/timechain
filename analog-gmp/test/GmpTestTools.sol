// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/GmpTestTools.sol)

pragma solidity >=0.8.0;

import {VmSafe, Vm} from "forge-std/Vm.sol";
import {TestUtils, SigningKey, SigningUtils} from "./TestUtils.sol";
import {Random} from "./Random.sol";
import {Gateway} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {GmpMessage, TssKey, Network, Signature, GmpSender, PrimitiveUtils} from "../src/Primitives.sol";
import {IUniversalFactory} from "../lib/universal-factory/src/IUniversalFactory.sol";

library GmpTestTools {
    /**
     * @dev Forge Cheat Code VM address, 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D.
     */
    address private constant VM_ADDRESS = address(uint160(uint256(keccak256("hevm cheat code"))));
    Vm private constant vm = Vm(VM_ADDRESS);

    // Sepolia Properties
    Gateway internal constant SEPOLIA_GATEWAY = Gateway(payable(0x000000007f56768De3133034fa730a909003A166));
    uint16 internal constant SEPOLIA_NETWORK_ID = 5;
    bytes32 internal constant SEPOLIA_SHARD_SECRET = keccak256("analog.sepolia.shard.secret");
    bytes32 internal constant SEPOLIA_DOMAIN_SEPARATOR = keccak256(
        abi.encode(
            keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
            keccak256("Analog Gateway Contract"),
            keccak256("0.1.0"),
            uint256(SEPOLIA_NETWORK_ID),
            address(SEPOLIA_GATEWAY)
        )
    );

    // Shibuya Properties
    Gateway internal constant SHIBUYA_GATEWAY = Gateway(payable(0x000000007f56768DE3133034fA730A909003a167));
    uint16 internal constant SHIBUYA_NETWORK_ID = 7;
    bytes32 internal constant SHIBUYA_SHARD_SECRET = keccak256("analog.shibuya.shard.secret");
    bytes32 internal constant SHIBUYA_DOMAIN_SEPARATOR = keccak256(
        abi.encode(
            keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
            keccak256("Analog Gateway Contract"),
            keccak256("0.1.0"),
            uint256(SHIBUYA_NETWORK_ID),
            address(SHIBUYA_GATEWAY)
        )
    );

    /**
     * @dev Minimal Eip1667 proxy bytecode.
     */
    bytes private constant _PROXY_BYTECODE =
        hex"363d3d373d3d3d363d7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc545af43d82803e903d91603857fd5bf3";

    /**
     * @dev Storage slot with the address of the current implementation.
     * This is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1.
     */
    bytes32 private constant _IMPLEMENTATION_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    /**
     * @dev Log index storage slot.
     * This prevents a given message from being execute more than once.
     */
    bytes32 private constant _LOG_INDEX_SLOT = bytes32(uint256(keccak256("analog.GmpTestTools.logIndex")) - 1);

    /**
     * @dev Mapping of network ID to fork ID.
     */
    bytes32 private constant _FORKS_SLOT = bytes32(uint256(keccak256("analog.GmpTestTools.forks")) - 1);

    /**
     * @dev Storage slot.
     */
    function setup() internal {
        require(vm.isPersistent(address(this)), "GmpTestTools must only be called from Test contract");
        // Create forks
        uint256 sepoliaForkID = vm.createFork("https://sepolia.infura.io/v3/b9794ad1ddf84dfb8c34d6bb5dca2001", 5714300);
        uint256 shibuyaForkID = vm.createFork("https://evm.shibuya.astar.network", 6102790);

        // Save the fork IDs
        _storeForkID(SEPOLIA_NETWORK_ID, sepoliaForkID);
        _storeForkID(SHIBUYA_NETWORK_ID, shibuyaForkID);

        // Deploy the gateways
        Network[] memory networks = new Network[](2);
        networks[0] = Network({id: SEPOLIA_NETWORK_ID, gateway: address(SEPOLIA_GATEWAY)});
        networks[1] = Network({id: SHIBUYA_NETWORK_ID, gateway: address(SHIBUYA_GATEWAY)});

        // Setup the networks
        require(switchNetwork(SEPOLIA_NETWORK_ID) == sepoliaForkID, "unexpected sepolia fork id");
        setupNetwork(SEPOLIA_NETWORK_ID, address(SEPOLIA_GATEWAY), SEPOLIA_SHARD_SECRET, networks);

        require(switchNetwork(SHIBUYA_NETWORK_ID) == shibuyaForkID, "unexpected shibuya fork id");
        setupNetwork(SHIBUYA_NETWORK_ID, address(SHIBUYA_GATEWAY), SHIBUYA_SHARD_SECRET, networks);

        // Record logs must be enabled to allow this tool to retrieve the GMP messages
        vm.recordLogs();
    }

    function setupNetwork(uint16 networkId, address gateway, bytes32 secret, Network[] memory networks) internal {
        // Deploy `Universal Factory` contract
        IUniversalFactory factory = TestUtils.deployFactory();

        SigningKey memory signer = TestUtils.signerFromEntropy(secret);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: uint8(signer.pubkey.py % 2), xCoord: signer.pubkey.px});

        // Check if the gateway is already deployed
        bool exists = gateway.code.length > 0;

        // Deploy the gateway proxy
        bytes memory implementationCreationCode = abi.encodePacked(type(Gateway).creationCode, abi.encode(gateway));

        // address implementation = address(new Gateway(networkId, gateway));
        address implementation;
        bytes32 salt = bytes32(0);
        if (exists) {
            implementation = factory.create2(salt, implementationCreationCode, "");
        } else {
            implementation = factory.create2(salt, implementationCreationCode, abi.encode(networkId, address(gateway)));
        }
        vm.etch(gateway, _PROXY_BYTECODE);
        vm.store(gateway, _IMPLEMENTATION_SLOT, bytes32(uint256(uint160(implementation))));

        // If the gateway is already deployed, just register the shard
        // This is useful when using forked networks
        if (exists) {
            bytes32 prevMessageHash = vm.load(gateway, bytes32(uint256(4)));
            if (prevMessageHash != bytes32(0)) {
                registerShard(gateway, signer);
                revert("ALREADY INITIALIZED");
            }
        }

        // Change caller mode because only the gateway can initialize itself
        (VmSafe.CallerMode callerMode, address msgSender, address txOrigin) =
            TestUtils.setCallerMode(VmSafe.CallerMode.Prank, gateway, gateway);

        // Initialize the gateway
        Gateway(payable(gateway)).initialize(msgSender, keys, networks);

        // Restore previous caller mode
        TestUtils.setCallerMode(callerMode, msgSender, txOrigin);
    }

    function deal(address account, uint256 newBalance) internal {
        // If the account is persistent, just need to deal once
        if (vm.isPersistent(account)) {
            vm.deal(account, newBalance);
            return;
        }
        // Select sepolia and execute callback
        switchNetwork(SEPOLIA_NETWORK_ID);
        vm.deal(account, newBalance);

        // Select shibuya and execute callback
        switchNetwork(SHIBUYA_NETWORK_ID);
        vm.deal(account, newBalance);
    }

    /**
     * @dev Execute all pending GMP messages.
     */
    function relayMessages() internal {
        uint256 activeFork = vm.activeFork();
        GmpMessage[] memory allMessages = messages();
        _executeMessages(SHIBUYA_GATEWAY, SHIBUYA_NETWORK_ID, SHIBUYA_SHARD_SECRET, allMessages);
        _executeMessages(SEPOLIA_GATEWAY, SEPOLIA_NETWORK_ID, SEPOLIA_SHARD_SECRET, allMessages);
        vm.selectFork(activeFork);
    }

    /**
     * @dev Switch to `network` fork id.
     */
    function switchNetwork(uint16 network) internal returns (uint256 forkId) {
        forkId = _loadForkID(network);
        require(forkId != uint256(_FORKS_SLOT), "GmpTestTools: network not found");
        vm.selectFork(forkId);
    }

    /**
     * @dev Switch to the `network` fork id and sets all subsequent calls' `msg.sender` to `msgSender`.
     */
    function switchNetwork(uint16 network, address msgSender) internal returns (uint256 forkId) {
        forkId = switchNetwork(network);
        vm.stopPrank();
        vm.startPrank(msgSender, msgSender);
    }

    /**
     * @dev Stores the network fork id.
     */
    function _storeForkID(uint16 network, uint256 forkId) private {
        bytes32 slot = _deriveMapping(_FORKS_SLOT, uint256(network));
        // Once zero is a valid fork id, we XOR before storing to prevent
        // an invalid network from returning a valid fork id
        _sstoreUint256(slot, forkId ^ uint256(_FORKS_SLOT));
    }

    /**
     * @dev Load the fork id of a given `network`
     */
    function _loadForkID(uint16 network) private view returns (uint256) {
        bytes32 slot = _deriveMapping(_FORKS_SLOT, uint256(network));
        // Once zero is a valid fork id, we XOR the returned result to prevent
        // an invalid network from returning a valid fork id
        return _sloadUint256(slot) ^ uint256(_FORKS_SLOT);
    }

    /**
     * @dev Returns the `uint256` located at `slot`.
     */
    function _sloadUint256(bytes32 slot) private view returns (uint256 r) {
        assembly {
            r := sload(slot)
        }
    }

    /**
     * @dev Store `value` at `slot`.
     */
    function _sstoreUint256(bytes32 slot, uint256 value) private {
        assembly {
            sstore(slot, value)
        }
    }

    /**
     * @dev Force register a new shard in the gateway.
     */
    function registerShard(address gateway, SigningKey memory shard) internal {
        // uint256 shardInfo = 1 | (shard.pubkey.py % 2);
        bytes32 slot = _deriveMapping(bytes32(0), shard.pubkey.px);
        uint256 shardInfo = uint256(vm.load(gateway, slot));
        uint256 nonce = shardInfo >> 224;
        nonce = BranchlessMath.ternary(nonce > 0, nonce, 1);
        shardInfo = (nonce << 224) | (1 << 216) | ((shard.pubkey.py % 2) << 217);
        vm.store(gateway, slot, bytes32(shardInfo));
    }

    /**
     * @dev Derive the location of a mapping element from the key.
     */
    function _deriveMapping(bytes32 slot, uint256 key) private pure returns (bytes32 result) {
        assembly ("memory-safe") {
            mstore(0x00, key)
            mstore(0x20, slot)
            result := keccak256(0x00, 0x40)
        }
    }

    /**
     * @dev Retrieve all pending messages from the recorded logs
     */
    function messages() internal returns (GmpMessage[] memory gmpMessages) {
        bytes32[] memory topics = new bytes32[](1);
        topics[0] = IGateway.GmpCreated.selector;
        Vm.Log[] memory logs = vm.getRecordedLogs();
        uint256 logIndex = _sloadUint256(_LOG_INDEX_SLOT);
        gmpMessages = new GmpMessage[](logs.length - logIndex);
        uint256 pos = 0;
        for (uint256 i = logIndex; i < logs.length; i++) {
            Vm.Log memory log = logs[i];

            // Filter emitters
            uint16 srcNetwork;
            if (log.emitter == address(SEPOLIA_GATEWAY)) {
                srcNetwork = SEPOLIA_NETWORK_ID;
            } else if (log.emitter == address(SHIBUYA_GATEWAY)) {
                srcNetwork = SHIBUYA_NETWORK_ID;
            } else {
                continue;
            }

            // Filter topics
            if (log.topics.length != 4 || log.topics[0] != IGateway.GmpCreated.selector) {
                continue;
            }

            // Decode the GMP message
            (uint16 destNetwork, uint64 gasLimit, uint64 nonce, bytes memory data) =
                abi.decode(log.data, (uint16, uint64, uint64, bytes));
            gmpMessages[pos++] = GmpMessage({
                source: GmpSender.wrap(log.topics[2]),
                srcNetwork: srcNetwork,
                dest: address(uint160(uint256(log.topics[3]))),
                destNetwork: destNetwork,
                gasLimit: gasLimit,
                nonce: nonce,
                data: data
            });
        }
        _sstoreUint256(_LOG_INDEX_SLOT, logs.length);
    }

    function _executeMessages(Gateway gateway, uint16 network, bytes32 secret, GmpMessage[] memory gmpMessages)
        private
    {
        switchNetwork(network);
        SigningKey memory signer = TestUtils.signerFromEntropy(secret);

        for (uint256 i = 0; i < gmpMessages.length; i++) {
            GmpMessage memory message = gmpMessages[i];

            // Compute the message ID
            bytes32 messageID = PrimitiveUtils.eip712hash(message);

            // Skip if the message is not intended for this network
            if (message.destNetwork != network) {
                continue;
            }

            // Sign the message
            (uint256 c, uint256 z) = SigningUtils.signPrehashed(signer, messageID, Random.nextUint());
            Signature memory signature = Signature({xCoord: signer.pubkey.px, e: c, s: z});

            // Execute the message
            gateway.execute(signature, message);
        }
    }
}
