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

INSERT INTO chains (chain_name, chain_description) VALUES ('Ethereum', 'Ethereum main net');

INSERT INTO chains (chain_name, chain_description) VALUES ('Cosmos', 'Cosmos main net');

INSERT INTO chains (chain_name, chain_description) VALUES ('Polkadot', 'Polkadot main net');

INSERT INTO chains (chain_name, chain_description) VALUES ('Timechain', 'Timechain main net');

INSERT INTO chains (chain_name, chain_description) VALUES ('Polygon', 'Polygon network');

INSERT INTO task_metadata(task_name, task_description) VALUES ('swap_price', 'get the swap price from one token to other');
