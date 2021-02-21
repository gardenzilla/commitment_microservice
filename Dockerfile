FROM debian:buster-slim
WORKDIR /usr/local/bin
COPY ./target/release/loyalty_microservice /usr/local/bin/loyalty_microservice
RUN apt-get update && apt-get install -y
RUN apt-get install curl -y
STOPSIGNAL SIGINT
ENTRYPOINT ["loyalty_microservice"]