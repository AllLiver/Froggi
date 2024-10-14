FROM rust:1.79
WORKDIR /froggi
COPY . .

RUN cargo install --path .

RUN apt-get update -y && apt-get install -y nano

EXPOSE 3000

CMD ["froggi"]
