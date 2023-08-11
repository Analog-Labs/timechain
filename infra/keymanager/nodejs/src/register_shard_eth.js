import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';
import { stringToU8a, u8aToHex } from '@polkadot/util';
import { Channel } from 'async-channel';

const wsProvider = new WsProvider('ws://bootnode:9944');
const api = await ApiPromise.create({
    provider: wsProvider,
    rpc: {
        time: {
            submitForSigning: {
                description: 'Submit SignPayload for signing alongside given signature',
                params: [
                    {
                        name: 'group_id',
                        type: 'u64'
                    },
                    {
                        name: 'message',
                        type: 'String'
                    },
                    {
                        name: 'signature',
                        type: 'String'
                    }
                ],
                type: 'RpcResult<((), u32)>'
            }
        }
    }
});

const time1 = [76, 181, 171, 246, 173, 121, 251, 245, 171, 188, 202, 252, 194, 105, 216, 92, 210, 101, 30, 212, 184, 133, 181, 134, 159, 36, 26, 237, 240, 165, 186, 41];
const time2 = [116, 34, 185, 136, 117, 152, 6, 142, 50, 196, 68, 138, 148, 154, 219, 41, 13, 15, 78, 53, 185, 224, 27, 14, 229, 241, 161, 230, 0, 254, 38, 116];
const time3 = [243, 129, 98, 110, 65, 231, 2, 126, 164, 49, 191, 227, 0, 158, 148, 189, 210, 90, 116, 107, 238, 196, 104, 148, 141, 108, 60, 124, 93, 201, 165, 75];
const collector_pubkey = { "sr25519": "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d" };
const phrase = "//Alice";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

const chan = new Channel(0 /* default */);

const validator_phrase = "//Alice//stash";
const validator_kv = new Keyring({ type: 'sr25519' });
const validator_pair = validator_kv.addFromUri(validator_phrase);


const register_shard = await api.tx.sudo
    .sudo(
        api.tx.shards.registerShard(0, [time1, time2, time3], collector_pubkey)
    )
    .signAndSend(pair, (result) => {
        console.log('Result of shard creation: ', result)
    });

await chan.get().then(value => console.log(value), error => console.error(error));
