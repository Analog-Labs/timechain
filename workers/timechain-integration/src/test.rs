#[cfg(test)]
mod tests {
	use crate::query::{collect_data, CollectData};

	use dotenv::dotenv;
	use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
	use reqwest::header;
	use std::env;
	#[tokio::test]
	#[ignore]
	async fn test_collect_data() {
		// Prepare the input variables
		let variables = collect_data::Variables {
			collection: "QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7".to_owned(),
			block: 1,
			cycle: 15,
			task_id: 1,
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
			.json::<GraphQLResponse<collect_data::ResponseData>>()
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
}
