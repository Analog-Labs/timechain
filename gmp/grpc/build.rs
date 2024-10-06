use tonic_build::manual::{Builder, Method, MethodBuilder, Service};

fn method(name: &str, route_name: &str) -> MethodBuilder {
	Method::builder()
		.name(name)
		.route_name(route_name)
		.input_type(format!("crate::proto::{route_name}Request"))
		.output_type(format!("crate::proto::{route_name}Response"))
		.codec_path("crate::codec::BincodeCodec")
}

fn main() {
	let service = Service::builder()
		.name("Gmp")
		.package("gmp")
		.method(method("faucet", "Faucet").build())
		.method(method("transfer", "Transfer").build())
		.method(method("balance", "Balance").build())
		.method(method("block_stream", "BlockStream").server_streaming().build())
		.method(method("read_events", "ReadEvents").build())
		.method(method("submit_commands", "SubmitCommands").build())
		.method(method("deploy_gateway", "DeployGateway").build())
		.method(method("redeploy_gateway", "RedeployGateway").build())
		.method(method("admin", "Admin").build())
		.method(method("set_admin", "SetAdmin").build())
		.method(method("shards", "Shards").build())
		.method(method("set_shards", "SetShards").build())
		.method(method("routes", "Routes").build())
		.method(method("set_route", "SetRoute").build())
		.method(method("deploy_test", "DeployTest").build())
		.method(method("estimate_message_cost", "EstimateMessageCost").build())
		.method(method("send_message", "SendMessage").build())
		.method(method("recv_messages", "RecvMessages").build())
		.build();
	Builder::new().compile(&[service]);
}
