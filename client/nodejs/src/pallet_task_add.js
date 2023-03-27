import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { dirname} from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9943';

const hexTostring = (str) => {
    const hexString = str.replace(/^0x/, '');
    const pairs = hexString.match(/.{2}/g);
    const codes = pairs.map((pair) => parseInt(pair, 16));
    return String.fromCharCode(...codes);
};

const stringToHex = (str) => {
    return '0x' + str.split('').map((char) => char.charCodeAt(0).toString(16)).join('');
};

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
        collection_id: 11,
        schema:[1],
        function:{ethereumcontract:{
            address: stringToHex('0x82E75Add4823372C5448A71E76cef5C78ba5259E'),
            abi: "[{\"inputs\":[],\"name\":\"sayHelloWorld\",\"outputs\":[{\"internalType\":\"string\",\"name\":\"\",\"type\":\"string\"}],\"stateMutability\":\"pure\",\"type\":\"function\"}]",
            function: "sayHelloWorld",
            input: 2,
            output: 2,
        }},
        with:['123','123'],
        cycle:12,
        validity:{Seconds:12},
        hash:'asdasd'
    }
    await api.isReady;
    console.log("api.tx.task_meta ---> ", api.tx.taskMeta.insertTask);
    let input_2 = {...input_task, collection_id : 22};
    let input_3 = {...input_task, collection_id : 33};

    const unsub = await api.tx.taskMeta.insertTask(input_task).signAndSend(keyspair, ({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
    });
    setTimeout(async() => {
        const unsub2 = await api.tx.taskMeta.insertTask(input_2).signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
        });
    },15000)
    setTimeout(async() => {
        const unsub3 = await api.tx.taskMeta.insertTask(input_3).signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
        });
    },30000)
    
    await chan.get().then(value => console.log(value), error => console.error(error));
    // chan.close();
};

pallet_task_add();
