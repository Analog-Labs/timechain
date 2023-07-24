import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Keypair } from '@solana/web3.js';
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
    const keyspair = keyring.addFromUri("owner word vocal dose decline sunset battle example forget excite gentle waste//4//time", { name: 'CollectorShard1' });
    const alice = keyring.addFromUri("//Alice", { name: 'Alice' });
    await api.isReady;
    //5CkJ4tBMgREW3mTwiG9CQZA5fEHW8RzLjX1fFv91BTuMVqqy
    console.log(keyspair.address);

    //faucet before claiming
    const unsub1 = await api.tx.balances.transfer(keyspair.address, 100000000000).signAndSend(alice, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
    console.log("faucet done");

    //update task id before claiming
    const unsub = await api.tx.tesseractSigStorage.claimTask(1, 2).signAndSend(keyspair, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
};

pallet_task_add();
