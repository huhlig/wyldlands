# Stage 1: Build
FROM postgres:18-trixie AS build

ENV PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config

RUN apt-get update && apt-get install -y \
    git build-essential postgresql-server-dev-18 \
    libreadline-dev zlib1g-dev flex bison \
    curl gnupg unixodbc-dev

# Build and Install Apache AGE
RUN git clone --branch release/PG18/1.7.0 https://github.com/apache/age.git /age && \
    cd /age && \
    make && make install

# Build and Install pgvector extension
RUN git clone --branch v0.8.1 https://github.com/pgvector/pgvector.git /pgvector && \
    cd /pgvector && \
    make && make install


# Stage 2: Production
FROM postgres:18-trixie

RUN apt-get update \
    && apt-get install -y --no-install-recommends --no-install-suggests locales

RUN echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && locale-gen \
    && update-locale LANG=en_US.UTF-8

ENV LANG=en_US.UTF-8
ENV LC_COLLATE=en_US.UTF-8
ENV LC_CTYPE=en_US.UTF-8

COPY --from=build /usr/lib/postgresql/18/lib/vector.so /usr/lib/postgresql/18/lib/
COPY --from=build /usr/share/postgresql/18/extension/vector--0.8.1.sql /usr/share/postgresql/18/extension/
COPY --from=build /usr/share/postgresql/18/extension/vector.control /usr/share/postgresql/18/extension/

COPY --from=build /usr/lib/postgresql/18/lib/age.so /usr/lib/postgresql/18/lib/
COPY --from=build /usr/share/postgresql/18/extension/age--1.7.0.sql /usr/share/postgresql/18/extension/
COPY --from=build /usr/share/postgresql/18/extension/age.control /usr/share/postgresql/18/extension/

RUN printf 'CREATE EXTENSION IF NOT EXISTS age;\nCREATE EXTENSION IF NOT EXISTS hstore;\nCREATE EXTENSION IF NOT EXISTS vector;\n' >> /docker-entrypoint-initdb.d/00-create-extensions.sql

CMD ["postgres", "-c", "shared_preload_libraries=age,vector,pg_stat_statements,auto_explain"]