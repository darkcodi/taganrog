ARG DATABASE_URL
ARG S3_BUCKET_NAME
ARG S3_ACCOUNT_ID
ARG S3_ACCESS_KEY
ARG S3_SECRET_KEY

FROM rust:latest AS builder
ENV DATABASE_URL=$DATABASE_URL
ENV S3_BUCKET_NAME=$S3_BUCKET_NAME
ENV S3_ACCOUNT_ID=$S3_ACCOUNT_ID
ENV S3_ACCESS_KEY=$S3_ACCESS_KEY
ENV S3_SECRET_KEY=$S3_SECRET_KEY
RUN update-ca-certificates
WORKDIR /app
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
RUN cargo build --release

FROM debian:bullseye-slim
ENV DATABASE_URL=$DATABASE_URL
ENV S3_BUCKET_NAME=$S3_BUCKET_NAME
ENV S3_ACCOUNT_ID=$S3_ACCOUNT_ID
ENV S3_ACCESS_KEY=$S3_ACCESS_KEY
ENV S3_SECRET_KEY=$S3_SECRET_KEY
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates
RUN update-ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/taganrog ./
EXPOSE 3000
CMD ["/app/taganrog"]
