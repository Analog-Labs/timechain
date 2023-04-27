use diesel::{insert_into, prelude::*};
use dotenvy::dotenv;
use std::env;

pub mod models;
mod schema;

use models::*;

const DEFAULT_LIMIT: i64 = 1000;

pub fn establish_connection(conn_url: Option<&str>) -> Result<PgConnection, String> {
	dotenv().ok();

	match conn_url {
		Some(url) =>
			PgConnection::establish(url).map_err(|_| format!("Error connecting to {url}.")),
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

pub fn write_data_to_db(conn: &mut PgConnection, record: Feeds) {
	use self::schema::_feeds_::dsl::*;

	insert_into(_feeds_)
		.values(record)
		.execute(conn)
		.expect("Failed to insert new user");
}

#[ignore]
#[test]
fn get_data() {
	let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
	let mut pg_conn = establish_connection(Some(conn_url)).unwrap();
	let _data = get_on_chain_data(&mut pg_conn, 0).unwrap();
}

#[ignore]
#[test]
fn insert_data() {
	let conn_url: &str = "postgresql://localhost/timechain?user=postgres&password=postgres";
	let mut pg_conn = establish_connection(Some(conn_url)).unwrap();

	let id = 1;
	let hash = "some_hash".to_owned();
	let task = b"some_task".to_vec();
	let validity = 123;
	let timestamp = Some(
		chrono::NaiveDateTime::parse_from_str("2023-04-25 15:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
	);
	let cycle = Some(456);

	let feeds = Feeds {
		id,
		hash,
		task,
		timestamp,
		validity,
		cycle,
	};

	write_data_to_db(&mut pg_conn, feeds);
}
