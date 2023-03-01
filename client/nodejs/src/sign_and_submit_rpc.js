import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { stringToU8a,stringToHex,u8aToHex } from '@polkadot/util';

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
                ]
            }
        }
    }
});
const phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

let input_data = '{"key": "value"}';
const message = stringToU8a(input_data);
const message_data = stringToHex(input_data);
const signature = u8aToHex(pair.sign(message))
const resp = await api.rpc.time.submitForSigning(1, message_data, signature);

console.log('Submitted data for signing: ', resp);

process.exit(0)
