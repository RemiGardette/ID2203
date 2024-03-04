FROM rust:1.76.0-slim-buster

WORKDIR /usr/src/myapp
COPY . .

CMD ["cargo", "run"]

