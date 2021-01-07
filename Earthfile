FROM rust:1.49
WORKDIR /app

plan:
    RUN cargo install cargo-chef --version 0.1.11
    
    # Compute a lock-like file for our project
    RUN cargo chef prepare --recipe-path recipe.json
    SAVE ARTIFACT recipe.json recipe.json
deps:
    FROM +plan

    COPY . .
    COPY +plan/recipe.json .

    RUN cargo chef cook --release --recipe-path recipe.json
    SAVE ARTIFACT /usr/local/cargo cargo
    SAVE ARTIFACT target target

build:
    FROM +deps

    # Copy over the cached dependencies
    COPY +deps/target .
    COPY +deps/cargo /usr/local/cargo
    # COPY . .

    # ENV SQLX_OFFLINE true
    ENV PORT 9091
    RUN cargo build --release --bin tide-tera
    
    SAVE ARTIFACT target/release/tide-tera tide-tera
    SAVE ARTIFACT templates templates
    SAVE ARTIFACT public public

docker:
    FROM debian:buster-slim
    
    COPY +build/tide-tera tide-tera
    COPY +build/templates templates
    COPY +build/public public
    
    RUN apt-get update -y \
        && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends openssl libcurl4 ca-certificates \
        # Clean up
        && DEBIAN_FRONTEND=noninteractive apt-get autoremove -y && apt-get clean -y && rm -rf /var/lib/apt/lists/*

    EXPOSE 9091
    ENTRYPOINT ["./tide-tera"]
    SAVE IMAGE --push jmnoz/tide-tera:latest