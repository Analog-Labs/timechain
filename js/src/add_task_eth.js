import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { dirname } from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9943';

const stringToHex = (str) => {
    return '0x' + str.split('').map((char) => char.charCodeAt(0).toString(16)).join('');
};

const setup_substrate = async () => {
    const wsProvider = new WsProvider(node_address);
    const api = await ApiPromise.create({
        provider: wsProvider,
        noInitWarn: true
    });
    return api;
};

const pallet_task_add = async (_keyspair, who) => {
    const api = await setup_substrate();
    const keyring = new Keyring({ type: 'sr25519' });
    const keyspair = keyring.addFromUri('//Alice', { name: 'Alice default' });
    
    const input_task = {
        network: 0,
        cycle: 4,
        start: 0,
        period: 2,
        hash: 'QmYFw5aYPKQ9oSw3L3UUed9fBqT4oTW5BZzAnPFGyuQir3',
        function: {
            EVMViewWithoutAbi: {
                address: stringToHex('0x3de7086ce750513ef79d14eacbd1282c4e4b0cea'),
                function_signature: "function get_votes_stats() external view returns (uint[] memory)",
                input: [],
            }
        },
    }
    await api.isReady;
    const result = await api.tx.tasks.createTask(input_task).signAndSend(keyspair, ({ status, events }) => {
        if (status.isInBlock || status.isFinalized) {
            const filtered_events = events.filter(({ event }) => api.events.tasks.TaskCreated.is(event))
            if (filtered_events.length >= 1) {
                console.log("registered_task:", filtered_events[0].event.data.toJSON().pop());
                process.exit(0);
            }
        }
    });
};

pallet_task_add();
