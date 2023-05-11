FROM paritytech/ci-linux:production as pre-build
ENV DEBIAN_FRONTEND=noninteractive
RUN apt update -y && apt install libpq-dev -y
ADD https://github.com/PowerShell/PowerShell/releases/download/v7.3.4/powershell-7.3.4-linux-x64.tar.gz /pwsh7/7.tar.gz
RUN cd /pwsh7 && tar xzf 7.tar.gz
COPY ./cargo.toml.tar /
COPY ./Cargo.lock /build/
COPY ./rust-toolchain.toml /build/
COPY ./.github/update-rust.ps1 /build/
COPY ./.github/config.txt /build/.github/
RUN cd /build && ls -la .github && /pwsh7/pwsh -nologo update-rust.ps1
RUN cd /build && tar xf ../cargo.toml.tar && cargo fetch --locked

FROM pre-build as build
COPY . /build/
RUN cd /build/ && cargo build --release --features runtime-benchmarks

FROM debian:stable-slim
MAINTAINER Analog Devs version: 0.1
ENV RUST_BACKTRACE 1

ADD https://github.com/ufoscout/docker-compose-wait/releases/download/2.9.0/wait /wait
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl \
        libpq-dev  && \
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	useradd -m -u 1000 -U -s /bin/sh -d /timechain timechain-node; \
    chmod a+x /wait

COPY --from=build /build/target/release/timechain-node /node
USER timechain-node
EXPOSE 30333 9933 9944
ENTRYPOINT ["/node"]
