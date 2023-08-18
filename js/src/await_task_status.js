import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { dirname } from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9943';

const setup_substrate = async () => {
    const wsProvider = new WsProvider(node_address);
    const api = await ApiPromise.create({
        provider: wsProvider,
        noInitWarn: true
    });
    return api;
};

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

const await_task_status = async (_keyspair, who) => {
    const api = await setup_substrate();
    const keyring = new Keyring({ type: 'sr25519' });
    const keyspair = keyring.addFromUri('//Alice', { name: 'Alice default' });
    const task_id = process.argv[2];

    while (true) {
        const task_state = await api.query.tasks.taskState(task_id);
        const task_state_unwraped = task_state.unwrap();
        console.log("Current Task Status:", task_state_unwraped.toHuman());

        if (task_state_unwraped.isCompleted || task_state_unwraped.isFailed) {
            console.log("Task success:", task_state_unwraped.isCompleted);
            const stored_result = await api.query.tasks.taskResults(task_id, null);
            console.log("Task results stored", stored_result.toJSON());
            break;
        }
        console.log("Task not finished, iterating again");
        sleep(1000);
    }

    process.exit(0);
};

await_task_status();
