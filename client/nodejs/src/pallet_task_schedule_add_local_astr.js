import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { dirname} from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9943';

const setup_substrate = async () => {
    const wsProvider = new WsProvider(node_address);
    // const custom_types = await get_custom_types();
    const api = await ApiPromise.create({
        provider: wsProvider
    });
    //         , "types": custom_types
    return api;
};

const pallet_task_add = async (_keyspair, who) => {
    const api =  await setup_substrate();
    const keyring = new Keyring({ type: 'sr25519' });
    const keyspair = keyring.addFromUri('//Alice', { name: 'Alice default' });

    const chan = new Channel(0 /* default */);
    const input_task = {
        task_id: 2,
        owner: 'address',
        shard_id: 1,
        frequency: 5,
        cycle: 3,
        validity: { Seconds: 12 },
        hash: 'asdasd',
        status: 0
    }
    await api.isReady;
    console.log("api.tx.task_meta ---> ", api.tx.taskSchedule);
    const unsub = await api.tx.taskSchedule.insertSchedule(input_task).signAndSend(keyspair, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
    
    await chan.get().then(value => console.log(value), error => console.error(error));
    // chan.close();
};

pallet_task_add();
