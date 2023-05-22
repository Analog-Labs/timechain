// @generated automatically by Diesel CLI.

diesel::table! {
    _collections_ (name) {
        name -> Text,
        cid -> Int8,
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
    _ql_ (name) {
        name -> Text,
        elements -> Nullable<Text>,
    }
}

diesel::table! {
    chains (chain_id) {
        chain_id -> Int4,
        chain_name -> Varchar,
        chain_description -> Varchar,
    }
}

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
    task_metadata (task_metadata_id) {
        task_metadata_id -> Int4,
        task_name -> Varchar,
        task_description -> Varchar,
    }
}

diesel::table! {
    tasks (task_id) {
        task_id -> Int4,
        chain_id -> Nullable<Int4>,
        task_metadata_id -> Nullable<Int4>,
        task_name -> Varchar,
        arguments -> Varchar,
        frequency -> Nullable<Int4>,
    }
}

diesel::table! {
    the_last_cycle (id) {
        id -> Int4,
        cycle -> Nullable<Int8>,
        updated -> Nullable<Timestamp>,
    }
}

diesel::joinable!(_collections_ -> _feeds_ (cid));
diesel::joinable!(on_chain_data -> tasks (task_id));
diesel::joinable!(tasks -> chains (chain_id));
diesel::joinable!(tasks -> task_metadata (task_metadata_id));

diesel::allow_tables_to_appear_in_same_query!(
    _collections_,
    _feeds_,
    _ql_,
    chains,
    on_chain_data,
    task_metadata,
    tasks,
    the_last_cycle,
);
