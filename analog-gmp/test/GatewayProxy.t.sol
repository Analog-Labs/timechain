// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (test/Gateway.t.sol)

pragma solidity >=0.8.0;

import {IUniversalFactory} from "../lib/universal-factory/src/IUniversalFactory.sol";
import {FactoryUtils} from "../lib/universal-factory/src/FactoryUtils.sol";
import {Test, console} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {TestUtils, SigningKey, SigningUtils} from "./TestUtils.sol";
import {GasSpender} from "./utils/GasSpender.sol";
import {Gateway, GatewayEIP712} from "../src/Gateway.sol";
import {GatewayProxy} from "../src/GatewayProxy.sol";
import {GasUtils} from "../src/utils/GasUtils.sol";
import {BranchlessMath} from "../src/utils/BranchlessMath.sol";
import {UFloat9x56, UFloatMath} from "../src/utils/Float9x56.sol";
import {IGateway} from "../src/interfaces/IGateway.sol";
import {IGmpReceiver} from "../src/interfaces/IGmpReceiver.sol";
import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {
    GmpMessage,
    UpdateKeysMessage,
    Signature,
    TssKey,
    Network,
    GmpStatus,
    PrimitiveUtils,
    GmpSender
} from "../src/Primitives.sol";

contract GatewayProxyTest is Test {
    using PrimitiveUtils for UpdateKeysMessage;
    using PrimitiveUtils for GmpMessage;
    using PrimitiveUtils for GmpSender;
    using PrimitiveUtils for address;
    using BranchlessMath for uint256;
    using SigningUtils for SigningKey;
    using FactoryUtils for IUniversalFactory;

    /**
     * @dev The address of the `UniversalFactory` contract, must be the same on all networks.
     */
    IUniversalFactory private constant FACTORY = IUniversalFactory(0x0000000000001C4Bf962dF86e38F0c10c7972C6E);
    // VmSafe.Wallet private proxyAdmin;
    // Gateway private gateway;

    // Chronicle TSS Secret
    // uint256 private constant ADMIN_SECRET = 0x42;
    uint256 private constant SIGNING_NONCE = 0x69;

    // Route IDS
    uint16 private constant SRC_NETWORK_ID = 1234;
    uint16 private constant DEST_NETWORK_ID = 1337;

    /**
     * @dev his is a special contract that implements the IGmpReceiver interface and wastes an exact amount of gas you send to it, helpful
     * for testing GMP refunds and gas limits.
     * See the file `GasSpender` contract for more details.
     */
    IGmpReceiver internal receiver;

    constructor() {
        require(FACTORY == TestUtils.deployFactory(), "factory address mismatch");
        receiver = IGmpReceiver(new GasSpender());
    }

    function setUp() external view {
        // check block gas limit as gas left
        assertEq(block.gaslimit, 30_000_000);
        assertTrue(gasleft() >= 10_000_000);
    }

    function _setup(VmSafe.Wallet memory admin, bytes32 salt, uint16 routeID) private returns (Gateway gateway) {
        ///////////////////////////////////////////
        // 1. Deploy the implementation contract //
        ///////////////////////////////////////////
        // 1.1 Compute the `GatewayProxy` address
        bytes memory proxyCreationCode = abi.encodePacked(type(GatewayProxy).creationCode, abi.encode(admin.addr));
        address proxyAddr = FACTORY.computeCreate2Address(salt, proxyCreationCode);

        // 1.2 Deploy the `Gateway` implementation contract
        bytes memory implementationCreationCode =
            abi.encodePacked(type(Gateway).creationCode, abi.encode(routeID, proxyAddr));
        address payable implementation = payable(FACTORY.create2(salt, implementationCreationCode, abi.encode(routeID)));
        assertEq(Gateway(implementation).networkId(), routeID);

        ////////////////////////////////////////////////////////
        // 2. ProxyAdmin approves the implementation contract //
        ////////////////////////////////////////////////////////
        bytes memory authorization;
        {
            // This allows anyone to deploy the Proxy.
            bytes32 digest = keccak256(abi.encode(proxyAddr, address(implementation)));
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(admin.privateKey, digest);
            authorization = abi.encode(v, r, s, address(implementation));
        }

        ////////////////////////////////////////////////////////////////
        // 3 - Deploy the `GatewayProxy` using the `UniversalFactory` //
        ////////////////////////////////////////////////////////////////
        SigningKey memory signer = TestUtils.createSigner(admin.privateKey);
        TssKey[] memory keys = new TssKey[](1);
        keys[0] = TssKey({yParity: signer.yParity() == 28 ? 1 : 0, xCoord: signer.xCoord()}); // Shard key
        Network[] memory networks = new Network[](2);
        networks[0].id = SRC_NETWORK_ID; // sepolia network id
        networks[0].gateway = proxyAddr; // sepolia proxy address
        networks[1].id = DEST_NETWORK_ID; // shibuya network id
        networks[1].gateway = proxyAddr; // shibuya proxy address

        // Initializer, used to initialize the Gateway contract
        bytes memory initializer = abi.encodeCall(Gateway.initialize, (admin.addr, keys, networks));
        address payable gatewayAddr = payable(FACTORY.create2(salt, proxyCreationCode, authorization, initializer));
        gateway = Gateway(gatewayAddr);

        // Send funds to the gateway contract
        vm.deal(address(gateway), 100 ether);
    }

    function test_deployProxy() external {
        VmSafe.Wallet memory admin = vm.createWallet(vm.randomUint());
        TestUtils.setupGateway(admin, bytes32(uint256(1234)), SRC_NETWORK_ID, DEST_NETWORK_ID);
    }
}
