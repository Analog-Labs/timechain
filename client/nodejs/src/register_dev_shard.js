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

const time1 = "0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d";
const time2 = "0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b";
const time3 = "0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02";
const phrase = "//Alice";
const kv = new Keyring({type: 'sr25519'});
const pair = kv.addFromUri(phrase);

const unsub = await api.tx.sudo
  .sudo(
    api.tx.tesseractSigStorage.registerShard(1, [time1, time2, time3], null)
  )
  .signAndSend(pair, (result) => { 
  console.log('Result of shard creation: ', resul)  
});

process.exit(0)
