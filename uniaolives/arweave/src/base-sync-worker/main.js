import axios from "axios";
import Arweave from "arweave";
import { interactWrite } from "smartweave";

const arweave = Arweave.init({ host: "arweave.net", port: 443, protocol: "https" });
const CONTRACT_TX_ID = process.env.CONTRACT_TX_ID; // deployed once
const ORACLE_PRIV_KEY = process.env.ORACLE_PRIV_KEY; // same address as contract.oracle

const BASE_RPC = "https://base-mainnet.g.alchemy.com/v2/YOUR_KEY";
const ENS = "uniaolives.base.eth";

async function fetchBaseActivity(address) {
  const res = await axios.post(BASE_RPC, {
    jsonrpc: "2.0",
    id: 1,
    method: "alchemy_getAssetTransfers",
    params: {
      fromBlock: "0x0",
      toBlock: "latest",
      fromAddress: address,
      withMetadata: true,
      maxCount: "0x64"
    }
  });
  return res.data.result.transfers;
}

async function updateOracle(address, snapshot) {
  const input = {
    function: "update",
    ens: ENS,
    address: address,
    snapshot: snapshot,
    txHash: snapshot.latestTxHash
  };
  const interactionTx = await interactWrite(arweave, ORACLE_PRIV_KEY, CONTRACT_TX_ID, input);
  console.log("Oracle updated:", interactionTx);
}

async function main() {
  const address = "0xbF7Da1f568684889A69A5BED9F1311F703985590"; // resolved ENS
  const snapshot = await fetchBaseActivity(address);
  await updateOracle(address, snapshot);
}

main().catch(console.error);
