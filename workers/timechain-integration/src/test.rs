

#[cfg(test)]
mod tests {
	use crate::query::{collect_data, CollectData};

	use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
	use dotenv::dotenv;
	use std::env;
	#[tokio::test]
	async fn test_collect_data() {
		// Prepare the input variables
		let variables = collect_data::Variables {
			collection: "QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7".to_owned(),
			block: 2,
			cycle: 10,
			task_id: 3,
			data: vec!["1agsdgdgfsdfsfsddfgdfg".to_owned()],
		};
		dotenv::from_filename("../../.env").ok();
		dotenv().ok();
		let testgraphql_url = env::var("TestGraphQL_URL")
			.expect("TIMEGRAPH_GRAPHQL_URL is not set in the .env file");
		
		// Build the GraphQL request
		let request = CollectData::build_query(variables);

		// Execute the GraphQL request
		let response = reqwest::Client::new()
			.post(testgraphql_url)
			.json(&request)
			.send()
			.await
			.expect("Failed to send request")
			.json::<GraphQLResponse<collect_data::ResponseData>>()
			.await
			.expect("Failed to parse response");

		match &response.data {
			Some(data) => {
				println!("{:?}", data.collect.status);

				println!("{:?}", data.collect);
			},
			None => println!("no data found"),
		};
	}
}
