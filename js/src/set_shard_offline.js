import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { exit } from 'process';
import { dotenv } from 'dotenv';

const setup_substrate = async () => {
    const wsProvider = new WsProvider('ws://127.0.0.1:9943');
    const api = await ApiPromise.create({
        provider: wsProvider,
        noInitWarn: true
    });
    return api;
};

const set_shard_offline = async () => {
    dotenv.config();
    if (process.env.PHRASE === undefined) {
        console.log('Cannot find PHRASE for extrinsic');
        exit(1);
    }

    if (process.argv.length < 4) {
        console.log('set_shard_offline.js <shard_id> <network>');
        exit(1);
       
    }
    const phrase = process.env.PHRASE;
    const shard_id = process.argv[2];
    const network = process.argv[3];

    const api = await setup_substrate();
    const kv = new Keyring({ type: 'sr25519' });
    const pair = kv.addFromUri(phrase);

    await api.isReady;
    await api.tx.ocw.setShardOffline(shard_id, 0)
        .signAndSend(pair, ({ status, events }) => {
            if (status.isInBlock || status.isFinalized) {
                const filtered_events = events.filter(({ event }) => api.events.shards.ShardOffline.is(event))
                if (filtered_events.length >= 1) {
                    console.log("details :", filtered_events[0].event.data.toJSON());
                    exit(0);
                }
            }
        });

}

set_shard_offline();



