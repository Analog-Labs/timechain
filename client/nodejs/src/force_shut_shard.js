import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';
import { stringToU8a, u8aToHex } from '@polkadot/util';
import { Channel } from 'async-channel';

const wsProvider = new WsProvider('ws://127.0.0.1:9943');
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

const phrase = "//Alice";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

const chan = new Channel(0 /* default */);

setTimeout(async () => {
    const register_shard = await api.tx.sudo
        .sudo(
            api.tx.tesseractSigStorage.forceSetShardOffline(0)
        )
        .signAndSend(pair, (result) => {
            console.log('Result of shard creation: ', result)
        });
}, 45000)

await chan.get().then(value => console.log(value), error => console.error(error));
