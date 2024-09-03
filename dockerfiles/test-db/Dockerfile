FROM postgis/postgis:16-3.4-alpine

COPY src/postgres/migrations/* /docker-entrypoint-initdb.d/
COPY src/postgres/test_migrations/* /docker-entrypoint-initdb.d/
