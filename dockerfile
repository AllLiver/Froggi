FROM rust:1.79
WORKDIR /froggi
COPY . .

RUN apt-get update -y && apt-get install -y nano libssl-dev pkg-config

RUN cargo install --path .

EXPOSE 3000

CMD ["froggi"]
