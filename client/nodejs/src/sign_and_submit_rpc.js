import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';
import { stringToU8a, u8aToHex } from '@polkadot/util';

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
const phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

let input_data = '{"key": "value"}';
const message = stringToU8a(input_data);
const message_hash = keccak256AsU8a(message);
console.log('hash length is ', message_hash.length);
const message_data = u8aToHex(message_hash);
const signature = u8aToHex(pair.sign(message_hash))
console.log('Message: ', message_data);
console.log('Signature:', signature);
const resp = await api.rpc.time.submitForSigning(0, message_data, signature);

console.log('Submitted data for signing: ', resp);

process.exit(0)
