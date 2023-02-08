use diesel::{pg::PgConnection, prelude::*};
use dotenvy::dotenv;
use std::env;

mod models;
mod schema;

use models::*;

const DEFAULT_LIMIT: i64 = 1000;

pub fn establish_connection() -> PgConnection {
	dotenv().ok();

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	PgConnection::establish(&database_url)
		.unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}


pub fn get_on_chain_data(min_id: i32) -> Vec::<OnChainData> {
    use self::schema::on_chain_data::dsl::*;

    let connection = &mut establish_connection();

    let results = on_chain_data
        .filter(data_id.lt(min_id))
        .limit(DEFAULT_LIMIT)
        .load::<OnChainData>(connection)
        .expect("Error loading posts");
    
    results
}


