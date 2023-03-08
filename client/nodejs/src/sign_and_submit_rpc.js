import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { sha512AsU8a, keccak256AsU8a } from '@polkadot/util-crypto';

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
                type: 'Result<(), u32>'
            }
        }
    }
});
const phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

// this should be exact message from source chain
let input_data = '{"key": "value"}';
// hash it with keccak256
const message = keccak256AsU8a(input_data);
// format into json array of bytes
const message_hash = '[' + message.toString('hex') + ']';
// sign using sr25519 pair
const raw_sig = pair.sign(message);
// format into json array of bytes
const signature = '[' + raw_sig.toString('hex') + ']';
// submit :)
const resp = await api.rpc.time.submitForSigning(123, message_hash, signature);
// confirm it worked
console.log('Submitted data for signing: ', resp);

process.exit(0)
