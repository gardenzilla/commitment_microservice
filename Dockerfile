FROM debian:buster-slim
WORKDIR /usr/local/bin
COPY ./target/release/commitment_microservice /usr/local/bin/commitment_microservice
RUN apt-get update && apt-get install -y
RUN apt-get install curl -y
STOPSIGNAL SIGINT
ENTRYPOINT ["commitment_microservice"]