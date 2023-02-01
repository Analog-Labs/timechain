import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { stringToU8a } from '@polkadot/util';

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
                        type: 'Vec<u8>'
                    },
                    {
                        name: 'signature',
                        type: 'Vec<u8>'
                    }
                ]
            }
        }
    }
});
const phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

const message = stringToU8a('{"key": "value"}');
const signature = pair.sign(message)
const resp = await api.rpc.time.submitForSigning(1, message, signature);

console.log('Submitted data for signing: ', resp);

process.exit(0)
