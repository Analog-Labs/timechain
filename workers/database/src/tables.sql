CREATE DATABASE timechain
    WITH
    OWNER = analog
    ENCODING = 'UTF8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1
    IS_TEMPLATE = False;

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
	time_stamp timestamp NOT NULL,
	data_value VARCHAR (100) NOT NULL,
	FOREIGN KEY(task_id) REFERENCES tasks(task_id)
);