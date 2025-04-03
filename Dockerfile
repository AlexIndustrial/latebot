FROM rust:1.75-slim-bullseye as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/latebot .

CMD ["./latebot"] 