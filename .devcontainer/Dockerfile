FROM quay.io/fedora/fedora-minimal:42

# Pulling base dependencies
RUN echo install_weak_deps=False >> /etc/dnf/dnf.conf && \
    dnf install -y git sudo openssh-server nano wget tar bash-completion --skip-unavailable

# Enable rpmfusion
RUN dnf install -y https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm && \
    dnf install clang ffmpeg-devel sway wlr-protocols-devel bear lld -y && \
    dnf clean all && rm -rf /var/cache/dnf

# Manually install eza
RUN wget https://github.com/eza-community/eza/releases/download/v0.21.5/eza_x86_64-unknown-linux-gnu.tar.gz -O eza.tar.gz && \
    tar -xvf eza.tar.gz && \
    mv eza /usr/bin && \
    rm eza.tar.gz

# Create a user for ssh
RUN useradd -m dev && \
# Passwordless sudo privilege
    echo "dev ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/dev && \
    chmod 0440 /etc/sudoers.d/dev