import { ethers } from "hardhat";

interface SwapToken {
  chain: String,
  chain_endpoint: String,
  exchange: String,
  exchange_address: String,
  exchange_endpoint: String,
  token: String,
  token_address: String,
  token_endpoint: String,
  swap_token: String,
  swap_token_address: String,
  swap_token_endpoint: String,
}

let token:any;
let contractAddress: any[] = [];
let numberOfContractToDeploy = 2; // number Of Contract To Deploy

async function main() {

  const Swap_price = await ethers.getContractFactory("TokenSwap");
  const swap_price = await Swap_price.deploy();

  await swap_price.deployed();
// deploy tokens
  const Token = await ethers.getContractFactory("ERC20");

    for(let itr = 0; itr < numberOfContractToDeploy; itr++) {
        token = await Token.deploy(`analog${itr}`, `ANA${itr}`);
        await token.deployed();
        contractAddress.push(token);

    }

    // return contractAddress;

  console.log("address = ",swap_price, swap_price.address);

  const swap_input: SwapToken = {
    chain: "localChain",
    chain_endpoint: "http://127.0.0.1:8545",
    exchange: "localChain",
    exchange_address: swap_price.address,
    exchange_endpoint: "contracts/artifacts/contracts/swap_price.sol/TokenSwap.json",
    token: await contractAddress[0].symbol(),
    token_address: contractAddress[0].address,// "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    token_endpoint: 'contracts/artifacts/contracts/ERC20.sol/ERC20.json',
    swap_token: await contractAddress[1].symbol(),
    swap_token_address: contractAddress[1].address,//"0x0F5D2fB29fb7d3CFeE444a200298f468908cC942",
    swap_token_endpoint: 'contracts/artifacts/contracts/ERC20.sol/ERC20.json',
  };
  console.log("swap_input ---> ", swap_input);

}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
