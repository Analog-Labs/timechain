# Timenode deployment

`deploy-validators.yaml` is a Ansible playbook to do a rolling update of Docker images for the timenodes.

Prerequisites:
- Ansible

## Usage

To run the playbook, make sure to have a inventory file describing the hosts, SSH key paths and variables.
An example of how the inventory could look like:

```ini
[bootnodes]
BOOTNODE_IP validator_id="alice"

[validators]
VALIDATOR1_IP validator_id="bob"
VALIDATOR2_IP validator_id="charlie"

[all:vars]
ansible_ssh_user=ubuntu
```

Once the inventory is set up, be sure to have all the necessary variables which are specified under the `vars` section inside the playbook inside the `--extra-vars` arguments.

To run the playbook:

`ansible-playbook -i inventory.ini --extra-vars "db_url=$DATABASE_URL" ... deploy-validators.yaml`

After running the playbook, inspect the logs from Ansible for any errors. 

## Common issues

### Ansible created another container instead of replacing the old one

The Ansible playbook checks for the container named `timenode` on the Docker host. If the container is named differently, be sure to add a extra variable on the command `--extra-vars "container_name=custom-container"`

### `server unreachable` or `permission denied`

Check if the servers are running, check if port 22 is open and check if the path to the private key exists and holds a valid SSH key. To customize the key path, under the `[all:vars]` section, add `ansible_ssh_private_key_file=$KEY_PATH`.

### After upgrade, nodes won't sync

Be sure to check the logs from the containers. Some of the common mistakes are:
- wrong IP addresses (if it's not a stable IP, it might have changed)
- ports not open (RPC - 9933, node - 30333)
- firewall blocking
