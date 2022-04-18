import { Keyring } from '@polkadot/api';
import '@polkadot/api-augment';
import { SECRETSEED, setup_substrate, pallet_membership_reset_members} from './pallet_membership.js';
let api;

const main = async () => {
    // let provider = null;
    try {
        api = await setup_substrate();
        const keyring = new Keyring({ type: 'sr25519' });
        // create Alice based on the development seed
        const keyspair = keyring.addFromUri(SECRETSEED);
        const bob = keyring.addFromUri('//Bob');
        const charlie = keyring.addFromUri('//Charlie');
        const dave = keyring.addFromUri('//Dave');
        const eve = keyring.addFromUri('//Eve');
        // Wait until we are ready and connected
        await api.isReady;
        const members = [keyspair.address, bob.address, charlie.address, dave.address, eve.address];
        await pallet_membership_reset_members(api, keyspair, members);
        const stored_members = await api.query.membership.members();
        console.log(stored_members.toHuman());
    }
    catch (err) {
        console.error('Error', err);
    }
    finally {
        // provider!.engine.stop();
    }
};
main().catch(console.error).finally(() => {
    console.log('end');
    process.exit(0);
});
