// SPDX-License-Identifier: MIT
// Analog's Contracts (last updated v0.1.0) (src/utils/Schnorr.sol)

pragma solidity >=0.8.20;

library Schnorr {
    // secp256k1 group order
    uint256 internal constant Q = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

    /**
     * Verify Schnorr signature (secp256k1) without memory allocation, Solidity's `ecrecover`
     * allocates memory, which complicates the gas estimation.
     *
     * @param parity public key y-coord parity (27 or 28)
     * @param px public key x-coord
     * @param message 32-byte message hash
     * @param e schnorr signature challenge
     * @param s schnorr signature
     */
    function verify(uint8 parity, uint256 px, uint256 message, uint256 e, uint256 s)
        internal
        view
        returns (bool valid)
    {
        // the ecrecover precompile implementation checks that the `r` and `s`
        // inputs are non-zero (in this case, `px` and `ep`), thus we don't need to
        // check if they're zero.
        assembly ("memory-safe") {
            // backup the memory values for restore later
            let b0 := mload(0x40)
            let b1 := mload(0x60)
            {
                // sp = Q - mulmod(s, px, Q)
                let sp := sub(Q, mulmod(s, px, Q))

                // ep = Q - mulmod(e, px, Q)
                let ep := sub(Q, mulmod(e, px, Q))

                // R = ecrecover(bytes32 hash, uint8 v, bytes32 r, bytes32 s)
                mstore(0x00, sp)
                mstore(0x20, parity)
                mstore(0x40, px)
                mstore(0x60, ep)
                pop(staticcall(gas(), 1, 0x00, 0x80, 0x00, 0x20))
                let R := mload(0x00)

                // Compute keccak256(abi.encodePacked(R, parity, px, message))
                mstore(0x20, shl(248, parity))
                mstore(0x21, px)
                mstore(0x41, message)

                // sp != 0 && R != 0
                valid := and(gt(sp, 0), gt(R, 0))
                // R == keccak256(abi.encodePacked(R, parity, px, message)
                valid := and(valid, eq(e, keccak256(0x0c, 85)))
            }
            // restore the original memory values
            mstore(0x40, b0)
            mstore(0x60, b1)
        }
    }
}
