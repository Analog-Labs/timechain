FROM docker.io/library/ubuntu:22.04 AS builder

WORKDIR /builds

# config for clang 15
COPY base-ci-linux-config.toml /root/.cargo/config

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    RUST_VERSION=1.70.0 \
    RUST_NIGHTLY=2023-05-23 \
    RUST_BACKTRACE=1 \
    CC=clang-15 \
    CXX=clang-15 \
    GOLANG_VERSION=1.20.6 \
    GOPATH=/go \
    NODE_VERSION=18.16.1 \
    YARN_VERSION=1.22.19 \
    PATH=/usr/local/cargo/bin:$GOPATH/bin:/usr/local/go/bin:$PATH

# install tools and dependencies
RUN set -eux; \
	apt-get -y update; \
	dpkgArch="$(dpkg --print-architecture)"; \
	apt-get install -y --no-install-recommends \
        g++ gcc libc6-dev libboost-all-dev libz3-dev wget musl-tools \
		libssl-dev libsasl2-dev make cmake graphviz build-essential \
		git pkg-config curl time rhash ca-certificates jq \
		python3 python3-pip lsof ruby ruby-bundler git-restore-mtime xz-utils unzip gnupg protobuf-compiler && \
# add clang 15 repo
	curl -s https://apt.llvm.org/llvm-snapshot.gpg.key | gpg --dearmor -o /usr/share/keyrings/llvm-snapshot.gpg; \
	echo "deb [arch=${dpkgArch} signed-by=/usr/share/keyrings/llvm-snapshot.gpg] http://apt.llvm.org/jammy/ llvm-toolchain-jammy-15 main" >> /etc/apt/sources.list.d/llvm-toolchain-jammy-15.list; \
	apt-get -y update; \
	apt-get install -y --no-install-recommends \
		clang-15 lldb-15 lld-15 libclang-15-dev && \
# add non-root user
	groupadd -g 1000 nonroot && \
	useradd -u 1000 -g 1000 -s /bin/bash -m nonroot && \
# set a link to clang
	update-alternatives --install /usr/bin/cc cc /usr/bin/clang-15 100; \
# set a link to ldd
	update-alternatives --install /usr/bin/ld ld /usr/bin/ld.lld-15 100; \
# install rustup, use minimum components
	case "${dpkgArch##*-}" in \
        amd64) \
          rustArch='x86_64-unknown-linux-gnu'; \
          rustTargetArch='x86_64-unknown-linux-musl'; \
          rustupSha256='0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db'; \
          muslLinker='x86_64-linux-musl-gcc'; \
          golangUrl='https://dl.google.com/go/go1.20.6.linux-amd64.tar.gz'; \
          goSha256='b945ae2bb5db01a0fb4786afde64e6fbab50b67f6fa0eb6cfa4924f16a7ff1eb'; \
          nodejsArch='x64';; \
        armhf) \
          rustArch='armv7-unknown-linux-gnueabihf'; \
          rustTargetArch='armv7-unknown-linux-musleabi'; \
          rustupSha256='f21c44b01678c645d8fbba1e55e4180a01ac5af2d38bcbd14aa665e0d96ed69a'; \
          golangUrl='https://dl.google.com/go/go1.20.6.linux-armv6l.tar.gz'; \
          goSha256='669902f5c8efefbd5d5fd078db01e34355af3693e48659b89593da7db367c488'; \
          nodejsArch='armv7l';; \
        arm64) \
          rustArch='aarch64-unknown-linux-gnu'; \
          rustTargetArch='aarch64-unknown-linux-musl'; \
          rustupSha256='673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800'; \
          muslLinker='aarch64-linux-musl-gcc'; \
          golangUrl='https://dl.google.com/go/go1.20.6.linux-arm64.tar.gz'; \
          goSha256='4e15ab37556e979181a1a1cc60f6d796932223a0f5351d7c83768b356f84429b'; \
          nodejsArch='arm64';; \
        i386) \
          rustArch='i686-unknown-linux-gnu'; \
          rustTargetArch='i686-unknown-linux-musl'; \
          rustupSha256='e7b0f47557c1afcd86939b118cbcf7fb95a5d1d917bdd355157b63ca00fc4333'; \
          golangUrl='https://dl.google.com/go/go1.20.6.linux-386.tar.gz'; \
          goSha256='2e27c9db1defbf4d58e907f9843bf60a1ce229688f8463bf24d6a0a19dc949de'; \
          nodejsArch='x86';; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
# gpg keys listed at https://github.com/nodejs/node#release-keys
    for key in \
        4ED778F539E3634C779C87C6D7062848A1AB005C \
        141F07595B7B3FFE74309A937405533BE57C7D57 \
        74F12602B6F1C4E913FAA37AD3A89613643B6201 \
        DD792F5973C6DE52C432CBDAC77ABFA00DDBF2B7 \
        61FC681DFB92A079F1685E77973F295594EC4689 \
        8FCCA13FEF1D0C2E91008E09770F7A9A5AE15600 \
        C4F0DFFF4E8C1A8236409D08E73BC641CC11F4C8 \
        890C08DB8579162FEE0DF9DB8BEAB4DFCF555EF4 \
        C82FA3AE1CBEDC6BE46B9360C43CEC45C17AB93C \
        108F52B48DB57BB0CC439B2997B01419BD92F80A \
        6A010C5166006599AA17F08146C2130DFD2497F5 \
    ; do \
        gpg --batch --keyserver hkps://keys.openpgp.org --recv-keys "$key" || \
        gpg --batch --keyserver keyserver.ubuntu.com --recv-keys "$key" ; \
    done; \
    url="https://static.rust-lang.org/rustup/archive/1.26.0/${rustArch}/rustup-init"; \
	curl -L "$url" -o rustup-init; \
	echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
	chmod +x rustup-init; \
	./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
	rm rustup-init; \
	chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
	chown -R root:nonroot ${RUSTUP_HOME} ${CARGO_HOME}; \
	chmod -R g+w ${RUSTUP_HOME} ${CARGO_HOME}; \
# install `rust-src` component for ui test
	rustup component add rust-src rustfmt clippy && \
# install wasm target into default (stable) toolchain
    rustup target add wasm32-unknown-unknown && \
# install specific Rust nightly, default is stable, use minimum components
	rustup toolchain install nightly-${RUST_NIGHTLY} --profile minimal --component rustfmt clippy && \
# install wasm target into nightly toolchain
    rustup target add wasm32-unknown-unknown --toolchain "nightly-${RUST_NIGHTLY}" && \
# "alias" pinned nightly toolchain as nightly
	ln -s /usr/local/rustup/toolchains/nightly-${RUST_NIGHTLY}-${rustArch} /usr/local/rustup/toolchains/nightly-${rustArch} && \
# install wasm-pack
    cargo install wasm-pack --version 0.12.1 && \
# versions
	rustup show; \
	rustup --version; \
	cargo --version; \
	rustc --version; \
# cargo clean up
# removes compilation artifacts cargo install creates (>250M)
	rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git" /root/.cache/sccache; \
# Install NodeJS
    curl -fsSLO --compressed "https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-$nodejsArch.tar.xz" && \
    curl -fsSLO --compressed "https://nodejs.org/dist/v$NODE_VERSION/SHASUMS256.txt.asc" && \
    gpg --batch --decrypt --output SHASUMS256.txt SHASUMS256.txt.asc && \
    grep " node-v$NODE_VERSION-linux-$nodejsArch.tar.xz\$" SHASUMS256.txt | sha256sum -c - && \
    tar -xJf "node-v$NODE_VERSION-linux-$nodejsArch.tar.xz" -C /usr/local --strip-components=1 --no-same-owner && \
    rm "node-v$NODE_VERSION-linux-$nodejsArch.tar.xz" SHASUMS256.txt.asc SHASUMS256.txt && \
    ln -s /usr/local/bin/node /usr/local/bin/nodejs && \
    node --version && \
    npm --version && \
# Install Yarn
    curl -fsSLO --compressed "https://yarnpkg.com/downloads/$YARN_VERSION/yarn-v$YARN_VERSION.tar.gz" && \
    curl -fsSLO --compressed "https://yarnpkg.com/downloads/$YARN_VERSION/yarn-v$YARN_VERSION.tar.gz.asc" && \
    gpg --batch --verify yarn-v$YARN_VERSION.tar.gz.asc yarn-v$YARN_VERSION.tar.gz && \
    mkdir -p /opt && \
    tar -xzf yarn-v$YARN_VERSION.tar.gz -C /opt/ && \
    ln -s /opt/yarn-v$YARN_VERSION/bin/yarn /usr/local/bin/yarn && \
    ln -s /opt/yarn-v$YARN_VERSION/bin/yarnpkg /usr/local/bin/yarnpkg && \
    rm yarn-v$YARN_VERSION.tar.gz.asc yarn-v$YARN_VERSION.tar.gz && \
    yarn --version && \
# Install Go
    wget -O go.tgz.asc "$golangUrl.asc"; \
    wget -O go.tgz "$golangUrl" --progress=dot:giga; \
    echo "$goSha256 *go.tgz" | sha256sum -c -; \
# https://github.com/golang/go/issues/14739#issuecomment-324767697
    GNUPGHOME="$(mktemp -d)"; export GNUPGHOME; \
# https://www.google.com/linuxrepositories/
    gpg --batch --keyserver keyserver.ubuntu.com --recv-keys 'EB4C 1BFD 4F04 2F6D DDCC  EC91 7721 F63B D38B 4796'; \
# let's also fetch the specific subkey of that key explicitly that we expect "go.tgz.asc" to be signed by, just to make sure we definitely have it
    gpg --batch --keyserver keyserver.ubuntu.com --recv-keys '2F52 8D36 D67B 69ED F998  D857 78BD 6547 3CB3 BD13'; \
    gpg --batch --verify go.tgz.asc go.tgz; \
    gpgconf --kill all; \
    rm -rf "$GNUPGHOME" go.tgz.asc; \
    tar -C /usr/local -xzf go.tgz; \
    rm go.tgz; \
    go version && \
    mkdir -p "$GOPATH/src" "$GOPATH/bin" && chmod -R 1777 "$GOPATH" && \
# apt clean up
	apt-get autoremove -y && \
	apt-get clean && \
	rm -rf /var/lib/apt/lists/* && \
# cargo clean up
# removes compilation artifacts cargo install creates (>250M)
	rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git"
