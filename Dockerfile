FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
RUN cargo install sqlx-cli

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build the application
FROM chef AS builder
ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN sqlx database reset
RUN cargo build --release -p sandbox

# We do not need the Rust toolchain to run the binary!
FROM lukemathwalker/cargo-chef:latest-rust-1 AS runtime
ARG BITSKIN_API_KEY
ENV BITSKIN_API_KEY=$BITSKIN_API_KEY
WORKDIR /app
COPY --from=builder /app/target/release/sandbox /usr/local/bin
ENTRYPOINT ["/usr/local/bin/sandbox"]
