# Runtime userspace only: no compiler, Rust toolchain, repository or development sysroot.
FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852
COPY linux-runtime-packages.txt /acceptance/linux-runtime-packages.txt
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive xargs apt-get install -y --no-install-recommends < /acceptance/linux-runtime-packages.txt \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends python3 xvfb xdotool imagemagick \
    && rm -rf /var/lib/apt/lists/*
COPY tests /acceptance/tests
RUN chmod -R a+rX /acceptance
USER 1000:1000
WORKDIR /work
ENTRYPOINT ["python3", "/acceptance/tests/native/clean_linux_smoke.py"]
