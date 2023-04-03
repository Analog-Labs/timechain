diesel::table! {
	on_chain_data (data_id) {
		data_id -> Int4,
		task_id -> Int4,
		block_number -> Int4,
		time_stamp -> Timestamp,
		data_value -> Varchar,
	}
}
