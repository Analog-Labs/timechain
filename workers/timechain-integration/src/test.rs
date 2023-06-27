#[cfg(test)]
mod tests {
	use crate::query::{collect_data, CollectData};

	use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
    
    #[tokio::test]
	async fn test_collect_data() {
		// Prepare the input variables
		let variables = collect_data::Variables {
			collection: "QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7".to_owned(),
			block: 1,
			cycle: 17,
			task_id: 3,
			data: vec!["1".to_owned()],
		};

		// Build the GraphQL request
		let request = CollectData::build_query(variables);

		// Execute the GraphQL request
		let response = reqwest::Client::new()
			.post("http://localhost:8009/graphql") // Replace with your GraphQL endpoint URL
			.json(&request)
			.send()
			.await
			.expect("Failed to send request")
			.json::<GraphQLResponse<collect_data::ResponseData>>()
			.await
			.expect("Failed to parse response");

		 match &response.data {
			Some(data) => {
                println!("{:?}",data.collect.status);

                println!("{:?}",data.collect);
            },
			None => println!("no deta found"),
		};
	}
}
