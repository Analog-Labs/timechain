FROM debian:stable-slim
MAINTAINER Analog Devs version: 0.1
ADD https://github.com/ufoscout/docker-compose-wait/releases/download/2.9.0/wait /
RUN apt-get update -y && \
	apt-get install -y curl nodejs yarn npm && \
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete
COPY set-keys.sh /set-keys.sh
COPY nodejs /nodejs
RUN chmod a+x /wait && chmod a+x /set-keys.sh
RUN cd /nodejs && npm install
ENTRYPOINT ["/set-keys.sh"]
