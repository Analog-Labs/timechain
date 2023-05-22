diesel::table! {
	on_chain_data (data_id) {
		data_id -> Int4,
		task_id -> Int4,
		block_number -> Int4,
		time_stamp -> Timestamp,
		data_value -> Varchar,
	}
}

diesel::table! {
	_feeds_ (id) {
		id -> Int8,
		hash -> Bpchar,
		task -> Bytea,
		validity -> Int8,
		timestamp -> Nullable<Timestamp>,
		cycle -> Nullable<Int8>,
	}
}

diesel::table! {
	c_hash(block_id) {
		cycle -> Nullable<Int8>,
		block_id -> Int8,
		data -> Bpchar,
	}
}
