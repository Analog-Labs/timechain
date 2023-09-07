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
        await sleep(10000);
        const task_state = await api.query.tasks.taskState(task_id);
        const task_state_unwraped = task_state.unwrap();
        const stored_result = await api.query.tasks.taskResults.keys(task_id);

        if (task_state_unwraped.isCompleted) {
            console.log("task_id:" ,task_id, "success with ok results:", stored_result.length);
            break;
        } else if (task_state_unwraped.isFailed){
            console.log("Task failed with error", task_state_unwraped.toHuman());
            break;
        }else{
            console.log("Task not finished, Iterating again, results stored: ", stored_result.length);
        }
    }

    process.exit(0);
};

await_task_status();
