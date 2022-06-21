FROM debian:stretch-slim

MAINTAINER Analog Devs version: 0.1

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user
	useradd -m -u 1000 -U -s /bin/sh -d /timechain-node timechain-node

# add timechain-node binary to docker image
COPY ./timechain-node /usr/local/bin

USER timechain-node

# check if executable works in this container
RUN /usr/local/bin/timechain-node --version

EXPOSE 30333 9933 9944
VOLUME ["/timechain-node"]

ENTRYPOINT ["/usr/local/bin/timechain-node"]