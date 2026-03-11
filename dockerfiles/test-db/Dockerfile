FROM postgis/postgis:17-3.5-alpine

ARG MIGRATIONS_DIR_PATH
ARG TEST_MIGRATIONS_DIR_PATH
ARG TEMPLATE_DB_PATH

COPY ${MIGRATIONS_DIR_PATH}/*  /docker-entrypoint-initdb.d/
COPY ${TEST_MIGRATIONS_DIR_PATH}/* /docker-entrypoint-initdb.d/
COPY ${TEMPLATE_DB_PATH} /docker-entrypoint-initdb.d/
