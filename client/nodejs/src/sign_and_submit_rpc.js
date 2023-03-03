import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';
import * as $ from 'scale-codec';

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
                ],
                type: 'RpcResult<((), u32)>'
            }
        }
    }
});
const phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

let input_data = '{"key": "value"}';
const message = keccak256AsU8a(input_data);
const message_hash = message.encode();
console.log('hash length is ', message_hash.length);
const signature = pair.sign(message_hash).encode();
console.log('sig_vec: ', sig_vec);
console.log('Message: ', message_hash);
console.log('Signature:', signature);
//const resp = await api.rpc.time.submitForSigning(123, message_hash, signature);

console.log('Submitted data for signing: ', resp);

process.exit(0)
