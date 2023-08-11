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

const time1 = [253, 80, 184, 227, 177, 68, 234, 36, 79, 191, 119, 55, 245, 80, 188, 141, 208, 194, 101, 11, 188, 26, 173, 168, 51, 202, 23, 255, 141, 191, 50, 155];
const time2 = [253, 228, 251, 160, 48, 173, 0, 47, 124, 47, 125, 76, 51, 31, 73, 209, 63, 176, 236, 116, 126, 206, 235, 236, 99, 79, 31, 244, 203, 202, 157, 239];
const time3 = [180, 201, 42, 251, 59, 165, 127, 58, 185, 89, 255, 230, 211, 25, 201, 132, 132, 162, 21, 90, 15, 76, 101, 178, 195, 112, 17, 255, 209, 151, 176, 117];
let collector_pubkey = { "sr25519": "0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f" }
const phrase = "//Alice";
const kv = new Keyring({ type: 'sr25519' });
const pair = kv.addFromUri(phrase);

const chan = new Channel(0 /* default */);


const register_shard = await api.tx.sudo
    .sudo(
        api.tx.shards.registerShard(0, [time1, time2, time3], collector_pubkey)
    )
    .signAndSend(pair, (result) => {
        console.log('Result of shard creation: ', result)
    });

await chan.get().then(value => console.log(value), error => console.error(error));
