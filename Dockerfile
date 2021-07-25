FROM fedora:34
RUN dnf update -y && dnf clean all -y
WORKDIR /usr/local/bin
COPY ./target/release/commitment_microservice /usr/local/bin/commitment_microservice
STOPSIGNAL SIGINT
ENTRYPOINT ["commitment_microservice"]
