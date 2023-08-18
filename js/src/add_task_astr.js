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
    // const custom_types = await get_custom_types();
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
    const contract_address = process.argv[2];
    const start_block = process.argv[3];

    const input_task = {
        network: 1,
        cycle: 1,
        start: start_block,
        period: 0,
        hash: 'QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7',
        function: {
            EVMViewWithoutAbi: {
                address: stringToHex(contract_address),
                function_signature: "function get_votes_stats() external view returns (uint, uint)",
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
