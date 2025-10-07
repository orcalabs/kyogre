FROM postgis/postgis:17-3.5-alpine

ARG MIGRATIONS_DIR_PATH
ARG TEST_MIGRATIONS_DIR_PATH

COPY ${MIGRATIONS_DIR_PATH}/*  /docker-entrypoint-initdb.d/
COPY ${TEST_MIGRATIONS_DIR_PATH}/* /docker-entrypoint-initdb.d/
