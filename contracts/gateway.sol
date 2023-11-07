// SPDX-License-Identifier: MIT
pragma solidity >=0.4.22;

contract Gateway {
    // TODO: need a local field for public_keys which has key: shard_id and value publickey
    // shardid: u64, publickey: Vec<u8>
    function init(uint64, uint[] memory) public {
        // insert_public_key(uint64, uint[] memory);
    }
    // shardid: u64, bytes: Vec<u8>, TssSignature signature
    function submit(uint64, uint[] memory, uint[] memory) public {
        // pk = get_public_key(shardid);
        // verify_signature(pk, uint[] memory, uint[] memory));
        // TODO: need Event Enum type that initializes from uint[] memory
        // match bytes.parse() {
        //     RegisterShard(shardid, pubkey) => insert_public_key(shardid, pubkey),
        //     UnregisterShard(shardid) => remove_public_key(shardid),
        //     SendMessage(address, payload) => send_message(address, payload),
        // }
    }
    // message: Vec<u8>, TssSignature inputs
    function send_message(uint[] memory, uint[] memory) private {}
}
