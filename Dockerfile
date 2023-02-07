ARG DATABASE_URL

FROM rust:latest AS builder
ENV DATABASE_URL=$DATABASE_URL
RUN update-ca-certificates
WORKDIR /app
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
RUN cargo build --release

FROM debian:bullseye-slim
ENV DATABASE_URL=$DATABASE_URL
WORKDIR /app
COPY --from=builder /app/target/release/taganrog ./
EXPOSE 3000
CMD ["/app/taganrog"]
