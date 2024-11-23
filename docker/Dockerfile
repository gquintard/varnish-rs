FROM ubuntu:22.04

RUN    apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y \
        apt-transport-https \
        bash-completion \
        build-essential \
        clang \
        cmake \
        curl \
        git \
        gnupg \
        llvm \
        pkg-config \
        sudo `# allows dev to install more packages without switching to root or rebuilding container` \
    && : # end of the RUN cmd - easier to keep a colon at the end of the list, than to keep the backslashes in check

# Pass in the VARNISH_VERSION build arg to specify the version of Varnish to install
ARG VARNISH_VERSION=76
# These steps should be the same as in the CI workflow
RUN set -ex; \
    curl -sSf "https://packagecloud.io/install/repositories/varnishcache/varnish${VARNISH_VERSION}/script.deb.sh" | sudo bash; \
    echo 'Package: varnish varnish-dev\nPin: origin "packagecloud.io"\nPin-Priority: 1001' > /etc/apt/preferences.d/varnish; \
    cat /etc/apt/preferences.d/varnish; \
    apt-cache policy varnish; \
    sudo apt-get install -y varnish varnish-dev;

ARG USERNAME=user
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Create docker user wuth sudo rights as passed in by the build command
# This was modeled on https://code.visualstudio.com/remote/advancedcontainers/add-nonroot-user
# On a Mac, USER_GID might already exist, so ignore it if it fails (--force)
RUN groupadd --force --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

# This allows users to `docker run` without specifying -u and -g
USER $USERNAME

ENV RUSTUP_HOME=/home/$USERNAME/.cache/.rustup \
    CARGO_HOME=/home/$USERNAME/.cache/.cargo \
    PATH=/home/$USERNAME/.cache/.cargo/bin:$PATH

# As the very last step, copy the startup script
USER root
COPY startup.sh /usr/local/bin/startup.sh
RUN chmod +x /usr/local/bin/startup.sh
USER $USERNAME

WORKDIR /app
ENTRYPOINT ["/usr/local/bin/startup.sh"]
CMD ["bash"]
