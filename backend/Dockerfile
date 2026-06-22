# build stage
FROM rust:slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# runtime stage
FROM debian:bookworm-slim
# git is required because the backend shells out to it for clone/fetch/show
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/lightning_git /app/lightning_git

# ephemeral clone dir; the backend populates it as projects are added
RUN mkdir -p /repos
ENV GIT_REPO_DEV=/repos
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080

CMD ["/app/lightning_git"]
