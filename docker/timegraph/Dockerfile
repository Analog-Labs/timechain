FROM analoglabs/timegraph:latest
ENV RUST_LOG debug
ENV RUST_BACKTRACE 1
COPY ./registry.yml /registry.yml
COPY ./entrypoint.sh /entrypoint.sh
RUN chmod a+x /entrypoint.sh /wait
ENTRYPOINT ["/entrypoint.sh"]
