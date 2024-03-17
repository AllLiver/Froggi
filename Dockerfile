FROM rust:slim

WORKDIR /usr/src/froggi
COPY . .

RUN cargo install --path .
RUN cargo clean
RUN rm -rf /usr/src/froggi/.git

RUN apt-get update -y && apt-get install -y nano

EXPOSE 8080
CMD ["froggi"]