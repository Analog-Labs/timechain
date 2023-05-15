import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { dirname} from 'path';
import { fileURLToPath } from 'url';
import * as fs from 'fs';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9943';

// const node_address = 'ws://54.80.170.60:9944';
const data_store = [];

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
    const keyspairII = keyring.addFromUri('//Bob', { name: 'Eve default' });
    const BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';
    const Alice = '5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY';

    const chan = new Channel(0 /* default */);
    await api.isReady;
    console.log(" ---> ", api.tx.balances);
    setTimeout(async() => {
        console.log("transfer balance extrinsic");
        const txHash = await api.tx.balances.transfer(BOB, 10).signAndSend(keyspair, async({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            if(status.isFinalized) {
                let recipent = events[5].event.data[0];
                let fee_deduct = events[5].event.data[1];
                data_store.push({
                    "pallet":"balance",
                    method: "transfer",
                    recipent,
                    fee_deduct
                });
                console.log(`Current status is ${events[5].event.data}`);
            }
        });
    },1000);

    setTimeout(async() => {
        console.log("Set balance extrinsic");
        const txHash = await api.tx.balances.setBalance(BOB, 10, 10).signAndSend(keyspair, async({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            if(status.isFinalized) {
                let recipent = events[5].event.data[0];
                let fee_deduct = events[5].event.data[1];
                data_store.push({
                    "pallet":"balance",
                    method: "setBalance",
                    recipent,
                    fee_deduct
                });
                console.log(`Current status is ${events[5].event.data}`);
            }
        });
    },3000);

    setTimeout(async() => {
        console.log("transfer all balance extrinsic");
        const txHash = await api.tx.balances.transferAll(BOB, true).signAndSend(keyspairII, async({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            if(status.isFinalized) {
                let recipent = events[5].event.data[0];
                let fee_deduct = events[5].event.data[1];
                data_store.push({
                    "pallet":"balance",
                    method: "transferAll",
                    recipent,
                    fee_deduct
                });
                console.log(`Current status is ${events[5].event.data}`);
            }
        });
    },10000);
// staking pallet


setTimeout(async() => {
    console.log("validate extrinsic");
    const txHash = await api.tx.staking.validate({commission:10,blocked:false}).signAndSend(keyspairII, async({ status, events, dispatchError }) => {
        console.log(`Current status is ${status}`);
        if(status.isFinalized) {
            let recipent = events[5].event.data[0];
            let fee_deduct = events[5].event.data[1];
            data_store.push({
                "pallet":"staking",
                method: "validate",
                recipent,
                fee_deduct
            });
            console.log(`Current status is ${events[5].event.data}`);
        }
    });
},15000);


    setTimeout(async() => {
        fs.writeFileSync("./report.json",JSON.stringify(data_store),{encoding:'utf8',flag:'w'})
    },100000);



    
    await chan.get().then(value => console.log("logs --> ",value), error => console.error(error));
    // chan.close();
};

pallet_task_add();
