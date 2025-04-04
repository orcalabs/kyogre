## Building

The system in structured in a single Cargo workspace and can be built with cargo
while standing in the `src` directory.

```
cargo build
```

The following build dependencies are required:

- OpenSSL
- protobuf
- cargo

[NVIDIA container toolkit](https://github.com/NVIDIA/nvidia-container-toolkit)
is also required for local deployment and testing.

## Local deployment

For local development with real data the `engine` container can be started and
will scrape all datasources and store them in your local postgres instance. The
only data missing will be AIS position data which only exists in our azure
database. The engine will use quite abit of time on a fresh run, consult the
docker logs to view its progress.

Commands to run a local instance:

```
docker-compose up -d postgres
docker-compose -d --build engine
```

For a local api instance you will also need to start the duckdb service:

```
docker-compose up -d postgres
docker-compose up -d duckdb
docker-compose up -d api
```

The api will then be availabe at `http://localhost:8080`, swagger at
`http://localhost:8080/swagger-ui/`.

## Complile time SQL queries

We use [sqlx](https://github.com/launchbadge/sqlx) for all database interactions
and migrations. To be able to build the system you will need a running and
migrated database, a dedicated migration db service exists in the compose file
in this repository. Run the following commands to start and migrate your local
database:

```
cd src/postgres
docker-compose up -d migration-db
cargo sqlx migrate run
```

See the sqlx-cli crate [documentation](https://crates.io/crates/sqlx-cli) for
more information on how it operates.

The following build dependencies are required:

- [sqlx-cli](https://crates.io/crates/sqlx-cli)

## Testing

We use [Dockertest](https://github.com/orcalabs/dockertest-rs) as our test
library which will manage container lifecycles in our tests. All tests will
reuse the same postgres container across multiple `cargo test` invocations and
create their own isolated database within that instance. Prior to running tests
remember to build the `test-db` service if you've added any migrations or if its
the first run:

```
docker-compose build test-db
```

By default not all tests are run, to enable cache tests and db migrations run the following
(you will need the master db state image):

```
echo <GITHUB_TOKEN> | docker login ghcr.io/orcalabs/kyogre -u <USERNAME> --password-stdin
docker pull ghcr.io/orcalabs/kyogre/test-postgres:master
cargo test --features all-tests
```

If you experience `Too many open files` errors from tests either increase your
max amount of file descriptors (`ulimit -n NUM`, linux) or run `cargo test` with
limited test threads `cargo test -- --test-threads=NUM`

The following build dependencies are required:

- docker
- docker buildkit
- docker-compose (to simplify image building)
- python 3.9 with the packages specified
  [here](scripts/python/fishing_predictor/requirements.txt)

## Cloudsmith access

To access the private Orca Labs packages you will need to be invited to the Orca
Labs cloudsmith organization, then add your credentials to your credentials
helper:

```
git config --global credential.helper store

echo "https://USERNAME:API-KEY@dl.cloudsmith.io" > ~/.git-credentials
```

## Documentation

To view the code documentation run the following from the `src` directory (the docs will popup in your browser):

```
cargo doc --open --no-deps --workspace --document-private-items
```
