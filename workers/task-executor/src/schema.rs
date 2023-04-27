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
