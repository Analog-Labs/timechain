use crate::tasks as Tasks;

use clap::Parser;
use std::process::Command;
use subxt::rpc::{rpc_params, RpcParams};
use subxt::{OnlineClient, PolkadotConfig};

mod tasks;
#[subxt::subxt(runtime_metadata_path = "../infra/metadata.scale")]
pub mod polkadot {}

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "ws://127.0.0.1:9943")]
	url: String,
	#[clap(subcommand)]
	cmd: TestCommand,
}

#[derive(Parser, Debug)]
enum TestCommand {
	Basic,
	SetKeys,
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let url = args.url;
	let mut command = Command::new("sh");

	// let api = OnlineClient::<PolkadotConfig>::from_url(url).await.unwrap();

	match args.cmd {
		TestCommand::SetKeys => {
			set_keys().await;
		},
		TestCommand::Basic => {
			// basic_test_timechain(&api, &mut command).await;
		},
	}
}

async fn basic_test_timechain(api: &OnlineClient<PolkadotConfig>, command: &mut Command) {
	set_keys();

	// command.arg("-c").arg("echo hello");
	// let hello_1 = command.output().expect("failed to execute process");
	// println!("{:?}", hello_1.stdout);

	// let data = Tasks::get_task_state(api, 1).await;
	// println!("task data {:?}", data);
}

async fn set_keys() {
	let start_port = 9943;
	let keys = [
		"0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d",
		"0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b",
		"0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02",
		"0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f",
		"0x1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659",
		"0x72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69",
	];
	for i in 0..6 {
		log::info!("submitting key for node: {}", i+1);
		let suri = format!("owner word vocal dose decline sunset battle example forget excite gentle waste//{}//time", i+1);
		let node_port = start_port + ((i - 1) * 2) ;
		let url = format!("ws://127.0.0.1:{}", node_port);
		log::info!("url {:?}", url);
		let api = OnlineClient::<PolkadotConfig>::from_url(url).await.unwrap();
		let params: RpcParams = rpc_params!["time", suri, keys[i]];
		let _: () = api.rpc().request("author_insertKey", params).await.unwrap();
	}
}
