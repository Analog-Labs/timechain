import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { Channel } from 'async-channel';
import { readFileSync } from 'fs';
import { dirname, join, normalize, format } from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const node_address = 'ws://127.0.0.1:9944';
export const SECRETSEED = process.env.PALLET_MNEMONIC || "//Alice";

const get_custom_types = async () => {
    const relative_path = join('..', '..', '..');
    const root_path = normalize(join(__dirname, relative_path));
    const asset_path = join(root_path, 'assets');
    const types_file = format({
        root: '/ignored',
        dir: asset_path,
        base: 'types.json'
    });
    console.log('path \t ', types_file);
    const custom_types = JSON.parse(readFileSync(types_file, 'utf8'));
    return custom_types;
};

export const setup_substrate = async () => {
    const wsProvider = new WsProvider(node_address);
    // const custom_types = await get_custom_types();
    const api = await ApiPromise.create({
        provider: wsProvider
    });
    //         , "types": custom_types
    return api;
};

export const destination = async (api, block, index) => {
    try {
        const blockHash = await api.rpc.chain.getBlockHash(block);
        const signedBlock = await api.rpc.chain.getBlock(blockHash);
        const { method: { args, method, section } } = signedBlock.block.extrinsics[index];
        if (section === 'membership') {
            return args[1].toHex();
        } else {
            return null;
        }
    }
    catch (err) {
        console.error('Error', err);
        return null;
    }
}

export const native_transfer = async (api, keyspair, amount, address) => {
    const chan = new Channel(0 /* default */);
    const unsub = await api.tx.balances.transfer(address, amount)
        .signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            try {
                // status would still be set, but in the case of error we can shortcut
                // to just check it (so an error would indicate InBlock or Finalized)
                if (dispatchError) {
                    if (dispatchError.isModule) {
                        // for module errors, we have the section indexed, lookup
                        const decoded = api.registry.findMetaError(dispatchError.asModule);
                        const { name, section } = decoded;
                        console.log(`${section}.${name}`);
                    }
                    else {
                        // Other, CannotLookup, BadOrigin, no extra info
                        console.log(`other: ${dispatchError.toString()}`);
                    }
                    unsub();
                    chan.push("error");
                }
                if (status.isInBlock) {
                    console.log(`Transaction included at blockHash ${status.asInBlock}`);
                    chan.push(`isInBlock.`);
                }
                else if (status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
                    // Loop through Vec<EventRecord> to display all events
                    events.forEach(({ phase, event: { data, method, section } }) => {
                        console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
                    });
                    unsub();
                    // chan.push(`Event report end.`);
                }
            }
            catch {
                console.log("[Error] Too many transactions in short time");
            }
        });
    await chan.get().then(value => console.log(value), error => console.error(error));
    chan.close();
};

export const pallet_membership_add = async (api, keyspair, who) => {
    const chan = new Channel(0 /* default */);
    const unsub = await api.tx.sudo
        .sudo(api.tx.membership.addMember(who))
        .signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            try {
                // status would still be set, but in the case of error we can shortcut
                // to just check it (so an error would indicate InBlock or Finalized)
                if (dispatchError) {
                    if (dispatchError.isModule) {
                        // for module errors, we have the section indexed, lookup
                        const decoded = api.registry.findMetaError(dispatchError.asModule);
                        const { name, section } = decoded;
                        console.log(`${section}.${name}`);
                    }
                    else {
                        // Other, CannotLookup, BadOrigin, no extra info
                        console.log(`other: ${dispatchError.toString()}`);
                    }
                    unsub();
                    chan.push("error");
                }
                if (status.isInBlock) {
                    console.log(`Transaction included at blockHash ${status.asInBlock}`);
                }
                else if (status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
                    events
                        // We know this tx should result in `Sudid` event.
                        .filter(({ event }) =>
                            api.events.sudo.Sudid.is(event)
                        )
                        // We know that `Sudid` returns just a `Result`
                        .forEach(({ event: { data: [result] } }) => {
                            // Now we look to see if the extrinsic was actually successful or not...
                            if (result.isError) {
                                let error = result.asError;
                                if (error.isModule) {
                                    // for module errors, we have the section indexed, lookup
                                    const decoded = api.registry.findMetaError(error.asModule);
                                    const { docs, name, section } = decoded;

                                    console.log(`${section}.${name}: ${docs.join(' ')}`);
                                } else {
                                    // Other, CannotLookup, BadOrigin, no extra info
                                    console.log(error.toString());
                                }
                                unsub();
                                chan.push("error");
                            } else {

                            }
                        });
                    unsub();
                    chan.push(`Event report end.`);
                }
            }
            catch {
                console.log("[Error] Too many transactions in short time");
            }
        });
    await chan.get().then(value => console.log(value), error => console.error(error));
    chan.close();
};

export const pallet_membership_remove = async (api, keyspair, who) => {
    const chan = new Channel(0 /* default */);
    const unsub = await api.tx.sudo
        .sudo(api.tx.membership.removeMember(who))
        .signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            try {
                // status would still be set, but in the case of error we can shortcut
                // to just check it (so an error would indicate InBlock or Finalized)
                if (dispatchError) {
                    if (dispatchError.isModule) {
                        // for module errors, we have the section indexed, lookup
                        const decoded = api.registry.findMetaError(dispatchError.asModule);
                        const { name, section } = decoded;
                        console.log(`${section}.${name}`);
                    }
                    else {
                        // Other, CannotLookup, BadOrigin, no extra info
                        console.log(`other: ${dispatchError.toString()}`);
                    }
                    unsub();
                    chan.push("error");
                }
                if (status.isInBlock) {
                    console.log(`Transaction included at blockHash ${status.asInBlock}`);
                }
                else if (status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
                    events
                        // We know this tx should result in `Sudid` event.
                        .filter(({ event }) =>
                            api.events.sudo.Sudid.is(event)
                        )
                        // We know that `Sudid` returns just a `Result`
                        .forEach(({ event: { data: [result] } }) => {
                            // Now we look to see if the extrinsic was actually successful or not...
                            if (result.isError) {
                                let error = result.asError;
                                if (error.isModule) {
                                    // for module errors, we have the section indexed, lookup
                                    const decoded = api.registry.findMetaError(error.asModule);
                                    const { docs, name, section } = decoded;

                                    console.log(`${section}.${name}: ${docs.join(' ')}`);
                                } else {
                                    // Other, CannotLookup, BadOrigin, no extra info
                                    console.log(error.toString());
                                }
                                unsub();
                                chan.push("error");
                            } else {

                            }
                        });
                    unsub();
                    chan.push(`Event report end.`);
                }
            }
            catch {
                console.log("[Error] Too many transactions in short time");
            }
        });
    await chan.get().then(value => console.log(value), error => console.error(error));
    chan.close();
};

export const pallet_membership_reset_members = async (api, keyspair, members) => {
    const chan = new Channel(0 /* default */);
    const unsub = await api.tx.sudo
        .sudo(api.tx.membership.resetMembers(members))
        .signAndSend(keyspair, ({ status, events, dispatchError }) => {
            console.log(`Current status is ${status}`);
            try {
                // status would still be set, but in the case of error we can shortcut
                // to just check it (so an error would indicate InBlock or Finalized)
                if (dispatchError) {
                    if (dispatchError.isModule) {
                        // for module errors, we have the section indexed, lookup
                        const decoded = api.registry.findMetaError(dispatchError.asModule);
                        const { name, section } = decoded;
                        console.log(`${section}.${name}`);
                    }
                    else {
                        // Other, CannotLookup, BadOrigin, no extra info
                        console.log(`other: ${dispatchError.toString()}`);
                    }
                    unsub();
                    chan.push("error");
                }
                if (status.isInBlock) {
                    console.log(`Transaction included at blockHash ${status.asInBlock}`);
                }
                else if (status.isFinalized) {
                    console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
                    events
                        // We know this tx should result in `Sudid` event.
                        .filter(({ event }) =>
                            api.events.sudo.Sudid.is(event)
                        )
                        // We know that `Sudid` returns just a `Result`
                        .forEach(({ event: { data: [result] } }) => {
                            // Now we look to see if the extrinsic was actually successful or not...
                            if (result.isError) {
                                let error = result.asError;
                                if (error.isModule) {
                                    // for module errors, we have the section indexed, lookup
                                    const decoded = api.registry.findMetaError(error.asModule);
                                    const { docs, name, section } = decoded;

                                    console.log(`${section}.${name}: ${docs.join(' ')}`);
                                } else {
                                    // Other, CannotLookup, BadOrigin, no extra info
                                    console.log(error.toString());
                                }
                                unsub();
                                chan.push("error");
                            } else {

                            }
                        });
                    unsub();
                    chan.push(`Event report end.`);
                }
            }
            catch {
                console.log("[Error] Too many transactions in short time");
            }
        });
    await chan.get().then(value => console.log(value), error => console.error(error));
    chan.close();
};

