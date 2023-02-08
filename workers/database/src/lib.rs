use diesel::{pg::PgConnection, prelude::*};
use dotenvy::dotenv;
use std::env;

mod models;
mod schema;

use models::*;

const DEFAULT_LIMIT: i64 = 1000;

pub fn establish_connection(conn_url: Option<&str>) -> PgConnection {
	dotenv().ok();

	match conn_url {
		Some(url) =>
			PgConnection::establish(url).unwrap_or_else(|_| panic!("Error connecting to {url}")),
		None => {
			let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
			PgConnection::establish(&database_url)
				.unwrap_or_else(|_| panic!("Error connecting to {database_url}"))
		},
	}
}

pub fn get_on_chain_data(conn: &mut PgConnection, min_id: i32) -> Vec<OnChainData> {
	use self::schema::on_chain_data::dsl::*;

	on_chain_data
		.filter(data_id.lt(min_id))
		.limit(DEFAULT_LIMIT)
		.load::<OnChainData>(conn)
		.expect("Error loading data")
}

#[test]
fn get_data() {
	let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
	let mut pg_conn = establish_connection(Some(conn_url));
	let data = get_on_chain_data(&mut pg_conn, 0);

	assert_eq!(data.len() >= 0, true);
}
