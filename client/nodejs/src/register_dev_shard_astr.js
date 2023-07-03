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

const time1 = "0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f";
const time2 = "0x1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659";
const time3 = "0x72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69";
const phrase = "//Alice";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

const chan = new Channel(0 /* default */);

const validator_phrase = "//Alice//stash";
const validator_kv = new Keyring({ type: 'sr25519' });
const validator_pair = validator_kv.addFromUri(validator_phrase);

const register_chronicle = await api.tx.tesseractSigStorage.registerChronicle(time1).signAndSend(validator_pair, ({ status, events, dispatchError }) => {
    console.log(`Current status is ${status}`);
});

setTimeout(async () => {
    const register_chronicle_2 = await api.tx.tesseractSigStorage.registerChronicle(time2).signAndSend(validator_pair, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
}, 15000)


setTimeout(async () => {
    const register_chronicle_3 = await api.tx.tesseractSigStorage.registerChronicle(time3).signAndSend(validator_pair, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
}, 30000)


setTimeout(async () => {
    const register_shard = await api.tx.sudo
        .sudo(
            api.tx.tesseractSigStorage.registerShard([time1, time2, time3], 0)
        )
        .signAndSend(pair, (result) => {
            console.log('Result of shard creation: ', result)
        });
}, 45000)

await chan.get().then(value => console.log(value), error => console.error(error));
