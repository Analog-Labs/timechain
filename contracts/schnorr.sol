//SPDX-License-Identifier: LGPLv3
pragma solidity ^0.8.0;

contract Schnorr {
  // secp256k1 group order
  uint256 constant public Q =
    0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

  // parity := public key y-coord parity (27 or 28)
  // px := public key x-coord
  // message := 32-byte message
  // e := schnorr signature challenge
  // s := schnorr signature
  function verify(
    uint8 parity,
    uint256 px,
    uint256 message,
    uint256 e,
    uint256 s
  ) public pure returns (bool) {
    // ecrecover = (m, v, r, s);
    uint256 sp = Q - mulmod(s, px, Q);
    uint256 ep = Q - mulmod(e, px, Q);

    require(sp != 0);
    // the ecrecover precompile implementation checks that the `r` and `s`
    // inputs are non-zero (in this case, `px` and `ep`), thus we don't need to
    // check if they're zero.
    address R = ecrecover(bytes32(sp), parity, bytes32(px), bytes32(ep));
    require(R != address(0), "ecrecover failed");
    return bytes32(e) == keccak256(
      abi.encodePacked(R, parity, px, message)
    );
  }
}