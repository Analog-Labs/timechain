/**
 * Base-Activity Oracle for Arweave
 * Stores Base-chain activity snapshots for any ENS / address.
 * U.S. Pat. 12,131,320 B2 (cross-chain attestation)
 */

export const contractSrc = `
export async function handle(state, action) {
  const input = action.input;

  switch (input.function) {

    case "update": {
      // only oracle worker (hard-coded address) may update
      if (action.caller !== state.oracle) {
        throw "Only oracle may update";
      }
      if (!input.ens || !input.address || !input.snapshot) {
        throw "Missing fields";
      }
      state.snapshots[input.ens] = {
        address: input.address,
        snapshot: input.snapshot,
        updatedAt: SmartWeave.block.timestamp,
        txHash: input.txHash || ""
      };
      return { state };
    }

    case "query": {
      const ens = input.ens;
      if (!state.snapshots[ens]) {
        throw "ENS not found";
      }
      return { result: state.snapshots[ens] };
    }

    case "list": {
      return { result: Object.keys(state.snapshots) };
    }

    default:
      throw "Unknown function";
  }
};

export const initialState = {
  oracle: "0xbF7Da1f568684889A69A5BED9F1311F703985590", // uniaolives.base.eth
  snapshots: {}
};
`