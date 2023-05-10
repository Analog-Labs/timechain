FROM paritytech/ci-linux:production as pre-build
ENV DEBIAN_FRONTEND=noninteractive
RUN rustup toolchain uninstall nightly && rustup install nightly
RUN apt update -y && apt install libpq-dev -y
COPY ./cargo.toml.tar /
COPY ./Cargo.lock /build/
RUN cd /build && tar xf ../cargo.toml.tar && cargo fetch --locked

FROM pre-build as build
COPY . /build/
RUN cd /build/ && cargo build --release --features runtime-benchmarks

FROM debian:stretch-slim
MAINTAINER Analog Devs version: 0.1
ENV RUST_BACKTRACE 1

RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl && \
        libpq && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user
	useradd -m -u 1000 -U -s /bin/sh -d /timechain timechain-node

COPY --from=build /build/target/release/timechain-node /timechain-node
USER timechain-node
EXPOSE 30333 9933 9944
ENTRYPOINT ["/timechain-node"]
