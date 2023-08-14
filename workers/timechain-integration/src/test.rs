use crate::query::{CollectData, ResponseData, Variables};
use dotenv::dotenv;
use graphql_client::{GraphQLQuery, Response};
use reqwest::header;
use std::env;

#[tokio::test]
#[ignore]
async fn test_collect_data() {
	// Prepare the input variables
	let variables = Variables {
		collection: "QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7".to_owned(),
		task_id: 1,
		task_counter: 15,
		block: 1,
		cycle: 15,
		tss: hex::encode([1u8; 64]),
		data: vec!["0".to_string()],
	};
	dotenv().ok();
	let testgraphql_url =
		env::var("TestGraphQL_URL").expect("TIMEGRAPH_GRAPHQL_URL is not set in the .env file");

	// Build the GraphQL request
	let request = CollectData::build_query(variables);
	println!("{:?}", request);
	// Execute the GraphQL request
	let response = reqwest::Client::new()
			.post(testgraphql_url)
			.json(&request)
			.header(header::AUTHORIZATION, "0;FJ9BTQbLg8DcSYpDUSdPwdGudTmGFqDokcBYRiJBYXWX;0;*;1;Xk7BGGAymUw4Cwg8p4BFpEbn5AgHmN2JjYNkcFdsT9ECJPLuRYXcy5Ct1w1F77R88uxicwGZUPkNpsib6C3gzYY")
			.send()
			.await
			.expect("Failed to send request")
			.json::<Response<ResponseData>>()
			.await
			.expect("Failed to parse response");
	println!("{:?}", response);
	match &response.data {
		Some(data) => {
			println!("{:?}", data.collect.status);

			println!("{:?}", data.collect);
		},
		None => println!("no data found"),
	};
}
