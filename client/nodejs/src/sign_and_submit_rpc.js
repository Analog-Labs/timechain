import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { stringToU8a } from '@polkadot/util';
import { sha512AsU8a } from '@polkadot/util-crypto';

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
                        name: 'message_a',
                        type: '[u8; 32]'
                    },
                    {
                        name: 'message_b',
                        type: '[u8; 32]'
                    },
                    {
                        name: 'signature_a',
                        type: '[u8; 32]'
                    },
                    {
                        name: 'signature_b',
                        type: '[u8; 32]'
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

const message = '{"key": "value"}';
const hash = sha512AsU8a(message);
const signature = pair.sign(message)
const set_id = api.createType('u64', 1);
const hash_a = api.createType('[u8; 32]', hash.slice(0, 32));
const hash_b = api.createType('[u8; 32]', hash.slice(32, 64));
const signature_a = api.createType('[u8; 32]', signature.slice(0, 32));
const signature_b = api.createType('[u8; 32]', signature.slice(32, 64));
const resp = await api.rpc.time.submitForSigning(set_id, hash_a, hash_b, signature_a, signature_b);

console.log('Submitted data for signing: ', resp);

process.exit(0)
