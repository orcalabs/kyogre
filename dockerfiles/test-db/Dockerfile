FROM postgis/postgis:15-3.4-alpine

COPY src/postgres/migrations/* /docker-entrypoint-initdb.d/
COPY src/postgres/test_migrations/* /docker-entrypoint-initdb.d/
