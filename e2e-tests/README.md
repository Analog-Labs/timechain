# e2e tests

## Useful commands

List integration tests:

```sh
cargo test -- --list
```

Running an integration test:

```sh
./scripts/build_docker.sh
cargo test -p e2e-tests TEST_NAME
```

Cleaning up after aborting a test:

```sh
docker ps -q | xargs docker stop
docker container ls -a -q | xargs docker rm
docker network ls -q | xargs docker network rm
```

Inspecting container logs in grafana:

```sh
docker compose -f docker-compose-grafana.yml up -d
browser http://127.0.0.1:3000
# login with admin admin123
# Left side menu > Explore > Loki (data source)
```

Using tc-cli for debugging:

```sh
# after starting the test it should log:
> 2025-03-23T10:55:55.899924Z  INFO e2e_tests: tempdir: /tmp/.tmpcZyl3c
cd /tmp/.tmpcZyl3c
tc-cli --env . log container tmpcZyl3c-chronicle
```
