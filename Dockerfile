FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
RUN cargo install sqlx-cli

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build the application
FROM chef AS builder
ARG DATABASE_URL
ARG BITSKIN_API_KEY

# Set environment variables for the build stage
ENV DATABASE_URL=$DATABASE_URL
ENV BITSKIN_API_KEY=$BITSKIN_API_KEY

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN sqlx migrate run
RUN cargo build --release --bin bot

# We do not need the Rust toolchain to run the binary!
FROM lukemathwalker/cargo-chef:latest-rust-1 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/bot /usr/local/bin
ENTRYPOINT ["/usr/local/bin/bot"]
