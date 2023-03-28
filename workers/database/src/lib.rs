use diesel::{pg::PgConnection, prelude::*};
use dotenvy::dotenv;
use std::env;

mod models;
mod schema;

use models::*;

const DEFAULT_LIMIT: i64 = 1000;

pub fn establish_connection(conn_url: Option<&str>) -> Result<PgConnection, String> {
	dotenv().ok();

	match conn_url {
		Some(url) => {
			PgConnection::establish(url).map_err(|_| format!("Error connecting to {url}."))
		},
		None => {
			let url = env::var("DATABASE_URL").map_err(|_| "Error the DATABASE_URL not set.")?;
			PgConnection::establish(&url).map_err(|_| format!("Error connecting to {url}"))
		},
	}
}

pub fn get_on_chain_data(
	conn: &mut PgConnection,
	min_id: i32,
) -> Result<Vec<OnChainData>, &'static str> {
	use self::schema::on_chain_data::dsl::*;

	on_chain_data
		.filter(data_id.lt(min_id))
		.limit(DEFAULT_LIMIT)
		.load::<OnChainData>(conn)
		.map_err(|_| "Can't load data from on_chain_data table.")
}

#[ignore]
#[test]
fn get_data() {
	let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
	let mut pg_conn = establish_connection(Some(conn_url)).unwrap();
	let _data = get_on_chain_data(&mut pg_conn, 0).unwrap();
}
