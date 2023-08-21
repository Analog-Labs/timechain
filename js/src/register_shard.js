import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';
import { stringToU8a, u8aToHex } from '@polkadot/util';
import { Channel } from 'async-channel';
import { exit } from 'process';

const setup_substrate = async () => {
    const wsProvider = new WsProvider('ws://127.0.0.1:9943');
    // const custom_types = await get_custom_types();
    const api = await ApiPromise.create({
        provider: wsProvider,
        noInitWarn: true
    });
    return api;
};

const register_shard = async () => {
    var collection = 0;
    //accepted args 0, 1
    // 0 for first 3
    // 1 for last 3
    if (process.argv[2] != undefined){
        collection = process.argv[2];
    }

    var time1 = [76, 181, 171, 246, 173, 121, 251, 245, 171, 188, 202, 252, 194, 105, 216, 92, 210, 101, 30, 212, 184, 133, 181, 134, 159, 36, 26, 237, 240, 165, 186, 41];
    var time2 = [116, 34, 185, 136, 117, 152, 6, 142, 50, 196, 68, 138, 148, 154, 219, 41, 13, 15, 78, 53, 185, 224, 27, 14, 229, 241, 161, 230, 0, 254, 38, 116];
    var time3 = [243, 129, 98, 110, 65, 231, 2, 126, 164, 49, 191, 227, 0, 158, 148, 189, 210, 90, 116, 107, 238, 196, 104, 148, 141, 108, 60, 124, 93, 201, 165, 75];
    var collector_pubkey = { "sr25519": "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d" };

    if (collection == 1) {
        time1 = [253, 80, 184, 227, 177, 68, 234, 36, 79, 191, 119, 55, 245, 80, 188, 141, 208, 194, 101, 11, 188, 26, 173, 168, 51, 202, 23, 255, 141, 191, 50, 155];
        time2 = [253, 228, 251, 160, 48, 173, 0, 47, 124, 47, 125, 76, 51, 31, 73, 209, 63, 176, 236, 116, 126, 206, 235, 236, 99, 79, 31, 244, 203, 202, 157, 239];
        time3 = [180, 201, 42, 251, 59, 165, 127, 58, 185, 89, 255, 230, 211, 25, 201, 132, 132, 162, 21, 90, 15, 76, 101, 178, 195, 112, 17, 255, 209, 151, 176, 117];
        collector_pubkey = { "sr25519": "0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f" };
    }

    var network = 0;
    // accepted args 0, 1
    // 0 for ethereum
    // 1 for astar
    if (process.argv[3] != undefined){
        network = parseInt(process.argv[3]);
    }

    const api = await setup_substrate();
    const phrase = "//Alice";
    const kv = new Keyring({ type: 'sr25519' });
    const pair = kv.addFromUri(phrase);

    await api.isReady;
    const register_shard = await api.tx.sudo
        .sudo(
            api.tx.shards.registerShard(network, [time1, time2, time3], collector_pubkey)
        )
        .signAndSend(pair, ({ status, events }) => {
            if (status.isInBlock || status.isFinalized) {
                const filtered_events = events.filter(({ event }) => api.events.shards.ShardCreated.is(event))
                if (filtered_events.length >= 1) {
                    console.log("details :", filtered_events[0].event.data.toJSON());
                    process.exit(0);
                }
            }
        });

}

register_shard();



