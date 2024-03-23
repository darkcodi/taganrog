FROM rust:1.77.0

WORKDIR /usr/src/taganrog
COPY . .
RUN cargo install --path .

RUN mkdir -p /workdir/uploads
RUN taganrog config set work-dir /workdir
RUN taganrog config set upload-dir /workdir/uploads

ENTRYPOINT ["taganrog", "web-ui", "-v"]