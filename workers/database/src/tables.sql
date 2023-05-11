CREATE DATABASE timechain WITH OWNER = analog ENCODING = 'UTF8' LC_COLLATE = 'en_US.utf8' LC_CTYPE = 'en_US.utf8' TABLESPACE = pg_default CONNECTION
LIMIT
    = -1 IS_TEMPLATE = False;

CREATE TABLE chains (
    chain_id serial PRIMARY KEY,
    chain_name VARCHAR (20) NOT NULL,
    chain_description VARCHAR (100) NOT NULL
);

CREATE TABLE task_metadata (
    task_metadata_id serial PRIMARY KEY,
    task_name VARCHAR (20) NOT NULL,
    task_description VARCHAR (100) NOT NULL
);

CREATE TABLE tasks (
    task_id serial PRIMARY KEY,
    chain_id INT,
    task_metadata_id INT,
    task_name VARCHAR (20) NOT NULL,
    arguments VARCHAR (500) NOT NULL,
    frequency INT,
    FOREIGN KEY(chain_id) REFERENCES chains(chain_id),
    FOREIGN KEY(task_metadata_id) REFERENCES task_metadata(task_metadata_id)
);

CREATE TABLE on_chain_data (
    data_id serial PRIMARY KEY,
    task_id INT NOT NULL,
    block_number INT NOT NULL,
    time_stamp VARCHAR (200) NOT NULL,
    data_value VARCHAR (100) NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(task_id)
);

CREATE SEQUENCE IF NOT EXISTS _object_id_ AS BIGINT MINVALUE 100;

CREATE TABLE IF NOT EXISTS _feeds_ (
    id BIGINT PRIMARY KEY DEFAULT nextval('_object_id_'),
    hash CHAR(40) NOT NULL UNIQUE,
    task BYTEA NOT NULL,
    validity BIGINT NOT NULL,
    timestamp TIMESTAMP,
    cycle BIGINT
);

CREATE TABLE IF NOT EXISTS _collections_ (
    name TEXT PRIMARY KEY,
    cid BIGINT NOT NULL,
    FOREIGN KEY(cid) REFERENCES _feeds_(id)
);

CREATE INDEX IF NOT EXISTS _collections_hashes_ ON _feeds_ (hash);

CREATE INDEX IF NOT EXISTS _collections_cycles_ ON _feeds_ (cycle);

CREATE TABLE IF NOT EXISTS the_last_cycle(
    id INTEGER PRIMARY KEY DEFAULT 0,
    cycle BIGINT,
    updated TIMESTAMP
);

CREATE TABLE IF NOT EXISTS _ql_ (
    name TEXT PRIMARY KEY,
    elements TEXT
);