FROM rust:1.79 AS builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/froggi /usr/local/bin/froggi
COPY --from=builder /usr/local/cargo/bin/froggi-worker /usr/local/bin/froggi-worker

RUN apt-get update -y && apt-get install -y nano

EXPOSE 3000

CMD ["froggi"]
